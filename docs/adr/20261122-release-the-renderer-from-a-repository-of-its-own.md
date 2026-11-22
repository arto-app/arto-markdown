# Release the renderer from a repository of its own

Status: Accepted

## Context

The renderer and its frontend lived in Arto's repository and were released
with every Arto release, at Arto's version. Hosts other than Arto build on
them, and more are coming: a browser through WebAssembly, phones through
native bindings. Each of those would have waited for an app release to get a
renderer fix, and taken a new renderer version whenever the app changed
alone. A version number that moved with the app said nothing about whether
the renderer's API had changed.

## Decision

The renderer, its frontend and the samples that pin its output are developed
and released from `arto-app/arto-markdown`, with a version of their own. The
version continues each name's line on its registry, so the first release here
is newer than any released from Arto, and it follows semantic versioning on
the renderer's API: below 1.0, a breaking change raises the minor version.

## Alternatives considered

- Stay in Arto's repository and release the renderer on its own tags. The
  renderer's history, CI and releases would still be tangled with the app's,
  and a host would have to read the app's changes to find the renderer's.
- Start the new version line at 1.0. The API is still changing for the
  WebAssembly and native hosts, so 1.0 would promise a stability that the
  next releases would have to break.

## Consequences

- Arto depends on a released renderer like any other host, and a change that
  needs both lands here first.
- The renderer's releases no longer say which Arto they came with; Arto's
  manifest does.
- `arto-frontend`, which only re-exports the renderer's frontend under its
  old name, is not released from here and stays at the last version Arto
  published.
