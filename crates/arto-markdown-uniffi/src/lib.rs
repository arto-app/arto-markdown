//! `arto-markdown` as a [UniFFI](https://mozilla.github.io/uniffi-rs/)
//! interface, for the Swift and Kotlin bindings an app generates from it and
//! builds in.
//!
//! Each render function is the crate's of the same name. The options are the
//! ones `config.json` names, each left out keeping the renderer's default; the
//! files a document names are asked of the host's [`Files`]; and the
//! libraries the HTML needs come back by the names the build's
//! `manifest.json` lists the bundles under. The bundles themselves are files
//! the `arto-markdown` crate's package carries.
//!
//! The bindings are generated from a built library (`uniffi-bindgen generate
//! --library`), which reads the interface from metadata the library carries.
//! A build with `strip = true` drops that metadata, so generate them from an
//! unstripped build and strip the one the app ships.

use arto_markdown::{ImageResolution, Renderer};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

uniffi::setup_scaffolding!();

/// What becomes of raw HTML written into the Markdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum RawHtml {
    /// Write every raw HTML node through untouched.
    Allow,
    /// Neutralize the tags that restyle or script the page, and leave all
    /// other markup working.
    Filter,
    /// Escape every raw HTML node, so the document shows its own markup.
    Escape,
}

/// The options of a render, named as `config.json` names them. Each one left
/// out (`None`) keeps the renderer's default, so a binding never carries a
/// copy of the defaults that could drift from them.
#[derive(Debug, Clone, Default, PartialEq, Eq, uniffi::Record)]
pub struct RenderOptions {
    #[uniffi(default = None)]
    pub auto_link_urls: Option<bool>,
    #[uniffi(default = None)]
    pub math: Option<bool>,
    #[uniffi(default = None)]
    pub html_embed: Option<bool>,
    #[uniffi(default = None)]
    pub wiki_links: Option<bool>,
    #[uniffi(default = None)]
    pub superscript: Option<bool>,
    #[uniffi(default = None)]
    pub subscript: Option<bool>,
    #[uniffi(default = None)]
    pub definition_lists: Option<bool>,
    #[uniffi(default = None)]
    pub heading_attributes: Option<bool>,
    #[uniffi(default = None)]
    pub smart_punctuation: Option<bool>,
    #[uniffi(default = None)]
    pub cjk_emphasis: Option<bool>,
    #[uniffi(default = None)]
    pub heading_permalinks: Option<bool>,
    #[uniffi(default = None)]
    pub raw_html: Option<RawHtml>,
    /// Leave local images to the host, written as
    /// `{deferred_image_base_url}/{id}` and listed in
    /// [`RenderResult::images`], instead of inlining them as data URLs.
    #[uniffi(default = None)]
    pub deferred_image_base_url: Option<String>,
}

impl From<RenderOptions> for arto_markdown::RenderOptions {
    fn from(options: RenderOptions) -> Self {
        let defaults = Self::default();
        Self {
            auto_link_urls: options.auto_link_urls.unwrap_or(defaults.auto_link_urls),
            math: options.math.unwrap_or(defaults.math),
            html_embed: options.html_embed.unwrap_or(defaults.html_embed),
            wiki_links: options.wiki_links.unwrap_or(defaults.wiki_links),
            superscript: options.superscript.unwrap_or(defaults.superscript),
            subscript: options.subscript.unwrap_or(defaults.subscript),
            definition_lists: options
                .definition_lists
                .unwrap_or(defaults.definition_lists),
            heading_attributes: options
                .heading_attributes
                .unwrap_or(defaults.heading_attributes),
            smart_punctuation: options
                .smart_punctuation
                .unwrap_or(defaults.smart_punctuation),
            cjk_emphasis: options.cjk_emphasis.unwrap_or(defaults.cjk_emphasis),
            heading_permalinks: options
                .heading_permalinks
                .unwrap_or(defaults.heading_permalinks),
            raw_html: options
                .raw_html
                .map_or(defaults.raw_html, |raw_html| match raw_html {
                    RawHtml::Allow => arto_markdown::RawHtml::Allow,
                    RawHtml::Filter => arto_markdown::RawHtml::Filter,
                    RawHtml::Escape => arto_markdown::RawHtml::Escape,
                }),
            images: match options.deferred_image_base_url {
                Some(base_url) => ImageResolution::Deferred { base_url },
                None => defaults.images,
            },
        }
    }
}

/// A file as the host holds it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FileStat {
    /// The one spelling of the file, whatever path named it.
    pub path: String,
    pub len: u64,
    /// When the content last changed, when the host knows.
    #[uniffi(default = None)]
    pub modified: Option<SystemTime>,
}

/// The files a document names, as the host holds them, addressed by the
/// paths it renders documents at.
#[uniffi::export(with_foreign)]
pub trait Files: Send + Sync {
    /// The regular file `path` names, or `None` when there is none.
    fn stat(&self, path: String) -> Option<FileStat>;

    /// The bytes of the file `path` names, or `None` when it cannot be read
    /// or holds more than `limit` bytes.
    fn read(&self, path: String, limit: u64) -> Option<Vec<u8>>;
}

/// The host's [`Files`], or none at all: a phone has no disk the renderer
/// may read on its own, so a document rendered without them names no file
/// that exists.
struct HostFiles(Option<Arc<dyn Files>>);

impl arto_markdown::Files for HostFiles {
    fn stat(&self, path: &Path) -> Option<arto_markdown::FileStat> {
        let stat = self.0.as_ref()?.stat(path.to_string_lossy().into_owned())?;
        Some(arto_markdown::FileStat {
            path: PathBuf::from(stat.path),
            len: stat.len,
            modified: stat.modified,
        })
    }

    fn read(&self, path: &Path, limit: u64) -> Option<Vec<u8>> {
        let bytes = self
            .0
            .as_ref()?
            .read(path.to_string_lossy().into_owned(), limit)?;
        // The limit is the renderer's guard, so it holds whatever the host
        // answered.
        (bytes.len() as u64 <= limit).then_some(bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Heading {
    pub level: u8,
    pub text: String,
    pub id: String,
}

/// A local image the host agreed to serve, under the id the HTML names it by.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct DeferredImage {
    pub id: String,
    pub path: String,
    pub mime: String,
}

/// What one top-level block of the document asks a reader to read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct ReadingBlock {
    pub line: u32,
    pub cjk_chars: u32,
    pub words: u32,
    pub code_lines: u32,
    pub figures: u32,
}

/// What a render produced.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RenderResult {
    pub html: String,
    /// Every heading, when a table of contents was asked for.
    pub headings: Vec<Heading>,
    /// The images left to the host.
    pub images: Vec<DeferredImage>,
    /// What the document asks a reader to read, block by block.
    pub reading: Vec<ReadingBlock>,
    /// The libraries the HTML needs, by their names in `manifest.json`, in
    /// the order to load them.
    pub libraries: Vec<String>,
}

impl From<arto_markdown::RenderResult> for RenderResult {
    fn from(rendered: arto_markdown::RenderResult) -> Self {
        Self {
            libraries: rendered
                .libraries
                .iter()
                .map(|library| library.name().to_owned())
                .collect(),
            html: rendered.html,
            headings: rendered
                .headings
                .into_iter()
                .map(|heading| Heading {
                    level: heading.level,
                    text: heading.text,
                    id: heading.id,
                })
                .collect(),
            images: rendered
                .images
                .into_iter()
                .map(|image| DeferredImage {
                    id: image.id,
                    path: image.path.to_string_lossy().into_owned(),
                    mime: image.mime.to_owned(),
                })
                .collect(),
            reading: rendered
                .reading
                .blocks
                .into_iter()
                .map(|block| ReadingBlock {
                    line: block.line,
                    cjk_chars: block.cjk_chars,
                    words: block.words,
                    code_lines: block.code_lines,
                    figures: block.figures,
                })
                .collect(),
        }
    }
}

/// Why a document could not be rendered.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum RenderError {
    /// The Markdown parser gave up on the document, for `reason`.
    ///
    /// Not `message`: Kotlin makes this an exception, and a field of that
    /// name redeclares `Throwable.message`, so the bindings would not compile.
    #[error("failed to parse Markdown: {reason}")]
    Parse { reason: String },
}

impl From<arto_markdown::RenderError> for RenderError {
    fn from(error: arto_markdown::RenderError) -> Self {
        let reason = std::error::Error::source(&error)
            .map_or_else(|| error.to_string(), ToString::to_string);
        Self::Parse { reason }
    }
}

fn with_renderer(
    options: RenderOptions,
    files: Option<Arc<dyn Files>>,
    render: impl FnOnce(
        &Renderer,
        &arto_markdown::RenderOptions,
    ) -> Result<arto_markdown::RenderResult, arto_markdown::RenderError>,
) -> Result<RenderResult, RenderError> {
    let files = HostFiles(files);
    Ok(render(&Renderer::new(&files), &options.into())?.into())
}

/// Render Markdown to HTML. Relative links and images resolve against the
/// directory of `base_path`.
#[uniffi::export(default(files = None, options))]
pub fn render(
    markdown: String,
    base_path: String,
    options: RenderOptions,
    files: Option<Arc<dyn Files>>,
) -> Result<RenderResult, RenderError> {
    with_renderer(options, files, |renderer, options| {
        renderer.render_to_html(markdown, base_path, options)
    })
}

/// [`render`], keeping the heading ids and reporting the headings.
#[uniffi::export(default(files = None, options))]
pub fn render_with_toc(
    markdown: String,
    base_path: String,
    options: RenderOptions,
    files: Option<Arc<dyn Files>>,
) -> Result<RenderResult, RenderError> {
    with_renderer(options, files, |renderer, options| {
        renderer.render_to_html_with_toc(markdown, base_path, options)
    })
}

/// [`render`] for Markdown that was not read from the file it is shown with,
/// fetching nothing from elsewhere.
#[uniffi::export(default(files = None, options))]
pub fn render_detached(
    markdown: String,
    base_path: String,
    options: RenderOptions,
    files: Option<Arc<dyn Files>>,
) -> Result<RenderResult, RenderError> {
    with_renderer(options, files, |renderer, options| {
        renderer.render_detached(markdown, base_path, options)
    })
}

/// The document at `base_path`, rendered to be previewed beside a link to it
/// on another document's page.
#[uniffi::export(default(files = None, options))]
pub fn render_preview(
    markdown: String,
    base_path: String,
    options: RenderOptions,
    files: Option<Arc<dyn Files>>,
) -> Result<RenderResult, RenderError> {
    with_renderer(options, files, |renderer, options| {
        renderer.render_preview(markdown, base_path, options)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    type Ours = fn(&mut RenderOptions, bool);
    type Theirs = fn(&mut arto_markdown::RenderOptions, bool);

    /// Every boolean option, as set here and as set on the renderer's own.
    const BOOLEANS: [(&str, Ours, Theirs); 11] = [
        (
            "auto_link_urls",
            |o, v| o.auto_link_urls = Some(v),
            |o, v| o.auto_link_urls = v,
        ),
        ("math", |o, v| o.math = Some(v), |o, v| o.math = v),
        (
            "html_embed",
            |o, v| o.html_embed = Some(v),
            |o, v| o.html_embed = v,
        ),
        (
            "wiki_links",
            |o, v| o.wiki_links = Some(v),
            |o, v| o.wiki_links = v,
        ),
        (
            "superscript",
            |o, v| o.superscript = Some(v),
            |o, v| o.superscript = v,
        ),
        (
            "subscript",
            |o, v| o.subscript = Some(v),
            |o, v| o.subscript = v,
        ),
        (
            "definition_lists",
            |o, v| o.definition_lists = Some(v),
            |o, v| o.definition_lists = v,
        ),
        (
            "heading_attributes",
            |o, v| o.heading_attributes = Some(v),
            |o, v| o.heading_attributes = v,
        ),
        (
            "smart_punctuation",
            |o, v| o.smart_punctuation = Some(v),
            |o, v| o.smart_punctuation = v,
        ),
        (
            "cjk_emphasis",
            |o, v| o.cjk_emphasis = Some(v),
            |o, v| o.cjk_emphasis = v,
        ),
        (
            "heading_permalinks",
            |o, v| o.heading_permalinks = Some(v),
            |o, v| o.heading_permalinks = v,
        ),
    ];

    #[test]
    fn test_no_option_given_is_the_renderer_s_defaults() {
        assert_eq!(
            arto_markdown::RenderOptions::from(RenderOptions::default()),
            arto_markdown::RenderOptions::default()
        );
    }

    #[test]
    fn test_each_option_given_sets_the_renderer_s_option_of_its_name_and_no_other() {
        for value in [true, false] {
            for (name, ours, theirs) in BOOLEANS {
                let mut given = RenderOptions::default();
                ours(&mut given, value);
                let mut expected = arto_markdown::RenderOptions::default();
                theirs(&mut expected, value);

                assert_eq!(
                    arto_markdown::RenderOptions::from(given),
                    expected,
                    "{name} = {value}"
                );
            }
        }
    }

    #[test]
    fn test_each_raw_html_choice_reaches_the_renderer() {
        for (given, expected) in [
            (RawHtml::Allow, arto_markdown::RawHtml::Allow),
            (RawHtml::Filter, arto_markdown::RawHtml::Filter),
            (RawHtml::Escape, arto_markdown::RawHtml::Escape),
        ] {
            let options = RenderOptions {
                raw_html: Some(given),
                ..RenderOptions::default()
            };

            assert_eq!(
                arto_markdown::RenderOptions::from(options).raw_html,
                expected
            );
        }
    }

    #[test]
    fn test_a_deferred_image_base_url_defers_the_images() {
        let options = RenderOptions {
            deferred_image_base_url: Some("https://host.test/images".into()),
            ..RenderOptions::default()
        };

        assert_eq!(
            arto_markdown::RenderOptions::from(options).images,
            ImageResolution::Deferred {
                base_url: "https://host.test/images".into()
            }
        );
    }
}
