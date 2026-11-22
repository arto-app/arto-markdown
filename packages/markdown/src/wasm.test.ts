import fs from "fs";
import path from "path";
import { beforeAll, describe, expect, it } from "vitest";

import {
  type Files,
  initSync,
  render,
  renderDetached,
  renderPreview,
  renderWithToc,
} from "../wasm/arto_markdown_wasm.js";

/** A folder held in memory, addressed by paths that exist nowhere on disk. */
function memoryFiles(files: Record<string, Uint8Array>): Files {
  const normalize = (file: string) => path.posix.normalize(file);
  return {
    stat: (file) => {
      const bytes = files[normalize(file)];
      return bytes ? { path: normalize(file), len: bytes.length, modified: 1000 } : null;
    },
    read: (file, limit) => {
      const bytes = files[normalize(file)];
      return bytes && bytes.length <= limit ? bytes : null;
    },
  };
}

const png = new Uint8Array([0x89, 0x50, 0x4e, 0x47]);

describe("the WebAssembly renderer", () => {
  beforeAll(() => {
    initSync({
      module: fs.readFileSync(
        path.resolve(import.meta.dirname, "../wasm/arto_markdown_wasm_bg.wasm"),
      ),
    });
  });

  it("renders Markdown to the HTML the crate writes", () => {
    const rendered = render("# Hello\n\n> [!NOTE]\n> Hi.\n", "/docs/hello.md");

    expect(rendered.html).toContain("markdown-alert-note");
    expect(rendered.headings).toEqual([]);
  });

  it("names the libraries the HTML needs, in the order to load them", () => {
    const markdown = "$x$\n\n```mermaid\ngraph TD;\n```\n\n```rust\nfn main() {}\n```\n";

    expect(render(markdown, "/docs/a.md").libraries).toEqual(["mermaid", "math", "hljs-common"]);
    expect(render("Prose.\n", "/docs/a.md").libraries).toEqual([]);
  });

  it("reports the headings when a table of contents is asked for", () => {
    const rendered = renderWithToc("# One\n\n## Two\n", "/docs/a.md");

    expect(rendered.headings).toEqual([
      { level: 1, text: "One", id: "one" },
      { level: 2, text: "Two", id: "two" },
    ]);
  });

  it("takes the options by the names config.json gives them", () => {
    expect(render("$x$\n", "/docs/a.md", { math: false }).libraries).toEqual([]);
    expect(render("<b>bold</b>\n", "/docs/a.md", { rawHtml: "escape" }).html).toContain(
      "&lt;b&gt;",
    );
  });

  it("asks the host's files whether a linked document exists", () => {
    const files = memoryFiles({ "/notes/other.md": new TextEncoder().encode("# Other") });

    const html = render("[there](./other.md) [gone](./gone.md)\n", "/notes/doc.md", {}, files).html;

    expect(html).toContain('data-md-link="./other.md" class="md-link"');
    expect(html).toContain('data-md-link="./gone.md" class="md-link md-link-missing"');
  });

  it("inlines an image the host's files hold", () => {
    const files = memoryFiles({ "/notes/pic.png": png });

    expect(render("![pic](./pic.png)\n", "/notes/doc.md", {}, files).html).toContain(
      'src="data:image/png;base64,iVBORw=="',
    );
  });

  it("leaves an image to the host when told where it serves them", () => {
    const files = memoryFiles({ "/notes/pic.png": png });

    const rendered = render(
      "![pic](./pic.png)\n",
      "/notes/doc.md",
      { deferredImageBaseUrl: "https://host.test/images" },
      files,
    );

    expect(rendered.images).toEqual([
      { id: expect.any(String), path: "/notes/pic.png", mime: "image/png" },
    ]);
    expect(rendered.html).toContain(`src="https://host.test/images/${rendered.images[0].id}`);
  });

  it("counts what the document asks a reader to read", () => {
    const rendered = render("Two words.\n\n```\ncode\n```\n", "/docs/a.md");

    expect(rendered.reading).toEqual([
      { line: 1, cjkChars: 0, words: 2, codeLines: 0, figures: 0 },
      { line: 3, cjkChars: 0, words: 0, codeLines: 1, figures: 0 },
    ]);
  });

  it("drops what a detached render or a preview would fetch from elsewhere", () => {
    const remote = "![remote](https://elsewhere.test/a.png)\n";

    expect(renderDetached(remote, "/docs/a.md").html).not.toContain("elsewhere.test");
    expect(renderPreview(remote, "/docs/a.md").html).not.toContain("elsewhere.test");
  });

  it("throws when the parser gives up on the document", () => {
    expect(() => render(`${">".repeat(200)} too deep`, "/docs/a.md")).toThrow(
      "failed to parse Markdown",
    );
  });
});
