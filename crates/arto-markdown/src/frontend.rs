//! Arto's frontend, for an application that places this crate's output in a
//! page of its own. Behind the `frontend` feature, since a host that only
//! wants the HTML has no use for the bundles.
//!
//! The HTML this crate writes leaves these to script: diagrams
//! (`preprocessed-mermaid`), formulas (`preprocessed-math*`), HTML embeds
//! (`preprocessed-html-embed`) and the colours of code. This module carries
//! the build of Arto's frontend that draws them in someone else's page,
//! together with the library bundles `arto page` and Quick Look embed, so a
//! host's documents look and behave the way they do in Arto. The bundles are
//! built from the same commit as the renderer, so the markup they read is the
//! markup it writes.
//!
//! A host places the rendered fragment as
//!
//! ```html
//! <div class="markdown-viewer"><article class="markdown-body">…</article></div>
//! ```
//!
//! loads [`STYLESHEET`] and [`RUNTIME`], then the scripts
//! [`Libraries::for_fragment`] names for that fragment, and calls
//! `ArtoRenderer.render(viewer)` on the `.markdown-viewer` element. The
//! script API is described in the repository's `docs/spec/embedding.md`.
//!
//! ```
//! use arto_markdown::frontend::{Libraries, RUNTIME, STYLESHEET};
//!
//! let fragment = r#"<pre class="preprocessed-mermaid" data-original-content="graph TD;">graph TD;</pre>"#;
//! let scripts: Vec<&str> = std::iter::once(RUNTIME)
//!     .chain(Libraries::for_fragment(fragment).scripts())
//!     .collect();
//!
//! assert!(!STYLESHEET.is_empty());
//! assert_eq!(scripts.len(), 2);
//! ```

pub use crate::libraries::*;

/// The stylesheet for a document in a host's page.
///
/// Every rule starts at `.markdown-viewer`: the design tokens are declared on
/// that element and the theme is named on it, so nothing outside it is
/// restyled, whatever the host names on its own root.
pub const STYLESHEET: &str = include_str!("../assets/frontend/embed.css");

/// The runtime for a document in a host's page, published as
/// `window.ArtoRenderer`. It does nothing until it is handed an element.
pub const RUNTIME: &str = include_str!("../assets/frontend/embed.iife.js");

/// Mermaid, for a fragment that draws a diagram. Load it after [`RUNTIME`].
///
/// It is several times the runtime by weight and it is a bundle of its own
/// for that reason: the IIFE format cannot code-split, so the only way to
/// leave a library out of a page is to have built it separately.
pub const MERMAID: &str = include_str!("../assets/frontend/page-mermaid.iife.js");

/// KaTeX, for a fragment that sets a formula. Load it after [`RUNTIME`].
///
/// It carries the rasterizer behind the block's copy-as-image button too,
/// which has nothing but a formula to draw.
pub const MATH: &str = include_str!("../assets/frontend/page-math.iife.js");

/// highlight.js with the languages it calls common, which is a seventh of the
/// library and what nearly every document writes its code blocks in. Load it
/// after [`RUNTIME`].
pub const HLJS_COMMON: &str = include_str!("../assets/frontend/page-hljs-common.iife.js");

/// highlight.js with every language, for the fragments that name one the
/// common build does not have. Load it after [`RUNTIME`].
pub const HLJS_ALL: &str = include_str!("../assets/frontend/page-hljs-all.iife.js");

/// The script an HTML embed's frame runs, exactly as the runtime writes it
/// into the frame.
///
/// The runtime draws a `preprocessed-html-embed` block in an `<iframe
/// srcdoc>`, and a `srcdoc` frame is held to the policy of the page around it
/// as well as to its own. A page whose `Content-Security-Policy` allowlists
/// scripts by hash therefore has to name this script's hash too, or the frame
/// cannot size itself or follow the theme.
pub const HTML_EMBED_FRAME: &str = include_str!("../assets/frontend/html-embed-frame.js");

/// The class of the block the runtime draws in a frame of its own.
const HTML_EMBED_CLASS: &str = "preprocessed-html-embed";

/// Whether `fragment`, HTML as `arto-markdown` renders it, draws an HTML
/// embed, and so runs [`HTML_EMBED_FRAME`].
pub fn draws_html_embed(fragment: impl AsRef<str>) -> bool {
    fragment.as_ref().contains(HTML_EMBED_CLASS)
}

impl Library {
    /// The bundle, compiled in.
    pub fn script(self) -> &'static str {
        match self {
            Self::Mermaid => MERMAID,
            Self::Math => MATH,
            Self::HljsCommon => HLJS_COMMON,
            Self::HljsAll => HLJS_ALL,
        }
    }
}

impl Libraries {
    /// The bundles to load after [`RUNTIME`], in the order they are to be
    /// loaded.
    pub fn scripts(self) -> impl Iterator<Item = &'static str> {
        self.iter().map(Library::script)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_the_scripts_are_the_bundles_of_the_libraries_in_their_order() {
        let everything = Libraries {
            mermaid: true,
            math: true,
            highlighting: Highlighting::Every,
        };

        assert_eq!(
            everything.scripts().collect::<Vec<_>>(),
            [MERMAID, MATH, HLJS_ALL]
        );
    }

    #[test]
    fn test_the_manifest_describes_the_bundles_compiled_in() {
        use base64::Engine as _;
        use sha2::Digest as _;

        let manifest: serde_json::Value =
            serde_json::from_str(include_str!("../assets/frontend/manifest.json"))
                .expect("the manifest is JSON");
        let integrity = |content: &str| {
            let digest = sha2::Sha256::digest(content.as_bytes());
            format!(
                "sha256-{}",
                base64::engine::general_purpose::STANDARD.encode(digest)
            )
        };
        let described = |entry: &serde_json::Value| entry["integrity"].as_str().map(str::to_owned);

        assert_eq!(manifest["version"], 1);
        assert_eq!(
            described(&manifest["stylesheet"]),
            Some(integrity(STYLESHEET))
        );
        assert_eq!(described(&manifest["runtime"]), Some(integrity(RUNTIME)));
        assert_eq!(
            described(&manifest["htmlEmbedFrame"]),
            Some(integrity(HTML_EMBED_FRAME))
        );
        for library in [
            Library::Mermaid,
            Library::Math,
            Library::HljsCommon,
            Library::HljsAll,
        ] {
            assert_eq!(
                described(&manifest["libraries"][library.name()]),
                Some(integrity(library.script())),
                "{library:?}"
            );
        }
        assert_eq!(
            manifest["libraries"].as_object().map(|names| names.len()),
            Some(4)
        );
    }

    #[test]
    fn test_the_stylesheet_names_nothing_outside_the_viewer() {
        // A host owns its root, and may name a `data-theme` or a token of its
        // own there; the build confines the sheet so neither is touched.
        assert!(!STYLESHEET.contains(":root"));
        assert!(STYLESHEET.contains(".markdown-viewer[data-theme=dark]"));
        assert!(STYLESHEET.contains(".markdown-viewer:not([data-theme])"));
    }

    #[test]
    fn test_the_runtime_is_published_where_the_libraries_look_for_it() {
        // Each library bundle registers itself on `window.ArtoRenderer`.
        assert!(RUNTIME.contains("ArtoRenderer"));
        assert!(MERMAID.contains("ArtoRenderer"));
    }
}
