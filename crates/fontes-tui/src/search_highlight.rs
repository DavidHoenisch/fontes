//! Highlight search query terms in result lines.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use fontes_core::search_terms;

const HIGHLIGHT: Style = Style::new()
    .fg(Color::Black)
    .bg(Color::Yellow)
    .add_modifier(Modifier::BOLD);

/// Highlight filter text in list rows (supports single-character matches).
pub fn highlight_filter_line(text: &str, filter: &str) -> Line<'static> {
    let terms = filter_terms(filter);
    highlight_terms_in_line(text, &terms)
}

pub fn highlight_line(text: &str, query: &str) -> Line<'static> {
    let terms = search_terms(query);
    highlight_terms_in_line(text, &terms)
}

fn filter_terms(filter: &str) -> Vec<String> {
    let trimmed = filter.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let parts: Vec<String> = filter
        .split_whitespace()
        .map(|t| t.to_lowercase())
        .filter(|t| !t.is_empty())
        .collect();
    if parts.is_empty() {
        vec![trimmed.to_lowercase()]
    } else {
        parts
    }
}

fn highlight_terms_in_line(text: &str, terms: &[String]) -> Line<'static> {
    if terms.is_empty() {
        return Line::from(text.to_string());
    }
    let lower = text.to_lowercase();
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    for term in terms {
        let mut start = 0;
        while let Some(rel) = lower[start..].find(term.as_str()) {
            let from = start + rel;
            let to = from + term.len();
            ranges.push((from, to));
            start = to.max(start + 1);
        }
    }

    if ranges.is_empty() {
        return Line::from(text.to_string());
    }

    ranges.sort_by_key(|(from, _)| *from);
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (from, to) in ranges {
        if let Some(last) = merged.last_mut() {
            if from <= last.1 {
                last.1 = last.1.max(to);
                continue;
            }
        }
        merged.push((from, to));
    }

    let mut spans = Vec::new();
    let mut cursor = 0;
    for (from, to) in merged {
        if from > cursor {
            spans.push(Span::raw(text[cursor..from].to_string()));
        }
        spans.push(Span::styled(text[from..to].to_string(), HIGHLIGHT));
        cursor = to;
    }
    if cursor < text.len() {
        spans.push(Span::raw(text[cursor..].to_string()));
    }
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlights_matching_substring() {
        let line = highlight_line("In the beginning", "begin");
        assert_eq!(line.spans.len(), 3);
        assert_eq!(line.spans[1].content, "begin");
        assert_eq!(line.spans[1].style, HIGHLIGHT);
    }

    #[test]
    fn filter_highlight_single_character() {
        let line = highlight_filter_line("Chapter 13", "3");
        assert!(line.spans.iter().any(|s| s.style == HIGHLIGHT));
    }
}
