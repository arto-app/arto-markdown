# arto-markdown

Markdown to HTML, and the frontend that draws it, for any host.

`arto-markdown` is the rendering engine of [Arto](https://github.com/arto-app/Arto),
the Markdown reader, released on its own. It turns GitHub-flavoured Markdown —
with frontmatter, alerts, footnotes, math, Mermaid diagrams and wiki links —
into HTML whose attributes and class names are a documented contract, and it
carries the stylesheet and scripts that draw that HTML in any web page.

```rust
use arto_markdown::{render_to_html, RenderOptions};

let markdown = "# Hello\n\n> [!NOTE]\n> Rendered by arto-markdown.\n";
// Relative links and images resolve against the directory of this path.
let rendered = render_to_html(markdown, "docs/hello.md", &RenderOptions::default())
    .expect("the document renders");
assert!(rendered.html.contains("markdown-alert-note"));
```

## Install

```toml
[dependencies]
# The renderer alone.
arto-markdown = "0.48"
# With the stylesheet, the runtime and the library bundles that draw the
# HTML in a page of the host's own.
arto-markdown = { version = "0.48", features = ["frontend"] }
```

The frontend's sources are the npm package `@arto-app/markdown`, for a host
that bundles them itself.

`arto-frontend`, the crate the bundles were published in before, stays at
0.47.1 and is not released any more; depend on `arto-markdown` with the
`frontend` feature instead.

## Usage

The crate documentation describes the pipeline and the HTML contract;
[docs/spec/embedding.md](docs/spec/embedding.md) describes how a host places a
rendered document in its page and what the runtime promises it. A host that
keeps documents somewhere other than this machine's disk answers for the files
a document names through `arto_markdown::Renderer` and its `Files` trait.

## Development

Enter the development shell with `nix develop` (or copy `.envrc.sample` to
`.envrc` and `direnv allow`), then:

```sh
just fmt   # format every source
just ci    # everything CI runs: format check, lint, tests, packaging
```

The sample documents under [samples/](samples/) are what the renderer's
output is snapshot-tested against. The design lives in
[docs/concept.md](docs/concept.md), the decisions behind it in
[docs/adr/](docs/adr/), and the behaviour it promises in
[docs/spec/](docs/spec/).

### Releasing

Publish a GitHub release tagged `vX.Y.Z` with notes written by hand. The
release workflow stamps that version over the `0.0.0` placeholders and
publishes `arto-markdown` to crates.io and `@arto-app/markdown` to npm through
trusted publishing. Below 1.0, a breaking change raises the minor version.

## License

MIT. See [LICENSE](LICENSE).
