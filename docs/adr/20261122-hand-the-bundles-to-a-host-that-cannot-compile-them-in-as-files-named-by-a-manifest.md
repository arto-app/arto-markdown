# Hand the bundles to a host that cannot compile them in as files named by a manifest

Status: Accepted

## Context

The frontend's bundles reached a host only as Rust strings behind the
`frontend` feature, and only that feature could say which of them a document
needed. The hosts the renderer is going to — a browser running it as
WebAssembly, an iOS or an Android app calling it through bindings — cannot
take megabytes of script through a function call: they load the bundles as
files from their own package. They still need the renderer's answer about
which ones a document needs, and that answer was compiled in only with the
bundles.

## Decision

The build writes a manifest that names each bundle's file and hash, and every
render returns, without the `frontend` feature, the libraries its HTML needs
under the names the manifest uses. A host that cannot compile the bundles in
loads the files the manifest names for those libraries.

## Alternatives considered

- Hand every host the bundles as strings through its bindings. Each render
  would carry or copy several megabytes, and a browser could not cache them
  as files.
- Let each host decide the libraries from the HTML on its own. Every binding
  would carry a copy of the rules, and the copies would drift from the
  renderer's markup and from highlight.js's languages.

## Consequences

- The manifest's form and the library names are an interface, and changing
  either is a breaking change.
- The renderer carries the lists of languages each highlight.js build has,
  and they have to be refreshed when highlight.js is.
- A host that compiles the bundles in keeps doing so, and the two forms come
  from the same build.
