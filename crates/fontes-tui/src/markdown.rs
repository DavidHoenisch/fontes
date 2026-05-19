//! Minimal markdown → plain text for note preview in the terminal.

pub fn to_plain(markdown: &str) -> String {
    use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

    let mut out = String::new();
    let parser = Parser::new_ext(markdown, Options::empty());
    for event in parser {
        match event {
            Event::Text(t) | Event::Code(t) => out.push_str(&t),
            Event::SoftBreak | Event::HardBreak => out.push('\n'),
            Event::Start(Tag::Paragraph) | Event::End(TagEnd::Paragraph) => {
                if !out.is_empty() && !out.ends_with('\n') {
                    out.push('\n');
                }
            }
            Event::Start(Tag::Heading { .. }) => {
                if !out.is_empty() && !out.ends_with('\n') {
                    out.push('\n');
                }
            }
            Event::End(TagEnd::Heading(_)) => out.push('\n'),
            _ => {}
        }
    }
    out.trim().to_string()
}
