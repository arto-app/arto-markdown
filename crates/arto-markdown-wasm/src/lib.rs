//! `arto-markdown` for a browser, through WebAssembly.
//!
//! Built by `just wasm-package` into `packages/markdown/wasm/` and published
//! inside the npm package `@arto-app/markdown` as its `./wasm` export. Each
//! render function is the crate's of the same name, with the options
//! `config.json` names, the files a document names asked of an object the
//! page hands over, and the result as a plain object.

use arto_markdown::{FileStat, Files, ImageResolution, RenderOptions, RenderResult, Renderer};
use js_sys::Uint8Array;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const TYPES: &str = r#"
/** The options of a render, named as `config.json` names them; each one left out keeps its default. */
export interface RenderOptions {
  autoLinkUrls?: boolean;
  math?: boolean;
  htmlEmbed?: boolean;
  wikiLinks?: boolean;
  superscript?: boolean;
  subscript?: boolean;
  definitionLists?: boolean;
  headingAttributes?: boolean;
  smartPunctuation?: boolean;
  cjkEmphasis?: boolean;
  headingPermalinks?: boolean;
  rawHtml?: "allow" | "filter" | "escape";
  /**
   * Leave local images to the host, written as `{deferredImageBaseUrl}/{id}` —
   * followed by `.` and the file's extension when it has one, which the host
   * ignores when it looks the id up — and listed in `images`, instead of
   * inlining them as data URLs.
   */
  deferredImageBaseUrl?: string;
}

/** A file as the host holds it. */
export interface FileStat {
  /** The one spelling of the file, whatever path named it. */
  path: string;
  len: number;
  /** When the content last changed, in milliseconds since the epoch, when the host knows. */
  modified?: number;
}

/** The files a document names, answered by the host. Without them, no file exists. */
export interface Files {
  stat(path: string): FileStat | null | undefined;
  /** The file's bytes, or nothing when it cannot be read or holds more than `limit` bytes. */
  read(path: string, limit: number): Uint8Array | null | undefined;
}

export interface Heading {
  level: number;
  text: string;
  id: string;
}

export interface DeferredImage {
  id: string;
  path: string;
  mime: string;
}

export interface ReadingBlock {
  line: number;
  cjkChars: number;
  words: number;
  codeLines: number;
  figures: number;
}

/** A library bundle, by the name the build's `manifest.json` lists it under. */
export type Library = "mermaid" | "math" | "hljs-common" | "hljs-all";

export interface RenderResult {
  html: string;
  headings: Heading[];
  images: DeferredImage[];
  reading: ReadingBlock[];
  /** The libraries the HTML needs, in the order to load them. */
  libraries: Library[];
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "RenderOptions")]
    pub type JsRenderOptions;

    #[wasm_bindgen(typescript_type = "Files")]
    pub type JsFiles;

    #[wasm_bindgen(method, catch)]
    fn stat(this: &JsFiles, path: &str) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(method, catch)]
    fn read(this: &JsFiles, path: &str, limit: f64) -> Result<JsValue, JsValue>;
}

/// Render Markdown to HTML. Relative links and images resolve against the
/// directory of `basePath`.
#[wasm_bindgen(unchecked_return_type = "RenderResult")]
pub fn render(
    markdown: &str,
    #[wasm_bindgen(js_name = basePath)] base_path: &str,
    options: Option<JsRenderOptions>,
    files: Option<JsFiles>,
) -> Result<JsValue, JsError> {
    with_renderer(options, files, |renderer, options| {
        renderer.render_to_html(markdown, base_path, options)
    })
}

/// `render`, keeping the heading ids and reporting the headings.
#[wasm_bindgen(js_name = renderWithToc, unchecked_return_type = "RenderResult")]
pub fn render_with_toc(
    markdown: &str,
    #[wasm_bindgen(js_name = basePath)] base_path: &str,
    options: Option<JsRenderOptions>,
    files: Option<JsFiles>,
) -> Result<JsValue, JsError> {
    with_renderer(options, files, |renderer, options| {
        renderer.render_to_html_with_toc(markdown, base_path, options)
    })
}

/// `render` for Markdown that was not read from the file it is shown with,
/// fetching nothing from elsewhere.
#[wasm_bindgen(js_name = renderDetached, unchecked_return_type = "RenderResult")]
pub fn render_detached(
    markdown: &str,
    #[wasm_bindgen(js_name = basePath)] base_path: &str,
    options: Option<JsRenderOptions>,
    files: Option<JsFiles>,
) -> Result<JsValue, JsError> {
    with_renderer(options, files, |renderer, options| {
        renderer.render_detached(markdown, base_path, options)
    })
}

/// The document at `basePath`, rendered to be previewed beside a link to it
/// on another document's page.
#[wasm_bindgen(js_name = renderPreview, unchecked_return_type = "RenderResult")]
pub fn render_preview(
    markdown: &str,
    #[wasm_bindgen(js_name = basePath)] base_path: &str,
    options: Option<JsRenderOptions>,
    files: Option<JsFiles>,
) -> Result<JsValue, JsError> {
    with_renderer(options, files, |renderer, options| {
        renderer.render_preview(markdown, base_path, options)
    })
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct Options {
    #[serde(flatten)]
    render: RenderOptions,
    deferred_image_base_url: Option<String>,
}

impl Options {
    fn from_js(options: Option<JsRenderOptions>) -> Result<RenderOptions, JsError> {
        let Some(options) = options else {
            return Ok(RenderOptions::default());
        };
        let options: Self = serde_wasm_bindgen::from_value(options.into())?;
        Ok(RenderOptions {
            images: match options.deferred_image_base_url {
                Some(base_url) => ImageResolution::Deferred { base_url },
                None => ImageResolution::DataUrl,
            },
            ..options.render
        })
    }
}

fn with_renderer(
    options: Option<JsRenderOptions>,
    files: Option<JsFiles>,
    render: impl FnOnce(&Renderer, &RenderOptions) -> Result<RenderResult, arto_markdown::RenderError>,
) -> Result<JsValue, JsError> {
    let options = Options::from_js(options)?;
    let files = HostFiles(files);
    let rendered = render(&Renderer::new(&files), &options)?;
    // Plain objects and arrays, not `Map`s, so the result reads as the
    // interface it is declared as.
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    Ok(Output::from(rendered).serialize(&serializer)?)
}

/// The page's [`Files`], or none at all: a browser has no disk to fall back
/// on, so a document rendered without them names no file that exists.
struct HostFiles(Option<JsFiles>);

#[derive(Deserialize)]
struct JsStat {
    path: String,
    len: f64,
    modified: Option<f64>,
}

impl Files for HostFiles {
    fn stat(&self, path: &Path) -> Option<FileStat> {
        let answer = self.0.as_ref()?.stat(&path.to_string_lossy()).ok()?;
        if answer.is_null() || answer.is_undefined() {
            return None;
        }
        let stat: JsStat = serde_wasm_bindgen::from_value(answer).ok()?;
        Some(FileStat {
            path: PathBuf::from(stat.path),
            len: stat.len as u64,
            modified: stat
                .modified
                .map(|millis| SystemTime::UNIX_EPOCH + Duration::from_millis(millis as u64)),
        })
    }

    fn read(&self, path: &Path, limit: u64) -> Option<Vec<u8>> {
        let answer = self
            .0
            .as_ref()?
            .read(&path.to_string_lossy(), limit as f64)
            .ok()?;
        let bytes = answer.dyn_into::<Uint8Array>().ok()?;
        // The limit is the renderer's guard, so it holds whatever the host
        // answered.
        (u64::from(bytes.length()) <= limit).then(|| bytes.to_vec())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Output {
    html: String,
    headings: Vec<Heading>,
    images: Vec<Image>,
    reading: Vec<Reading>,
    libraries: Vec<&'static str>,
}

#[derive(Serialize)]
struct Heading {
    level: u8,
    text: String,
    id: String,
}

#[derive(Serialize)]
struct Image {
    id: String,
    path: String,
    mime: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Reading {
    line: u32,
    cjk_chars: u32,
    words: u32,
    code_lines: u32,
    figures: u32,
}

impl From<RenderResult> for Output {
    fn from(rendered: RenderResult) -> Self {
        Self {
            libraries: rendered
                .libraries
                .iter()
                .map(|library| library.name())
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
                .map(|image| Image {
                    id: image.id,
                    path: image.path.to_string_lossy().into_owned(),
                    mime: image.mime,
                })
                .collect(),
            reading: rendered
                .reading
                .blocks
                .into_iter()
                .map(|block| Reading {
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
