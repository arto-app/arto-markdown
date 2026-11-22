# Publish the frontend's sources to npm

Status: Accepted

## Context

The frontend's TypeScript and CSS were a private package in Arto's pnpm
workspace, imported by Arto's app under the package's name. Out of that
workspace there was no way to install them: a host that bundles its own page,
Arto's app included once the renderer left its repository, had only the
built bundles the crate carries, which cannot be imported module by module or
styled from the host's own build.

## Decision

`@arto-app/markdown` is published to npm, from the same release and at the
same version as the `arto-markdown` crate, as the frontend's sources: the
modules its `exports` name and the stylesheets, generated ones included.

## Alternatives considered

- Publish only the built bundles. A host could load them but not import a
  module from them or bundle them with its own code, which is what Arto's app
  does.
- Keep the package private and let a host vendor the sources. Every host
  would hold a copy that drifts from the renderer it runs with.

## Consequences

- What `exports` names is the package's API, and removing an export is a
  breaking change of the release.
- A consumer bundles TypeScript sources, so it needs a bundler that compiles
  them, as Vite does.
- The crate's bundles and the package's sources come from one commit, so the
  markup the frontend reads is the markup the renderer writes in either form.
