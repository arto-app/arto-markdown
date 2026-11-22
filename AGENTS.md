# arto-markdown

<!-- Quoted from README.md; do not rephrase it here. -->
Markdown to HTML, and the frontend that draws it, for any host.

## How it is built

- **Test first.** Red → green → refactor. Write the failing test before the
  code that makes it pass.
- **Each layer holds one thing.** Put a fact in the one layer that owns it,
  and nowhere else.

  | Layer | Holds |
  | --- | --- |
  | `docs/concept.md` | what this is and for whom |
  | `docs/adr/` | one decision and why it was made |
  | `docs/spec/` | the behaviour users and other components can rely on |
  | Tests | the details that pin the behaviour down: rules, boundaries, tables |
  | Code | how it is done |
  | Commits | why a change was made |
  | Comments | why it is not done another way |

  Before writing, fixing or reviewing a record, read `docs/adr/README.md`;
  before writing, fixing or reviewing a spec, read `docs/spec/README.md`.
  Follow their writing order.
- **Dependencies are current.** Before adding or bumping one, look up its
  latest release in the registry. Do not copy a version from memory or from
  another repository.

## Project rules

- **Test code**: `indoc` for multi-line strings, `tempfile` for files
- **Module system**: Rust 2018+ (no `mod.rs`)
- **The HTML is a contract**: read `.claude/rules/markdown-pipeline.md`
  (loaded when a file of the renderer, the frontend or the samples is read)
  before changing what the renderer writes or the frontend reads.

## Quality gate

Run `just ci` before reporting a change as done. It checks formatting
(rustfmt, oxfmt, nixfmt), lints (clippy, oxlint, tsc), builds the renderer for
wasm32, runs every test (sample snapshots included), and checks the rustdoc,
feature combinations, advisories and licenses, unused dependencies, the crate
package, the release stamp, the documentation's links and the workflow files.
CI runs the same recipe.

`just fmt` formats everything `just ci` checks.

## Layout

| Path | What lives there |
| --- | --- |
| `crates/arto-markdown/` | The renderer: Markdown → HTML, and (feature `frontend`) the frontend's bundles as Rust strings |
| `crates/arto-markdown-wasm/` | The renderer for the browser: wasm-bindgen bindings, built into `packages/markdown/wasm/` by `just wasm-package` and published in the npm package, not as a crate |
| `packages/markdown/` | `@arto-app/markdown`: the frontend's TypeScript and CSS, built by Vite into the crate's `assets/frontend/`; with the WebAssembly renderer when published |
| `samples/` | Sample documents whose HTML is snapshot-tested |
| `docs/` | concept, ADRs, and specs |
| `scripts/` | Release stamp and documentation checks the recipes run |

The pnpm workspace's root is the repository root; `packages/` holds its
packages.

## Language

- README, docs and commit messages: English
- Code comments, log and error messages: English

## Commits

Scoped Commits (`.gitmessage` is the template):

```
<scope>: <description>

<why this change is needed>
```

- The scope names where the change is, from the table below. Several scopes
  are joined with `, `; a change across the whole tree uses `treewide`.
- The description starts in lower case, has no trailing period, and stays
  within 72 characters.
- The body explains why. The code shows how; the tests show what.
- A release commit is `release: vX.Y.Z`.
- A breaking change says so in the body and carries a `BREAKING CHANGE:`
  trailer.

The history carried over from Arto keeps its own references:
`arto-app/Arto#N` names a pull request of that repository.

| Scope | Covers |
| --- | --- |
| `markdown` | `crates/arto-markdown/` |
| `wasm` | `crates/arto-markdown-wasm/` |
| `frontend` | `packages/markdown/`, `crates/arto-markdown/src/frontend.rs` |
| `samples` | `samples/` |
| `docs` | `README.md`, `LICENSE`, `docs/concept.md` |
| `adr` | `docs/adr/` |
| `spec` | `docs/spec/` |
| `agents` | `AGENTS.md`, `.claude/` |
| `build` | `justfile`, the workspace manifests and lockfiles, `rust-toolchain.toml`, `scripts/`, lint and deny settings |
| `ci` | `.github/` |
| `nix` | `flake.nix`, `flake.lock` |
| `deps` | dependency updates |

Add a row when a new area appears.
