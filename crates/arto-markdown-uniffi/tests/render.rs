//! The bindings as Swift and Kotlin call them, called from Rust: what each
//! render returns, and what it asks of the host's files.

use arto_markdown_uniffi::{
    render, render_detached, render_preview, render_with_toc, FileStat, Files, Heading, RawHtml,
    ReadingBlock, RenderError, RenderOptions,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// A folder held in memory, addressed by paths that exist nowhere on disk.
struct MemoryFiles(HashMap<String, Vec<u8>>);

impl MemoryFiles {
    fn holding(files: &[(&str, &[u8])]) -> Arc<dyn Files> {
        Arc::new(Self(
            files
                .iter()
                .map(|(path, bytes)| (path.to_string(), bytes.to_vec()))
                .collect(),
        ))
    }
}

/// `path` with every `.` and `..` taken out, the way the store spells it.
/// The renderer joins paths with the platform's separator, so on Windows a
/// path it asks about holds `\` as well as the `/` the document wrote.
fn normalize(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split(['/', '\\']) {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            part => parts.push(part),
        }
    }
    format!("/{}", parts.join("/"))
}

impl Files for MemoryFiles {
    fn stat(&self, path: String) -> Option<FileStat> {
        let path = normalize(&path);
        let bytes = self.0.get(&path)?;
        Some(FileStat {
            len: bytes.len() as u64,
            modified: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1)),
            path,
        })
    }

    fn read(&self, path: String, limit: u64) -> Option<Vec<u8>> {
        let bytes = self.0.get(&normalize(&path))?;
        (bytes.len() as u64 <= limit).then(|| bytes.clone())
    }
}

const PNG: &[u8] = &[0x89, 0x50, 0x4e, 0x47];

#[test]
fn renders_markdown_to_the_html_the_crate_writes() {
    let rendered = render(
        "# Hello\n\n> [!NOTE]\n> Hi.\n".into(),
        "/docs/hello.md".into(),
        RenderOptions::default(),
        None,
    )
    .expect("renders");

    assert!(
        rendered.html.contains("markdown-alert-note"),
        "{}",
        rendered.html
    );
    assert!(rendered.headings.is_empty());
}

#[test]
fn names_the_libraries_the_html_needs_in_the_order_to_load_them() {
    let markdown = "$x$\n\n```mermaid\ngraph TD;\n```\n\n```rust\nfn main() {}\n```\n";

    let rendered = render(
        markdown.into(),
        "/docs/a.md".into(),
        RenderOptions::default(),
        None,
    )
    .expect("renders");

    assert_eq!(rendered.libraries, ["mermaid", "math", "hljs-common"]);
}

#[test]
fn reports_the_headings_when_a_table_of_contents_is_asked_for() {
    let rendered = render_with_toc(
        "# One\n\n## Two\n".into(),
        "/docs/a.md".into(),
        RenderOptions::default(),
        None,
    )
    .expect("renders");

    assert_eq!(
        rendered.headings,
        [
            Heading {
                level: 1,
                text: "One".into(),
                id: "one".into()
            },
            Heading {
                level: 2,
                text: "Two".into(),
                id: "two".into()
            },
        ]
    );
}

#[test]
fn an_option_left_out_keeps_the_renderer_s_default() {
    let math_off = RenderOptions {
        math: Some(false),
        ..RenderOptions::default()
    };
    let escaped = RenderOptions {
        raw_html: Some(RawHtml::Escape),
        ..RenderOptions::default()
    };
    let render_with = |markdown: &str, options| {
        render(markdown.into(), "/docs/a.md".into(), options, None).expect("renders")
    };

    assert_eq!(
        render_with("$x$\n", RenderOptions::default()).libraries,
        ["math"]
    );
    assert!(render_with("$x$\n", math_off).libraries.is_empty());
    assert!(render_with("<b>bold</b>\n", escaped)
        .html
        .contains("&lt;b&gt;"));
}

#[test]
fn asks_the_host_s_files_whether_a_linked_document_exists() {
    let files = MemoryFiles::holding(&[("/notes/other.md", b"# Other")]);

    let html = render(
        "[there](./other.md) [gone](./gone.md)\n".into(),
        "/notes/doc.md".into(),
        RenderOptions::default(),
        Some(files),
    )
    .expect("renders")
    .html;

    assert!(
        html.contains(r#"data-md-link="./other.md" class="md-link""#),
        "{html}"
    );
    assert!(
        html.contains(r#"data-md-link="./gone.md" class="md-link md-link-missing""#),
        "{html}"
    );
}

#[test]
fn without_files_no_file_a_document_names_exists() {
    let html = render(
        "[there](./other.md)\n".into(),
        "/notes/doc.md".into(),
        RenderOptions::default(),
        None,
    )
    .expect("renders")
    .html;

    assert!(html.contains("md-link-missing"), "{html}");
}

#[test]
fn inlines_an_image_the_host_s_files_hold() {
    let files = MemoryFiles::holding(&[("/notes/pic.png", PNG)]);

    let html = render(
        "![pic](./pic.png)\n".into(),
        "/notes/doc.md".into(),
        RenderOptions::default(),
        Some(files),
    )
    .expect("renders")
    .html;

    assert!(
        html.contains(r#"src="data:image/png;base64,iVBORw==""#),
        "{html}"
    );
}

#[test]
fn leaves_an_image_to_the_host_when_told_where_it_serves_them() {
    let files = MemoryFiles::holding(&[("/notes/pic.png", PNG)]);
    let options = RenderOptions {
        deferred_image_base_url: Some("https://host.test/images".into()),
        ..RenderOptions::default()
    };

    let rendered = render(
        "![pic](./pic.png)\n".into(),
        "/notes/doc.md".into(),
        options,
        Some(files),
    )
    .expect("renders");

    let [image] = rendered.images.as_slice() else {
        panic!("one image: {:?}", rendered.images);
    };
    assert_eq!(image.path, "/notes/pic.png");
    assert_eq!(image.mime, "image/png");
    assert!(
        rendered
            .html
            .contains(&format!(r#"src="https://host.test/images/{}"#, image.id)),
        "{}",
        rendered.html
    );
}

#[test]
fn counts_what_the_document_asks_a_reader_to_read() {
    let rendered = render(
        "Two words.\n\n```\ncode\n```\n".into(),
        "/docs/a.md".into(),
        RenderOptions::default(),
        None,
    )
    .expect("renders");

    assert_eq!(
        rendered.reading,
        [
            ReadingBlock {
                line: 1,
                cjk_chars: 0,
                words: 2,
                code_lines: 0,
                figures: 0
            },
            ReadingBlock {
                line: 3,
                cjk_chars: 0,
                words: 0,
                code_lines: 1,
                figures: 0
            },
        ]
    );
}

#[test]
fn a_detached_render_and_a_preview_fetch_nothing_from_elsewhere() {
    let remote = "![remote](https://elsewhere.test/a.png)\n";

    let detached = render_detached(
        remote.into(),
        "/docs/a.md".into(),
        RenderOptions::default(),
        None,
    )
    .expect("renders");
    let preview = render_preview(
        remote.into(),
        "/docs/a.md".into(),
        RenderOptions::default(),
        None,
    )
    .expect("renders");

    assert!(
        !detached.html.contains("elsewhere.test"),
        "{}",
        detached.html
    );
    assert!(!preview.html.contains("elsewhere.test"), "{}", preview.html);
}

#[test]
fn a_document_the_parser_gives_up_on_is_a_parse_error() {
    let markdown = format!("{} too deep", ">".repeat(200));

    let error = render(
        markdown,
        "/docs/a.md".into(),
        RenderOptions::default(),
        None,
    )
    .expect_err("the parser gives up");

    assert!(matches!(error, RenderError::Parse { .. }), "{error:?}");
}

/// One PNG at `/notes/pic.png`, answered as the host chooses to.
struct OnePicture {
    modified: Option<SystemTime>,
    /// Answer `read` with this many bytes, whatever limit it was given.
    answer_len: Option<usize>,
}

impl Files for OnePicture {
    fn stat(&self, path: String) -> Option<FileStat> {
        (normalize(&path) == "/notes/pic.png").then(|| FileStat {
            path: "/notes/pic.png".into(),
            len: PNG.len() as u64,
            modified: self.modified,
        })
    }

    fn read(&self, path: String, _limit: u64) -> Option<Vec<u8>> {
        (normalize(&path) == "/notes/pic.png").then(|| match self.answer_len {
            Some(len) => vec![0; len],
            None => PNG.to_vec(),
        })
    }
}

#[test]
fn bytes_beyond_the_limit_are_not_drawn_whatever_the_host_answers() {
    let files: Arc<dyn Files> = Arc::new(OnePicture {
        modified: None,
        answer_len: Some(arto_markdown::MAX_IMAGE_BYTES as usize + 1),
    });

    let html = render(
        "![pic](./pic.png)\n".into(),
        "/notes/doc.md".into(),
        RenderOptions::default(),
        Some(files),
    )
    .expect("renders")
    .html;

    assert!(!html.contains("data:"), "{}", &html[..html.len().min(300)]);
}

#[test]
fn a_file_that_changed_gets_a_url_of_its_own() {
    let deferred_id = |modified| {
        let files: Arc<dyn Files> = Arc::new(OnePicture {
            modified,
            answer_len: None,
        });
        let options = RenderOptions {
            deferred_image_base_url: Some("https://host.test/images".into()),
            ..RenderOptions::default()
        };
        render(
            "![pic](./pic.png)\n".into(),
            "/notes/doc.md".into(),
            options,
            Some(files),
        )
        .expect("renders")
        .images
        .into_iter()
        .map(|image| image.id)
        .collect::<Vec<_>>()
    };
    let at = |secs| Some(SystemTime::UNIX_EPOCH + Duration::from_secs(secs));

    assert_eq!(deferred_id(at(1)), deferred_id(at(1)));
    assert_ne!(deferred_id(at(1)), deferred_id(at(2)));
}
