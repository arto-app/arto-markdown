use super::*;
use arto_markdown::{Highlighting, Libraries, Library};

fn libraries(markdown: &str) -> Libraries {
    render_to_html(
        markdown,
        Path::new("/nonexistent/test.md"),
        &RenderOptions::default(),
    )
    .expect("renders")
    .libraries
}

#[test]
fn test_a_prose_document_needs_no_library() {
    assert_eq!(libraries("Just *prose*.\n"), Libraries::default());
    assert_eq!(libraries("Just *prose*.\n").iter().count(), 0);
}

#[test]
fn test_a_diagram_needs_mermaid() {
    let markdown = indoc! {"
        ```mermaid
        graph TD;
        ```
    "};

    assert!(libraries(markdown).mermaid);
}

#[test]
fn test_a_formula_needs_math() {
    assert!(libraries("Euler: $e^{i\\pi} + 1 = 0$\n").math);
}

#[test]
fn test_a_code_block_in_a_usual_language_needs_the_common_highlighter() {
    let markdown = indoc! {"
        ```rust
        fn main() {}
        ```
    "};

    assert_eq!(libraries(markdown).highlighting, Highlighting::Common);
}

#[test]
fn test_the_frontmatter_needs_nothing_of_its_own() {
    let markdown = indoc! {"
        ---
        title: A document
        ---

        Prose.
    "};

    assert_eq!(libraries(markdown), Libraries::default());
}

#[test]
fn test_the_libraries_come_named_in_the_order_to_load_them() {
    let markdown = indoc! {"
        $x$

        ```mermaid
        graph TD;
        ```

        ```brainfuck
        +++.
        ```
    "};

    assert_eq!(
        libraries(markdown).iter().collect::<Vec<_>>(),
        [Library::Mermaid, Library::Math, Library::HljsAll]
    );
    assert_eq!(
        libraries(markdown)
            .iter()
            .map(Library::name)
            .collect::<Vec<_>>(),
        ["mermaid", "math", "hljs-all"]
    );
}

#[test]
fn test_a_preview_reports_what_its_own_html_needs() {
    let markdown = indoc! {"
        ```mermaid
        graph TD;
        ```
    "};
    let preview = render_preview(
        markdown,
        Path::new("/nonexistent/test.md"),
        &RenderOptions::default(),
    )
    .expect("renders");

    assert_eq!(preview.libraries, Libraries::for_fragment(&preview.html));
    assert!(preview.libraries.mermaid);
}
