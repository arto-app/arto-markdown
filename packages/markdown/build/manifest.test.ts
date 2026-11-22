import { createHash } from "crypto";
import fs from "fs";
import os from "os";
import path from "path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { bundleManifest, type BundleLayout } from "./manifest.ts";

const layout: BundleLayout = {
  stylesheet: "embed.css",
  runtime: "embed.iife.js",
  htmlEmbedFrame: "html-embed-frame.js",
  libraries: { mermaid: "page-mermaid.iife.js", math: "page-math.iife.js" },
};

function integrity(content: string): string {
  return `sha256-${createHash("sha256").update(content).digest("base64")}`;
}

describe("bundleManifest", () => {
  let dir: string;

  beforeEach(() => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), "bundle-manifest-"));
    for (const file of [
      "embed.css",
      "embed.iife.js",
      "html-embed-frame.js",
      "page-mermaid.iife.js",
      "page-math.iife.js",
    ]) {
      fs.writeFileSync(path.join(dir, file), `content of ${file}`);
    }
  });

  afterEach(() => {
    fs.rmSync(dir, { recursive: true, force: true });
  });

  it("names every file with the hash of what it holds", () => {
    const manifest = bundleManifest(dir, layout);

    expect(manifest).toEqual({
      version: 1,
      stylesheet: { file: "embed.css", integrity: integrity("content of embed.css") },
      runtime: { file: "embed.iife.js", integrity: integrity("content of embed.iife.js") },
      htmlEmbedFrame: {
        file: "html-embed-frame.js",
        integrity: integrity("content of html-embed-frame.js"),
      },
      libraries: {
        mermaid: {
          file: "page-mermaid.iife.js",
          integrity: integrity("content of page-mermaid.iife.js"),
        },
        math: { file: "page-math.iife.js", integrity: integrity("content of page-math.iife.js") },
      },
    });
  });

  it("fails on a file the build did not write", () => {
    fs.rmSync(path.join(dir, "page-math.iife.js"));

    expect(() => bundleManifest(dir, layout)).toThrow("page-math.iife.js");
  });
});
