//! Convert html into rsx

#![warn(
    rust_2018_idioms,
    unused,
    rust_2021_compatibility,
    nonstandard_style,
    future_incompatible,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
    clippy::unwrap_used,
    clippy::missing_assert_message,
    clippy::todo,
    clippy::allow_attributes_without_reason,
    clippy::panic,
    clippy::panicking_unwrap,
    clippy::panic_in_result_fn
)]

use std::borrow::Cow;
use std::collections::VecDeque;

use tl::Node;

use errors::Result;

mod errors;

enum Fragment<'a> {
    TlNode(&'a Node<'a>),
    ClosingBrace,
}

fn escape_string(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('\"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn to_rust_string(input: &str) -> String {
    format!("\"{}\"", escape_string(input))
}

/// Convert html into rsx
///
/// # Errors
///
/// Will return an error if the html is invalid, or if the html contains invalid characters that are not unicode
pub fn convert<'a>(input: impl Into<Cow<'a, str>>) -> Result<String> {
    let input = input.into();
    let dom = tl::parse(input.trim(), tl::ParserOptions::default())?;
    let parser = dom.parser();

    let mut work_stack = dom
        .children()
        .iter()
        .filter_map(|x| x.get(parser))
        .map(Fragment::TlNode)
        .collect::<VecDeque<_>>();

    let mut out = String::new();
    let mut indentation_level = 0;
    let indentation = 4;

    while let Some(work) = work_stack.pop_front() {
        match work {
            Fragment::TlNode(Node::Tag(tag)) => {
                out.push_str(" ".repeat(indentation_level * indentation).as_ref());
                out.push_str(tag.name().try_as_utf8_str().unwrap_or_default());
                out.push_str(" {");

                let mut attr = tag.attributes().iter().collect::<Vec<_>>();
                attr.sort();
                for (key, value) in &attr {
                    let value = value
                        .as_deref()
                        .map_or_else(|| String::from("true"), to_rust_string);

                    let key = key
                        .chars()
                        .enumerate()
                        .map(|(idx, chara)| {
                            if idx == 0 {
                                chara.to_lowercase().to_string()
                            } else {
                                chara.to_string()
                            }
                        })
                        .map(|x| {
                            if x == x.to_lowercase() {
                                x
                            } else {
                                format!("_{}", x.to_lowercase())
                            }
                        })
                        .collect::<String>();

                    out.push('\n');
                    out.push_str(" ".repeat((indentation_level + 1) * indentation).as_ref());
                    out.push_str(key.as_ref());
                    out.push_str(": ");
                    out.push_str(value.as_ref());
                    out.push(',');
                }
                if tag.children().start().is_none() && !tag.attributes().is_empty() {
                    out.push('\n');
                    out.push_str(" ".repeat((indentation_level) * indentation).as_ref());
                }

                if tag.children().start().is_none() {
                    out.push_str("}\n");
                } else {
                    out.push('\n');
                    work_stack.push_front(Fragment::ClosingBrace);

                    for child in tag
                        .children()
                        .top()
                        .iter()
                        .filter_map(|x| x.get(parser))
                        .collect::<Vec<_>>()
                        .iter()
                        .rev()
                    {
                        work_stack.push_front(Fragment::TlNode(child));
                    }
                    indentation_level += 1;
                }
            }
            Fragment::TlNode(Node::Raw(text)) => {
                out.push_str(" ".repeat(indentation_level * indentation).as_ref());
                out.push_str(to_rust_string(text.try_as_utf8_str().unwrap_or_default()).as_ref());
                out.push('\n');
            }
            Fragment::TlNode(Node::Comment(comment)) => {
                out.push_str(" ".repeat(indentation_level * indentation).as_ref());
                out.push_str("// ");
                out.push_str(
                    comment
                        .try_as_utf8_str()
                        .unwrap_or_default()
                        .trim_start_matches("<!-- ")
                        .trim_end_matches(" -->"),
                );
                out.push('\n');
            }
            Fragment::ClosingBrace => {
                indentation_level -= 1;
                out.push_str(" ".repeat(indentation_level * indentation).as_ref());
                out.push_str("}\n");
            }
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::*;

    #[test]
    fn empty_div() {
        let input = indoc! {"
        <div></div>
        "};

        let expected = indoc! {"
        div {}
        "};
        let actual = convert(input);
        assert_eq!(actual.expect("Failed to convert html"), expected);
    }

    #[test]
    fn empty_span() {
        let input = indoc! {"
        <span></span>
        "};

        let expected = indoc! {"
        span {}
        "};
        let actual = convert(input);
        assert_eq!(actual.expect("Failed to convert html"), expected);
    }

    #[test]
    fn self_closing_div() {
        let input = indoc! {"
        <div />
        "};

        let expected = indoc! {"
        div {}
        "};
        let actual = convert(input);
        assert_eq!(actual.expect("Failed to convert html"), expected);
    }

    #[test]
    fn comments_become_rust_comments() {
        let input = indoc! {"
        <div><!-- nothing in here --></div><!-- nothing out here -->
        "};

        let expected = indoc! {"
        div {
            // nothing in here
        }
        // nothing out here
        "};
        let actual = convert(input);
        assert_eq!(actual.expect("Failed to convert html"), expected);
    }

    #[test]
    fn div_with_attributes() {
        let input = indoc! {"
        <div class=\"example\"></div>
        "};

        let expected = indoc! {"
        div {
            class: \"example\",
        }
        "};
        let actual = convert(input);
        assert_eq!(actual.expect("Failed to convert html"), expected);
    }

    #[test]
    fn div_with_multiple_attributes() {
        let input = indoc! {"
        <div class=\"example\" id=\"id\"></div>
        "};

        let expected = indoc! {"
        div {
            class: \"example\",
            id: \"id\",
        }
        "};
        let actual = convert(input);
        assert_eq!(actual.expect("Failed to convert html"), expected);
    }

    #[test]
    fn div_with_inner_text() {
        let input = indoc! {"
        <div>Some text</div>
        "};

        let expected = indoc! {"
        div {
            \"Some text\"
        }
        "};
        let actual = convert(input);
        assert_eq!(actual.expect("Failed to convert html"), expected);
    }

    #[test]
    fn non_snake_capitalised_attributes_are_converted_to_snake() {
        let input = indoc! {r#"
        <div SomeAttribute="door"></div>
        "#};

        let expected = indoc! {r#"
        div {
            some_attribute: "door",
        }
        "#};
        let actual = convert(input);
        assert_eq!(actual.expect("Failed to convert html"), expected);
    }

    #[test]
    fn solo_attributes_are_marked_as_true() {
        let input = indoc! {r"
        <input disabled />
        "};

        let expected = indoc! {r"
        input {
            disabled: true,
        }
        "};
        let actual = convert(input);
        assert_eq!(actual.expect("Failed to convert html"), expected);
    }

    #[test]
    fn realistic_html() {
        let input = indoc! {"
        <html><head><title>HTML Tutorial</title></head><body id=\"body\"><h1>This is a heading</h1><p class=\"bold\">This is a paragraph.</p></body></html>
        "};

        let expected = indoc! {"
        html {
            head {
                title {
                    \"HTML Tutorial\"
                }
            }
            body {
                id: \"body\",
                h1 {
                    \"This is a heading\"
                }
                p {
                    class: \"bold\",
                    \"This is a paragraph.\"
                }
            }
        }
        "};
        let actual = convert(input);
        assert_eq!(actual.expect("Failed to convert html"), expected);
    }

    #[test]
    fn realistic_svg() {
        let input = indoc! {r##"
        <svg width="800px" height="800px" viewBox="0 0 1024 1024" class="icon"  version="1.1" xmlns="http://www.w3.org/2000/svg"><path d="M512 301.2m-10 0a10 10 0 1 0 20 0 10 10 0 1 0-20 0Z" fill="#E73B37" /><path d="M400.3 744.5c2.1-0.7 4.1-1.4 6.2-2-2 0.6-4.1 1.3-6.2 2z m0 0c2.1-0.7 4.1-1.4 6.2-2-2 0.6-4.1 1.3-6.2 2z" fill="#39393A" /><path d="M511.8 256.6c24.4 0 44.2 19.8 44.2 44.2S536.2 345 511.8 345s-44.2-19.8-44.2-44.2 19.9-44.2 44.2-44.2m0-20c-35.5 0-64.2 28.7-64.2 64.2s28.7 64.2 64.2 64.2 64.2-28.7 64.2-64.2-28.7-64.2-64.2-64.2z" fill="#E73B37" /><path d="M730.7 529.5c0.4-8.7 0.6-17.4 0.6-26.2 0-179.6-86.1-339.1-219.3-439.5-133.1 100.4-219.2 259.9-219.2 439.5 0 8.8 0.2 17.5 0.6 26.1-56 56-90.6 133.3-90.6 218.7 0 61.7 18 119.1 49.1 167.3 30.3-49.8 74.7-90.1 127.7-115.3 39-18.6 82.7-29 128.8-29 48.3 0 93.9 11.4 134.3 31.7 52.5 26.3 96.3 67.7 125.6 118.4 33.4-49.4 52.9-108.9 52.9-173.1 0-85.4-34.6-162.6-90.5-218.6zM351.1 383.4c9.2-37.9 22.9-74.7 40.6-109.5a502.1 502.1 0 0 1 63.6-95.9c17.4-20.6 36.4-39.9 56.8-57.5 20.4 17.6 39.4 36.9 56.8 57.5 24.8 29.5 46.2 61.8 63.6 95.9 17.7 34.8 31.4 71.6 40.6 109.5 8.7 35.8 13.5 72.7 14.2 109.9C637.4 459 577 438.9 512 438.9c-65 0-125.3 20.1-175.1 54.4 0.7-37.2 5.5-74.1 14.2-109.9z m-90.6 449.2c-9.1-27-13.7-55.5-13.7-84.4 0-35.8 7-70.6 20.8-103.2 8.4-19.8 19-38.4 31.9-55.5 9.7 61.5 29.5 119.7 57.8 172.6-36.4 17.8-69 41.6-96.8 70.5z m364.2-85.3c-0.7-0.3-1.5-0.5-2.2-0.8-0.4-0.2-0.9-0.3-1.3-0.5-0.6-0.2-1.3-0.5-1.9-0.7-0.8-0.3-1.5-0.5-2.3-0.8-0.8-0.3-1.5-0.5-2.3-0.7l-0.9-0.3c-1-0.3-2.1-0.7-3.1-1-1.2-0.4-2.4-0.7-3.5-1.1l-3-0.9c-0.2-0.1-0.4-0.1-0.7-0.2-1.1-0.3-2.3-0.7-3.4-1-1.2-0.3-2.4-0.6-3.5-0.9l-3.6-0.9-3.6-0.9c-1-0.3-2.1-0.5-3.1-0.7-1.2-0.3-2.4-0.5-3.6-0.8-1.3-0.3-2.5-0.6-3.8-0.8h-0.3c-0.9-0.2-1.9-0.4-2.8-0.6-0.4-0.1-0.7-0.1-1.1-0.2-1.1-0.2-2.2-0.4-3.4-0.6-1.2-0.2-2.4-0.4-3.6-0.7l-5.4-0.9c-0.9-0.1-1.9-0.3-2.8-0.4-0.8-0.1-1.6-0.3-2.5-0.4-2.6-0.4-5.1-0.7-7.7-1-1.2-0.1-2.3-0.3-3.5-0.4h-0.4c-0.9-0.1-1.8-0.2-2.8-0.3-1.1-0.1-2.1-0.2-3.2-0.3-1.7-0.2-3.4-0.3-5.1-0.4-0.8-0.1-1.5-0.1-2.3-0.2-0.9-0.1-1.9-0.1-2.8-0.2-0.4 0-0.8 0-1.2-0.1-1.1-0.1-2.1-0.1-3.2-0.2-0.5 0-1-0.1-1.5-0.1-1.3-0.1-2.6-0.1-3.9-0.1-0.8 0-1.5-0.1-2.3-0.1-1.2 0-2.4 0-3.5-0.1h-13.9c-2.3 0-4.6 0.1-6.9 0.2-0.9 0-1.9 0.1-2.8 0.1-0.8 0-1.5 0.1-2.3 0.1-1.4 0.1-2.8 0.2-4.1 0.3-1.4 0.1-2.7 0.2-4.1 0.3-1.4 0.1-2.7 0.2-4.1 0.4-0.6 0-1.2 0.1-1.8 0.2l-7.8 0.9c-1.1 0.1-2.1 0.3-3.2 0.4-1 0.1-2.1 0.3-3.1 0.4-3.2 0.5-6.4 0.9-9.5 1.5-0.7 0.1-1.4 0.2-2.1 0.4-0.9 0.1-1.7 0.3-2.6 0.5-1.1 0.2-2.3 0.4-3.4 0.6-0.9 0.2-1.7 0.3-2.6 0.5-0.4 0.1-0.8 0.1-1.1 0.2-0.7 0.1-1.4 0.3-2.1 0.4-1.2 0.3-2.4 0.5-3.6 0.8-1.2 0.3-2.4 0.5-3.6 0.8-0.2 0-0.4 0.1-0.6 0.1-0.5 0.1-1 0.2-1.5 0.4-1.1 0.3-2.3 0.6-3.5 0.9-1.3 0.3-2.5 0.6-3.8 1-0.4 0.1-0.9 0.2-1.4 0.4-1.3 0.4-2.7 0.7-4 1.1-1.5 0.4-3 0.9-4.6 1.3-1 0.3-2.1 0.6-3.1 1-2.1 0.6-4.1 1.3-6.2 2-0.7 0.2-1.4 0.5-2.1 0.7-15-27.5-27.4-56.4-37-86.2-11.7-36.1-19.2-73.6-22.5-111.6-0.6-6.7-1-13.3-1.3-20-0.1-1.2-0.1-2.4-0.1-3.6-0.1-1.2-0.1-2.4-0.1-3.6 0-1.2-0.1-2.4-0.1-3.6 0-1.2-0.1-2.4-0.1-3.7 18.8-14 39.2-25.8 61-35 36.1-15.3 74.5-23 114.1-23 39.6 0 78 7.8 114.1 23 21.8 9.2 42.2 20.9 61 35v0.1c0 1 0 1.9-0.1 2.9 0 1.4-0.1 2.8-0.1 4.3 0 0.7 0 1.3-0.1 2-0.1 1.8-0.1 3.5-0.2 5.3-0.3 6.7-0.8 13.3-1.3 20-3.3 38.5-11 76.5-23 113-9.7 30.3-22.3 59.4-37.6 87.1z m136.8 90.9a342.27 342.27 0 0 0-96.3-73.2c29.1-53.7 49.5-112.8 59.4-175.5 12.8 17.1 23.4 35.6 31.8 55.5 13.8 32.7 20.8 67.4 20.8 103.2 0 31-5.3 61.3-15.7 90z" fill="#39393A" /><path d="M512 819.3c8.7 0 24.7 22.9 24.7 60.4s-16 60.4-24.7 60.4-24.7-22.9-24.7-60.4 16-60.4 24.7-60.4m0-20c-24.7 0-44.7 36-44.7 80.4 0 44.4 20 80.4 44.7 80.4s44.7-36 44.7-80.4c0-44.4-20-80.4-44.7-80.4z" fill="#E73B37" /></svg>
        "##};

        let expected = indoc! {r##"
        svg {
            class: "icon",
            height: "800px",
            version: "1.1",
            view_box: "0 0 1024 1024",
            width: "800px",
            xmlns: "http://www.w3.org/2000/svg",
            path {
                d: "M512 301.2m-10 0a10 10 0 1 0 20 0 10 10 0 1 0-20 0Z",
                fill: "#E73B37",
            }
            path {
                d: "M400.3 744.5c2.1-0.7 4.1-1.4 6.2-2-2 0.6-4.1 1.3-6.2 2z m0 0c2.1-0.7 4.1-1.4 6.2-2-2 0.6-4.1 1.3-6.2 2z",
                fill: "#39393A",
            }
            path {
                d: "M511.8 256.6c24.4 0 44.2 19.8 44.2 44.2S536.2 345 511.8 345s-44.2-19.8-44.2-44.2 19.9-44.2 44.2-44.2m0-20c-35.5 0-64.2 28.7-64.2 64.2s28.7 64.2 64.2 64.2 64.2-28.7 64.2-64.2-28.7-64.2-64.2-64.2z",
                fill: "#E73B37",
            }
            path {
                d: "M730.7 529.5c0.4-8.7 0.6-17.4 0.6-26.2 0-179.6-86.1-339.1-219.3-439.5-133.1 100.4-219.2 259.9-219.2 439.5 0 8.8 0.2 17.5 0.6 26.1-56 56-90.6 133.3-90.6 218.7 0 61.7 18 119.1 49.1 167.3 30.3-49.8 74.7-90.1 127.7-115.3 39-18.6 82.7-29 128.8-29 48.3 0 93.9 11.4 134.3 31.7 52.5 26.3 96.3 67.7 125.6 118.4 33.4-49.4 52.9-108.9 52.9-173.1 0-85.4-34.6-162.6-90.5-218.6zM351.1 383.4c9.2-37.9 22.9-74.7 40.6-109.5a502.1 502.1 0 0 1 63.6-95.9c17.4-20.6 36.4-39.9 56.8-57.5 20.4 17.6 39.4 36.9 56.8 57.5 24.8 29.5 46.2 61.8 63.6 95.9 17.7 34.8 31.4 71.6 40.6 109.5 8.7 35.8 13.5 72.7 14.2 109.9C637.4 459 577 438.9 512 438.9c-65 0-125.3 20.1-175.1 54.4 0.7-37.2 5.5-74.1 14.2-109.9z m-90.6 449.2c-9.1-27-13.7-55.5-13.7-84.4 0-35.8 7-70.6 20.8-103.2 8.4-19.8 19-38.4 31.9-55.5 9.7 61.5 29.5 119.7 57.8 172.6-36.4 17.8-69 41.6-96.8 70.5z m364.2-85.3c-0.7-0.3-1.5-0.5-2.2-0.8-0.4-0.2-0.9-0.3-1.3-0.5-0.6-0.2-1.3-0.5-1.9-0.7-0.8-0.3-1.5-0.5-2.3-0.8-0.8-0.3-1.5-0.5-2.3-0.7l-0.9-0.3c-1-0.3-2.1-0.7-3.1-1-1.2-0.4-2.4-0.7-3.5-1.1l-3-0.9c-0.2-0.1-0.4-0.1-0.7-0.2-1.1-0.3-2.3-0.7-3.4-1-1.2-0.3-2.4-0.6-3.5-0.9l-3.6-0.9-3.6-0.9c-1-0.3-2.1-0.5-3.1-0.7-1.2-0.3-2.4-0.5-3.6-0.8-1.3-0.3-2.5-0.6-3.8-0.8h-0.3c-0.9-0.2-1.9-0.4-2.8-0.6-0.4-0.1-0.7-0.1-1.1-0.2-1.1-0.2-2.2-0.4-3.4-0.6-1.2-0.2-2.4-0.4-3.6-0.7l-5.4-0.9c-0.9-0.1-1.9-0.3-2.8-0.4-0.8-0.1-1.6-0.3-2.5-0.4-2.6-0.4-5.1-0.7-7.7-1-1.2-0.1-2.3-0.3-3.5-0.4h-0.4c-0.9-0.1-1.8-0.2-2.8-0.3-1.1-0.1-2.1-0.2-3.2-0.3-1.7-0.2-3.4-0.3-5.1-0.4-0.8-0.1-1.5-0.1-2.3-0.2-0.9-0.1-1.9-0.1-2.8-0.2-0.4 0-0.8 0-1.2-0.1-1.1-0.1-2.1-0.1-3.2-0.2-0.5 0-1-0.1-1.5-0.1-1.3-0.1-2.6-0.1-3.9-0.1-0.8 0-1.5-0.1-2.3-0.1-1.2 0-2.4 0-3.5-0.1h-13.9c-2.3 0-4.6 0.1-6.9 0.2-0.9 0-1.9 0.1-2.8 0.1-0.8 0-1.5 0.1-2.3 0.1-1.4 0.1-2.8 0.2-4.1 0.3-1.4 0.1-2.7 0.2-4.1 0.3-1.4 0.1-2.7 0.2-4.1 0.4-0.6 0-1.2 0.1-1.8 0.2l-7.8 0.9c-1.1 0.1-2.1 0.3-3.2 0.4-1 0.1-2.1 0.3-3.1 0.4-3.2 0.5-6.4 0.9-9.5 1.5-0.7 0.1-1.4 0.2-2.1 0.4-0.9 0.1-1.7 0.3-2.6 0.5-1.1 0.2-2.3 0.4-3.4 0.6-0.9 0.2-1.7 0.3-2.6 0.5-0.4 0.1-0.8 0.1-1.1 0.2-0.7 0.1-1.4 0.3-2.1 0.4-1.2 0.3-2.4 0.5-3.6 0.8-1.2 0.3-2.4 0.5-3.6 0.8-0.2 0-0.4 0.1-0.6 0.1-0.5 0.1-1 0.2-1.5 0.4-1.1 0.3-2.3 0.6-3.5 0.9-1.3 0.3-2.5 0.6-3.8 1-0.4 0.1-0.9 0.2-1.4 0.4-1.3 0.4-2.7 0.7-4 1.1-1.5 0.4-3 0.9-4.6 1.3-1 0.3-2.1 0.6-3.1 1-2.1 0.6-4.1 1.3-6.2 2-0.7 0.2-1.4 0.5-2.1 0.7-15-27.5-27.4-56.4-37-86.2-11.7-36.1-19.2-73.6-22.5-111.6-0.6-6.7-1-13.3-1.3-20-0.1-1.2-0.1-2.4-0.1-3.6-0.1-1.2-0.1-2.4-0.1-3.6 0-1.2-0.1-2.4-0.1-3.6 0-1.2-0.1-2.4-0.1-3.7 18.8-14 39.2-25.8 61-35 36.1-15.3 74.5-23 114.1-23 39.6 0 78 7.8 114.1 23 21.8 9.2 42.2 20.9 61 35v0.1c0 1 0 1.9-0.1 2.9 0 1.4-0.1 2.8-0.1 4.3 0 0.7 0 1.3-0.1 2-0.1 1.8-0.1 3.5-0.2 5.3-0.3 6.7-0.8 13.3-1.3 20-3.3 38.5-11 76.5-23 113-9.7 30.3-22.3 59.4-37.6 87.1z m136.8 90.9a342.27 342.27 0 0 0-96.3-73.2c29.1-53.7 49.5-112.8 59.4-175.5 12.8 17.1 23.4 35.6 31.8 55.5 13.8 32.7 20.8 67.4 20.8 103.2 0 31-5.3 61.3-15.7 90z",
                fill: "#39393A",
            }
            path {
                d: "M512 819.3c8.7 0 24.7 22.9 24.7 60.4s-16 60.4-24.7 60.4-24.7-22.9-24.7-60.4 16-60.4 24.7-60.4m0-20c-24.7 0-44.7 36-44.7 80.4 0 44.4 20 80.4 44.7 80.4s44.7-36 44.7-80.4c0-44.4-20-80.4-44.7-80.4z",
                fill: "#E73B37",
            }
        }
        "##};
        let actual = convert(input);
        assert_eq!(actual.expect("Failed to convert html"), expected);
    }
}

#[cfg(doctest)]
mod test_readme {
    macro_rules! external_doc_test {
        ($x:expr) => {
            #[doc = $x]
            extern "C" {}
        };
    }

    external_doc_test!(include_str!("../README.md"));
}
