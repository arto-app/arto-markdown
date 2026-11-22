# Run `nix develop` (or use direnv) first so every tool is on PATH.

set shell := ["bash", "-euo", "pipefail", "-c"]

# List the recipes.
default:
    @just --list

# Install the frontend's dependencies (pnpm keeps the lockfile frozen in CI).
setup:
    pnpm install

# Build the frontend's bundles into crates/arto-markdown/assets/frontend,
# which the crate's `frontend` feature compiles in and its package ships.
assets: setup
    pnpm --filter @arto-app/markdown run build

# Build the renderer for the browser into packages/markdown/wasm, which the
# npm package publishes as its `./wasm` export.
wasm-package:
    cargo build -p arto-markdown-wasm --target wasm32-unknown-unknown --release
    wasm-bindgen --target web --out-dir packages/markdown/wasm target/wasm32-unknown-unknown/release/arto_markdown_wasm.wasm

# Copy the languages each highlight.js bundle answers to into the crate's
# sources, where the renderer reads them to tell a host which bundle a
# document needs. Run it after a highlight.js update.
languages: assets
    cp packages/markdown/dist/page-hljs-common.txt crates/arto-markdown/src/libraries/hljs-common.txt
    cp packages/markdown/dist/page-hljs-all.txt crates/arto-markdown/src/libraries/hljs-all.txt

# The crate's copy of the language lists must be what the bundles answer to.
languages-check: assets
    diff -u crates/arto-markdown/src/libraries/hljs-common.txt packages/markdown/dist/page-hljs-common.txt
    diff -u crates/arto-markdown/src/libraries/hljs-all.txt packages/markdown/dist/page-hljs-all.txt

# Format every source.
fmt: setup
    cargo fmt --all
    pnpm --recursive run fmt
    just _nixfmt

# Check formatting without changing files.
fmt-check: setup
    cargo fmt --all --check
    pnpm --recursive run fmt:check
    just _nixfmt --check

# nixfmt comes from the devShell, which Windows does not have; there
# flake.nix is left to the legs that have it.
[private]
_nixfmt *args:
    if command -v nixfmt >/dev/null; then nixfmt {{ args }} flake.nix; else echo "nixfmt not found; skipping flake.nix" >&2; fi

# Lint with every warning as an error: clippy, then oxlint and tsc.
check: assets wasm-package
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    pnpm --recursive run check

# The renderer is what a browser host runs, so it has to build for wasm32
# with nothing in it that only the desktop platforms provide.
wasm:
    cargo clippy -p arto-markdown -p arto-markdown-wasm --target wasm32-unknown-unknown -- -D warnings

# Run every test: the crate's (sample snapshots included) and the frontend's.
test: assets wasm-package
    cargo test --workspace --all-targets --all-features
    cargo test --workspace --doc --all-features
    pnpm --recursive run test

# Documentation must build without warnings: a broken intra-doc link or a
# public item linking to a private one is an error here.
doc: assets
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features

# Every feature must build on its own; `check` only covers `--all-features`.
features: assets
    cargo hack check --each-feature --no-dev-deps -p arto-markdown

# Advisories, licenses and sources, per deny.toml.
deny:
    cargo deny --workspace check

# Declared dependencies nothing imports.
machete:
    cargo machete

# The crates packaged and built from their packages alone, as `cargo publish`
# verifies them on release (the bindings against the renderer's package, not
# its sources): a file an `include_str!` reads but the `include`
# list leaves out fails here, not after a release is out. `--allow-dirty`
# because under jj the working copy is always ahead of git's HEAD.
package: assets
    cargo package -p arto-markdown -p arto-markdown-uniffi --allow-dirty
    just npm-package

# The npm package as `npm publish` would pack it: every export and the
# generated theme stylesheet in it, no test in it.
npm-package: assets wasm-package
    perl scripts/check-npm-package.pl packages/markdown

# Stamp a copy of the manifests the way a release does, so a placeholder the
# release workflow would no longer find fails here, on every OS.
stamp-check:
    tmp=$(mktemp -d) && mkdir -p "$tmp/packages/markdown" && cp Cargo.toml Cargo.lock "$tmp" && cp packages/markdown/package.json "$tmp/packages/markdown" && perl scripts/stamp-version.pl 0.0.0-ci "$tmp"

# Links in Markdown files (offline, see lychee.toml) and repository paths
# named in documentation.
docs:
    lychee --config lychee.toml './**/*.md'
    perl scripts/check-doc-paths.pl

# Workflow files: syntax and expression errors, then security smells.
workflows:
    actionlint
    zizmor --min-severity low --config .github/zizmor.yml .

# Everything CI runs.
ci: fmt-check check wasm test languages-check doc features deny machete package stamp-check docs workflows
