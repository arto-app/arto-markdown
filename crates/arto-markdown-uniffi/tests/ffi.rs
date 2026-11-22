//! The renderer as Swift and Kotlin reach it: through the C functions UniFFI
//! exports, with every argument and result serialized the way the generated
//! bindings serialize them, and the host's files a foreign object behind the
//! vtable the bindings register.
//!
//! `tests/render.rs` calls the Rust functions; this calls what a binding
//! calls, so a value that does not survive the boundary, a callback the
//! renderer cannot reach, or a handle that is never released fails here
//! rather than in an app.

use arto_markdown_uniffi::{FileStat, RenderOptions, RenderResult, UniFfiTag};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{LazyLock, Mutex, Once};
use std::time::{Duration, SystemTime};
use uniffi::{Lift, Lower, RustBuffer, RustCallStatus, RustCallStatusCode};

/// The functions the generated bindings call, as the generated C header
/// declares them.
mod ffi {
    use super::*;

    unsafe extern "C" {
        pub fn uniffi_arto_markdown_uniffi_fn_func_render(
            markdown: RustBuffer,
            base_path: RustBuffer,
            options: RustBuffer,
            files: RustBuffer,
            status: &mut RustCallStatus,
        ) -> RustBuffer;
        pub fn uniffi_arto_markdown_uniffi_fn_func_render_with_toc(
            markdown: RustBuffer,
            base_path: RustBuffer,
            options: RustBuffer,
            files: RustBuffer,
            status: &mut RustCallStatus,
        ) -> RustBuffer;
        pub fn uniffi_arto_markdown_uniffi_fn_func_render_detached(
            markdown: RustBuffer,
            base_path: RustBuffer,
            options: RustBuffer,
            files: RustBuffer,
            status: &mut RustCallStatus,
        ) -> RustBuffer;
        pub fn uniffi_arto_markdown_uniffi_fn_func_render_preview(
            markdown: RustBuffer,
            base_path: RustBuffer,
            options: RustBuffer,
            files: RustBuffer,
            status: &mut RustCallStatus,
        ) -> RustBuffer;
        pub fn uniffi_arto_markdown_uniffi_fn_init_callback_vtable_files(vtable: &'static VTable);
    }

    /// `UniffiVTableCallbackInterfaceFiles`.
    #[repr(C)]
    pub struct VTable {
        pub free: extern "C" fn(handle: u64),
        pub clone: extern "C" fn(handle: u64) -> u64,
        pub stat: extern "C" fn(
            handle: u64,
            path: RustBuffer,
            out: &mut RustBuffer,
            status: &mut RustCallStatus,
        ),
        pub read: extern "C" fn(
            handle: u64,
            path: RustBuffer,
            limit: u64,
            out: &mut RustBuffer,
            status: &mut RustCallStatus,
        ),
    }
}

/// The foreign side's files: one folder per handle, as a binding's handle
/// map would hold them. Foreign handles are odd, which is how the Rust side
/// tells them from its own.
/// A folder: its files, by the paths they are stored at.
type Folder = HashMap<String, Vec<u8>>;

static FOLDERS: LazyLock<Mutex<HashMap<u64, Folder>>> = LazyLock::new(Default::default);
static NEXT_HANDLE: AtomicUsize = AtomicUsize::new(1);
/// The modification time each folder's files report, in seconds.
static MODIFIED: LazyLock<Mutex<HashMap<u64, u64>>> = LazyLock::new(Default::default);
static ASKED: LazyLock<Mutex<HashMap<u64, Vec<String>>>> = LazyLock::new(Default::default);

extern "C" fn free(handle: u64) {
    FOLDERS.lock().unwrap().remove(&handle);
}

extern "C" fn clone(handle: u64) -> u64 {
    let mut folders = FOLDERS.lock().unwrap();
    let copy = folders[&handle].clone();
    let new = NEXT_HANDLE.fetch_add(2, Ordering::Relaxed) as u64;
    folders.insert(new, copy);
    new
}

fn lift_path(path: RustBuffer) -> String {
    <String as Lift<UniFfiTag>>::try_lift(path).expect("a path comes across as a string")
}

extern "C" fn stat(handle: u64, path: RustBuffer, out: &mut RustBuffer, _: &mut RustCallStatus) {
    let path = lift_path(path);
    ASKED
        .lock()
        .unwrap()
        .entry(handle)
        .or_default()
        .push(path.clone());
    let path = normalize(&path);
    let answer = FOLDERS.lock().unwrap()[&handle]
        .get(&path)
        .map(|bytes| FileStat {
            len: bytes.len() as u64,
            modified: Some(
                SystemTime::UNIX_EPOCH
                    + Duration::from_secs(
                        MODIFIED.lock().unwrap().get(&handle).copied().unwrap_or(1),
                    ),
            ),
            path,
        });
    *out = <Option<FileStat> as Lower<UniFfiTag>>::lower(answer);
}

extern "C" fn read(
    handle: u64,
    path: RustBuffer,
    limit: u64,
    out: &mut RustBuffer,
    _: &mut RustCallStatus,
) {
    let path = normalize(&lift_path(path));
    let answer = FOLDERS.lock().unwrap()[&handle]
        .get(&path)
        .filter(|bytes| bytes.len() as u64 <= limit)
        .cloned();
    *out = <Option<Vec<u8>> as Lower<UniFfiTag>>::lower(answer);
}

static VTABLE: ffi::VTable = ffi::VTable {
    free,
    clone,
    stat,
    read,
};

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

/// A foreign `Files` holding `files`, as `Option<Files>` comes across: a
/// present-tag followed by the handle.
fn foreign_files(files: &[(&str, &[u8])]) -> (u64, RustBuffer) {
    foreign_files_modified_at(files, 1)
}

/// [`foreign_files`], each reporting it last changed `modified` seconds after
/// the epoch.
fn foreign_files_modified_at(files: &[(&str, &[u8])], modified: u64) -> (u64, RustBuffer) {
    static REGISTER: Once = Once::new();
    REGISTER.call_once(|| unsafe {
        ffi::uniffi_arto_markdown_uniffi_fn_init_callback_vtable_files(&VTABLE)
    });
    let handle = NEXT_HANDLE.fetch_add(2, Ordering::Relaxed) as u64;
    MODIFIED.lock().unwrap().insert(handle, modified);
    FOLDERS.lock().unwrap().insert(
        handle,
        files
            .iter()
            .map(|(path, bytes)| (path.to_string(), bytes.to_vec()))
            .collect(),
    );
    let mut lowered = vec![1u8];
    lowered.extend_from_slice(&handle.to_be_bytes());
    (handle, RustBuffer::from_vec(lowered))
}

fn no_files() -> RustBuffer {
    RustBuffer::from_vec(vec![0])
}

type Entry = unsafe extern "C" fn(
    RustBuffer,
    RustBuffer,
    RustBuffer,
    RustBuffer,
    &mut RustCallStatus,
) -> RustBuffer;

/// Call `entry` as a binding does and lift what it answered.
fn call(
    entry: Entry,
    markdown: &str,
    base_path: &str,
    options: RenderOptions,
    files: RustBuffer,
) -> Result<RenderResult, (RustCallStatusCode, Vec<u8>)> {
    let mut status = RustCallStatus::default();
    let returned = unsafe {
        entry(
            <String as Lower<UniFfiTag>>::lower(markdown.into()),
            <String as Lower<UniFfiTag>>::lower(base_path.into()),
            <RenderOptions as Lower<UniFfiTag>>::lower(options),
            files,
            &mut status,
        )
    };
    match status.code {
        RustCallStatusCode::Success => Ok(<RenderResult as Lift<UniFfiTag>>::try_lift(returned)
            .expect("the result comes across as a RenderResult")),
        code => Err((
            code,
            std::mem::ManuallyDrop::into_inner(status.error_buf).destroy_into_vec(),
        )),
    }
}

fn render(
    markdown: &str,
    base_path: &str,
    options: RenderOptions,
    files: RustBuffer,
) -> RenderResult {
    call(
        ffi::uniffi_arto_markdown_uniffi_fn_func_render,
        markdown,
        base_path,
        options,
        files,
    )
    .expect("renders")
}

const PNG: &[u8] = &[0x89, 0x50, 0x4e, 0x47];

#[test]
fn a_document_comes_back_across_the_boundary_as_the_renderer_wrote_it() {
    let markdown = "# Hello\n\n> [!NOTE]\n> Hi.\n\n$x$\n\n```rust\nfn main() {}\n```\n";

    let across = render(markdown, "/docs/a.md", RenderOptions::default(), no_files());
    let direct = arto_markdown_uniffi::render(
        markdown.into(),
        "/docs/a.md".into(),
        RenderOptions::default(),
        None,
    )
    .expect("renders");

    assert_eq!(across, direct);
    assert_eq!(across.libraries, ["math", "hljs-common"]);
}

#[test]
fn an_option_crosses_the_boundary() {
    let options = RenderOptions {
        math: Some(false),
        ..RenderOptions::default()
    };

    assert!(render("$x$\n", "/docs/a.md", options, no_files())
        .libraries
        .is_empty());
}

#[test]
fn each_entry_point_is_the_render_of_its_name() {
    let toc = call(
        ffi::uniffi_arto_markdown_uniffi_fn_func_render_with_toc,
        "# One\n\n## Two\n",
        "/docs/a.md",
        RenderOptions::default(),
        no_files(),
    )
    .expect("renders");
    assert_eq!(
        toc.headings
            .iter()
            .map(|h| h.id.as_str())
            .collect::<Vec<_>>(),
        ["one", "two"]
    );

    let remote = "![remote](https://elsewhere.test/a.png)\n";
    for entry in [
        ffi::uniffi_arto_markdown_uniffi_fn_func_render_detached as Entry,
        ffi::uniffi_arto_markdown_uniffi_fn_func_render_preview,
    ] {
        let html = call(
            entry,
            remote,
            "/docs/a.md",
            RenderOptions::default(),
            no_files(),
        )
        .expect("renders")
        .html;
        assert!(!html.contains("elsewhere.test"), "{html}");
    }
}

#[test]
fn the_renderer_asks_the_foreign_side_about_the_files_a_document_names() {
    let (handle, files) =
        foreign_files(&[("/notes/other.md", b"# Other"), ("/notes/pic.png", PNG)]);

    let html = render(
        "[there](./other.md) [gone](./gone.md) ![pic](./pic.png)\n",
        "/notes/doc.md",
        RenderOptions::default(),
        files,
    )
    .html;

    assert!(
        html.contains(r#"data-md-link="./other.md" class="md-link""#),
        "{html}"
    );
    assert!(
        html.contains(r#"data-md-link="./gone.md" class="md-link md-link-missing""#),
        "{html}"
    );
    assert!(
        html.contains(r#"src="data:image/png;base64,iVBORw==""#),
        "{html}"
    );
    assert!(
        !ASKED.lock().unwrap()[&handle].is_empty(),
        "the foreign callbacks were called"
    );
}

#[test]
fn the_foreign_files_are_released_once_the_render_is_done() {
    let (handle, files) = foreign_files(&[("/notes/other.md", b"# Other")]);

    render(
        "[there](./other.md)\n",
        "/notes/doc.md",
        RenderOptions::default(),
        files,
    );

    assert!(
        !FOLDERS.lock().unwrap().contains_key(&handle),
        "the renderer freed the handle it was given"
    );
}

#[test]
fn a_foreign_timestamp_reaches_the_url_of_a_deferred_image() {
    let deferred = |modified| {
        let (_, files) = foreign_files_modified_at(&[("/n/pic.png", PNG)], modified);
        let options = RenderOptions {
            deferred_image_base_url: Some("https://host.test/images".into()),
            ..RenderOptions::default()
        };
        let rendered = render("![pic](./pic.png)\n", "/n/doc.md", options, files);
        let [image] = rendered.images.as_slice() else {
            panic!("one image: {:?}", rendered.images);
        };
        assert_eq!(image.mime, "image/png");
        image.id.clone()
    };

    // The same file, told apart only by the time the foreign side reported:
    // the id moves with it only if the time came across.
    assert_eq!(deferred(1), deferred(1));
    assert_ne!(deferred(1), deferred(2));
}

#[test]
fn a_document_the_parser_gives_up_on_comes_back_as_the_error_the_binding_raises() {
    let markdown = format!("{} too deep", ">".repeat(200));

    let (code, error) = call(
        ffi::uniffi_arto_markdown_uniffi_fn_func_render,
        &markdown,
        "/docs/a.md",
        RenderOptions::default(),
        no_files(),
    )
    .expect_err("the parser gives up");

    assert_eq!(code, RustCallStatusCode::Error);
    // `RenderError::Parse`, the first variant, numbered from 1.
    assert_eq!(&error[..4], &1i32.to_be_bytes());
}
