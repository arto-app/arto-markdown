# Ship the browser renderer inside the npm package of the frontend

Status: Accepted

## Context

A web page that shows Markdown the way Arto does needs the renderer, built
as WebAssembly, and the frontend that draws its HTML. The frontend was
already published to npm as `@arto-app/markdown`, from the same release as
the crate. The renderer and the frontend are one interface: the frontend reads
the markup the renderer writes, and the renderer names the frontend's bundles
a document needs.

## Decision

The WebAssembly renderer and the built bundles are published inside
`@arto-app/markdown`, beside the frontend's sources, rather than as a package
of their own.

## Alternatives considered

- Publish the renderer as a package of its own. A page could install a
  renderer and a frontend from different releases, whose markup and bundle
  names need not agree.
- Publish it as a crate for each page to build. A page would need a Rust
  toolchain and the wasm-bindgen CLI of the matching version to show a
  document.

## Consequences

- One install gives a page the renderer, the bundles it names and the
  sources, all of one version.
- A page that only bundles the frontend's sources downloads the WebAssembly
  as well.
- Publishing to npm needs a Rust toolchain and the wasm-bindgen CLI the
  bindings pin.
