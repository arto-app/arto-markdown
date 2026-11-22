//! Which of the frontend's library bundles a rendered document needs.
//!
//! The HTML leaves diagrams, formulas and the colours of code to script, and
//! each library that draws them is paid for by every page that carries it. A
//! host loads only the ones a document has something for, and needs to know
//! which whether it compiles the bundles in (the `frontend` feature) or
//! loads them as files named by the build's `manifest.json`.

use std::collections::HashSet;
use std::sync::LazyLock;

/// The names each highlight.js build answers to, aliases included, one per
/// line, written by the frontend build (`just languages`).
///
/// Reading them rather than naming languages here is what keeps the choice
/// honest across a highlight.js release: a set hard-coded in this crate would
/// go quietly wrong the first time upstream moved a language in or out of
/// `common`, and the page would lose its colours with nothing failing. They
/// are kept in the sources rather than among the bundles so that the choice
/// is there without the `frontend` feature; `just ci` fails when they differ
/// from what the build writes.
const HLJS_COMMON_LANGUAGES: &str = include_str!("libraries/hljs-common.txt");
const HLJS_ALL_LANGUAGES: &str = include_str!("libraries/hljs-all.txt");

static HLJS_COMMON_NAMES: LazyLock<HashSet<&str>> =
    LazyLock::new(|| HLJS_COMMON_LANGUAGES.lines().collect());
static HLJS_ALL_NAMES: LazyLock<HashSet<&str>> =
    LazyLock::new(|| HLJS_ALL_LANGUAGES.lines().collect());

/// One of the frontend's library bundles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Library {
    /// Pele, which draws Mermaid diagrams.
    Mermaid,
    /// KaTeX, which sets formulas.
    Math,
    /// highlight.js with the languages it calls common.
    HljsCommon,
    /// highlight.js with every language.
    HljsAll,
}

impl Library {
    /// The name the build's `manifest.json` lists the bundle under.
    pub fn name(self) -> &'static str {
        match self {
            Self::Mermaid => "mermaid",
            Self::Math => "math",
            Self::HljsCommon => "hljs-common",
            Self::HljsAll => "hljs-all",
        }
    }
}

/// The libraries a rendered fragment calls for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Libraries {
    /// The fragment draws a diagram.
    pub mermaid: bool,
    /// The fragment sets a formula.
    pub math: bool,
    /// How much of highlight.js the fragment's code blocks need.
    pub highlighting: Highlighting,
}

/// How much of highlight.js a fragment has to be given.
///
/// Ordered, because a document is read block by block and what it needs is
/// the most any one block asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Highlighting {
    /// Nothing to colour, or nothing that highlight.js could colour anyway.
    #[default]
    None,
    /// Every language named is one the common build has — or a block names
    /// none, and the language has to be detected, which is what that set is
    /// upstream's answer for.
    Common,
    /// Something only the whole library can colour.
    Every,
}

impl Libraries {
    /// The libraries `fragment`, HTML as `arto-markdown` renders it, calls
    /// for.
    pub fn for_fragment(fragment: impl AsRef<str>) -> Self {
        let fragment = fragment.as_ref();
        Self {
            mermaid: draws_diagram(fragment),
            math: sets_math(fragment),
            highlighting: highlighting_for(fragment),
        }
    }

    /// The libraries, in the order they are to be loaded after the runtime.
    pub fn iter(self) -> impl Iterator<Item = Library> {
        let highlighter = match self.highlighting {
            Highlighting::None => None,
            Highlighting::Common => Some(Library::HljsCommon),
            Highlighting::Every => Some(Library::HljsAll),
        };
        [
            self.mermaid.then_some(Library::Mermaid),
            self.math.then_some(Library::Math),
            highlighter,
        ]
        .into_iter()
        .flatten()
    }
}

/// Whether the document sets a formula to typeset.
///
/// Math is typeset in the page rather than in the HTML handed to it, so a body
/// with no `preprocessed-math*` container never reaches for KaTeX — nor for a
/// KaTeX glyph, which is what lets the stylesheet's faces go as well.
fn sets_math(body_html: &str) -> bool {
    body_html.contains("preprocessed-math")
}

/// Whether the document holds a diagram to draw.
///
/// `arto-markdown` writes the container; the fenced form the renderer also
/// accepts is looked for among the code blocks rather than anywhere in the
/// body, because a document that merely quotes `language-mermaid` in a code
/// sample would otherwise carry a bundle several times the runtime for nothing.
/// The name is matched as written: the renderer selects it with
/// `code.language-mermaid`, and a class selector is case-sensitive.
fn draws_diagram(body_html: &str) -> bool {
    body_html.contains("preprocessed-mermaid")
        || code_block_languages(body_html).any(|language| language == Some("mermaid"))
}

/// Which build of highlight.js, if any, the body calls for.
fn highlighting_for(body_html: &str) -> Highlighting {
    code_block_languages(body_html)
        .map(|language| match language {
            None => Highlighting::Common,
            // `hljs.getLanguage` lowercases before it looks a name up, so a
            // fence written `JSON` colours like `json` and has to be counted
            // as `json` here — matched as written, the page would leave the
            // library out and show the block the app colours as plain text.
            Some(name) => match name.to_ascii_lowercase().as_str() {
                // Another renderer owns these two, which is why the frontend
                // takes them out of highlight.js rather than colouring them.
                "mermaid" | "math" => Highlighting::None,
                name if HLJS_COMMON_NAMES.contains(name) => Highlighting::Common,
                name if HLJS_ALL_NAMES.contains(name) => Highlighting::Every,
                // A name highlight.js has never heard of stays plain text
                // whichever build is embedded, so it asks for neither.
                _ => Highlighting::None,
            },
        })
        .max()
        .unwrap_or(Highlighting::None)
}

/// The language of every block the page would colour, `None` where the fence
/// named none and the language has to be detected.
///
/// A block is a `<code>` opening immediately inside a `<pre>`, which is what
/// the frontend highlights (`pre code`) and what the pipeline writes for a
/// fenced block. An inline code span is a `<code>` anywhere else, and a
/// diagram's or a formula's `<pre>` holds no `<code>` at all.
fn code_block_languages(body_html: &str) -> impl Iterator<Item = Option<&str>> {
    body_html.match_indices("<pre").filter_map(|(at, _)| {
        let tag_end = body_html[at..].find('>')? + 1;
        let attributes = body_html[at + tag_end..]
            .trim_start()
            .strip_prefix("<code")?;
        let end = attributes.find('>')?;
        Some(language_class(&attributes[..end]))
    })
}

/// The language a `<code>`'s attributes name, if any.
///
/// The marker is looked for as a class of its own anywhere in the list, which
/// is how the frontend reads it off `className` — a body carrying raw HTML may
/// have written the classes in any order, and taking only the first one would
/// call a labelled block unlabelled.
fn language_class(attributes: &str) -> Option<&str> {
    const CLASS: &str = "class=\"";
    const MARKER: &str = "language-";

    let at = attributes.find(CLASS)? + CLASS.len();
    let list = &attributes[at..];
    let list = &list[..list.find('"')?];
    list.split_ascii_whitespace()
        .find_map(|class| class.strip_prefix(MARKER))
        // `language-` on its own names nothing; the frontend detects instead.
        .filter(|name| !name.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn libraries_for(fragment: &str) -> Vec<Library> {
        Libraries::for_fragment(fragment).iter().collect()
    }

    #[test]
    fn test_a_prose_fragment_calls_for_no_library() {
        assert_eq!(Libraries::for_fragment("<p>hi</p>"), Libraries::default());
        assert_eq!(libraries_for("<p>hi</p>"), []);
    }

    #[test]
    fn test_a_fragment_that_draws_a_diagram_calls_for_mermaid() {
        let diagram = r#"<pre class="preprocessed-mermaid" data-original-content="graph TD;">graph TD;</pre>"#;

        assert_eq!(libraries_for(diagram), [Library::Mermaid]);
    }

    #[test]
    fn test_a_fragment_that_sets_a_formula_calls_for_katex() {
        let math = r#"<span class="preprocessed-math-inline" data-original-content="x">x</span>"#;

        assert_eq!(libraries_for(math), [Library::Math]);
    }

    #[test]
    fn test_the_libraries_come_in_one_order_whatever_the_fragment_holds() {
        let everything = Libraries {
            mermaid: true,
            math: true,
            highlighting: Highlighting::Every,
        };

        assert_eq!(
            everything.iter().collect::<Vec<_>>(),
            [Library::Mermaid, Library::Math, Library::HljsAll]
        );
    }

    #[test]
    fn test_a_document_with_no_code_block_carries_no_highlighter() {
        assert_eq!(highlighting_for("<p>hi</p>"), Highlighting::None);
        // An inline code span is not a block, and is not coloured.
        assert_eq!(
            highlighting_for("<p>the <code>--flag</code> flag</p>"),
            Highlighting::None
        );
    }

    #[test]
    fn test_a_usual_language_takes_the_common_build() {
        let rust = r#"<pre data-source-range="1:1-3:3"><code data-source-range="2:1-2:12" class="language-rust">fn main() {}</code></pre>"#;

        assert_eq!(libraries_for(rust), [Library::HljsCommon]);
    }

    #[test]
    fn test_an_unlabelled_block_takes_the_common_build_to_detect_with() {
        let plain = r#"<pre data-source-range="1:1-3:3"><code data-source-range="2:1-2:15">anything at all</code></pre>"#;

        assert_eq!(highlighting_for(plain), Highlighting::Common);
    }

    #[test]
    fn test_a_language_outside_the_common_set_takes_the_whole_library() {
        let unusual = r#"<pre><code class="language-brainfuck">+++.</code></pre>"#;
        // The premise: this is a language highlight.js has and `common` does
        // not, which is what makes the whole library the only build that can
        // colour it.
        assert!(!HLJS_COMMON_NAMES.contains("brainfuck"));
        assert!(HLJS_ALL_NAMES.contains("brainfuck"));

        assert_eq!(highlighting_for(unusual), Highlighting::Every);
    }

    #[test]
    fn test_the_most_any_block_asks_for_is_what_the_page_carries() {
        let both = r#"<pre><code class="language-rust">fn main() {}</code></pre><pre><code class="language-brainfuck">+++.</code></pre>"#;

        assert_eq!(highlighting_for(both), Highlighting::Every);
    }

    #[test]
    fn test_an_alias_is_the_language_it_stands_for() {
        // `sh` is `bash`, and the list carries the aliases for that reason: a
        // fence rarely says the language's own name.
        let shell = r#"<pre><code class="language-sh">ls</code></pre>"#;

        assert_eq!(highlighting_for(shell), Highlighting::Common);
    }

    #[test]
    fn test_a_language_named_in_capitals_is_the_language_it_names() {
        // `hljs.getLanguage` lowercases, so the app colours this block; a page
        // that read the name as written would embed no library at all.
        let shouty = r#"<pre><code class="language-JSON">{"a":1}</code></pre>"#;

        assert_eq!(libraries_for(shouty), [Library::HljsCommon]);
    }

    #[test]
    fn test_the_marker_is_found_wherever_it_sits_in_the_class_list() {
        // Raw HTML in the body may write the classes in any order.
        let second = r#"<pre><code class="hljs language-brainfuck">+++.</code></pre>"#;

        assert_eq!(highlighting_for(second), Highlighting::Every);
    }

    #[test]
    fn test_a_body_that_only_quotes_the_diagram_class_carries_no_mermaid() {
        // A document showing the frontend's own contract in a code sample
        // holds the text, not a diagram, and Pele is a heavy bundle to embed
        // by mistake.
        let quoting = r#"<pre><code class="language-html">&lt;code class="language-mermaid"&gt;</code></pre>"#;

        assert!(!draws_diagram(quoting));
    }

    #[test]
    fn test_a_name_highlight_js_does_not_know_asks_for_nothing() {
        let unknown = r#"<pre><code class="language-notalanguage">x</code></pre>"#;

        assert_eq!(libraries_for(unknown), []);
    }

    #[test]
    fn test_a_diagram_block_is_drawn_rather_than_coloured() {
        // The fenced form the renderer also accepts: Mermaid draws it, and
        // highlight.js is told not to colour it, so the page carries one
        // library rather than two.
        let fenced = r#"<pre><code class="language-mermaid">graph TD;</code></pre>"#;

        assert_eq!(libraries_for(fenced), [Library::Mermaid]);
    }

    #[test]
    fn test_every_library_has_a_name_of_its_own() {
        let all = [
            Library::Mermaid,
            Library::Math,
            Library::HljsCommon,
            Library::HljsAll,
        ];
        let names: HashSet<_> = all.iter().map(|library| library.name()).collect();

        assert_eq!(names.len(), all.len());
    }
}
