# Bind the renderer with UniFFI here, and let each app build it in

Status: Accepted

## Context

Arto's iOS and Android apps call the renderer from Swift and Kotlin, through
bindings UniFFI generates. The interface those bindings expose follows the
renderer's API and changes with it. The apps are the only hosts that call
it from those languages, and each builds its native code as part of its own
build.

## Decision

The UniFFI interface is a crate of this repository, published to crates.io
in the same release as the renderer. Each app generates its bindings from
that crate and builds it into itself; no Swift package or Android library is
published.

## Alternatives considered

- Publish a Swift package and an Android library from repositories of their
  own. Every release would have to commit a binary checksum to a tag and go
  through Maven Central, for packages nobody but the apps uses.
- Write the UniFFI interface in each app. The two copies would drift from
  the renderer and from each other.

## Consequences

- A change of the renderer's API changes the interface in the same pull
  request, under the same tests.
- An app takes the renderer by depending on the published crate at a
  version, and testing the bindings from Swift and Kotlin is the app's.
- The interface's version is the renderer's, and a breaking change to it is
  a breaking change of the release.
