use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, Wrap,
};
use ratatui::Frame;

use fontes_core::AnnotationKind;

use crate::app::{App, Mode, NoteEditFocus, SEARCH_MIN_CHARS};
use crate::list_scroll::{ensure_list_visible, inner_height};
use crate::markdown::to_plain;
use crate::overlay::{
    token_annotation_kind, token_has_note, token_in_pending_selection, verse_in_copy_range,
};
use crate::scroll::{ensure_verse_visible, verse_layout};
use crate::search_highlight::{highlight_filter_line, highlight_line};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(frame.area());

    draw_header(frame, app, root[0]);
    draw_body(frame, app, root[1]);
    draw_footer(frame, app, root[2]);

    match app.mode {
        Mode::StrongPopup => draw_strong_popup(frame, app),
        Mode::NoteEditor => draw_note_editor(frame, app),
        Mode::Search => draw_search_overlay(frame, app),
        Mode::BookPicker => draw_book_picker(frame, app),
        Mode::ChapterPicker => draw_chapter_picker(frame, app),
        Mode::Goto => draw_goto_bar(frame, app),
        Mode::NotesList => draw_notes_list(frame, app),
        Mode::Help => draw_help(frame),
        Mode::Reading => {}
    }
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    if app.chapter.verses.is_empty() {
        return;
    }
    let token = &app.current_verse().tokens[app.token_index];
    let strong = token.strong_key.as_deref().unwrap_or("—");
    let title = format!(
        " {} {}:{}  │  word: {}  │  Strong: {} ",
        app.chapter.book.name,
        app.chapter.chapter,
        app.current_verse().reference.verse,
        token.surface,
        strong,
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" fontes ");
    frame.render_widget(Paragraph::new(title).block(block), area);
}

/// Horizontal inset and max line length for the chapter column (readability).
const READING_H_MARGIN: u16 = 3;
const READING_MAX_WIDTH: u16 = 84;
const READING_V_PAD: u16 = 1;

fn reading_text_rect(inner: Rect) -> Rect {
    let width = inner
        .width
        .saturating_sub(READING_H_MARGIN * 2)
        .clamp(32, READING_MAX_WIDTH);
    let x = inner.x + inner.width.saturating_sub(width) / 2;
    Rect {
        x,
        y: inner.y + READING_V_PAD,
        width,
        height: inner.height.saturating_sub(READING_V_PAD * 2),
    }
}

fn draw_body(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(format!(
            " {} {} ",
            app.chapter.book.abbrev, app.chapter.chapter
        ))
        .title_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let text_area = reading_text_rect(inner);

    let mut lines: Vec<Line> = Vec::new();
    for (vi, verse) in app.chapter.verses.iter().enumerate() {
        let verse_ann: Vec<fontes_core::Annotation> = app
            .annotations
            .iter()
            .filter(|a| a.verse_id == verse.id)
            .cloned()
            .collect();
        let verse_anchors: Vec<fontes_core::NoteAnchor> = app
            .note_anchors
            .iter()
            .filter(|a| a.verse_id == Some(verse.id))
            .cloned()
            .collect();

        let mut spans = vec![Span::styled(
            format!("{:>3} ", verse.reference.verse),
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::DIM),
        )];

        let in_verse_copy_range = verse_in_copy_range(app.verse_anchor, app.verse_index, vi);

        for (ti, token) in verse.tokens.iter().enumerate() {
            let is_cursor = vi == app.verse_index && ti == app.token_index;
            let mut style = Style::default().fg(Color::White);
            if in_verse_copy_range && !is_cursor {
                style = style.bg(Color::Rgb(40, 40, 55));
            }
            if let Some(kind) = token_annotation_kind(&verse_ann, token.idx) {
                style = match kind {
                    AnnotationKind::Highlight => style.bg(Color::Yellow).fg(Color::Black),
                    AnnotationKind::Underline => style.underlined(),
                };
            }
            if token_has_note(&verse_anchors, token.idx) {
                style = style.fg(Color::Magenta);
            }
            if token_in_pending_selection(
                app.selection_anchor,
                vi,
                app.verse_index,
                app.token_index,
                ti,
            ) && !is_cursor
            {
                style = style.bg(Color::DarkGray);
            }
            if is_cursor {
                style = style
                    .add_modifier(Modifier::REVERSED)
                    .add_modifier(Modifier::BOLD);
            }
            spans.push(Span::styled(format!("{} ", token.surface), style));
        }
        lines.push(Line::from(spans));
    }

    let mut display_lines: Vec<Line> = Vec::with_capacity(lines.len() * 2);
    for (i, line) in lines.iter().enumerate() {
        display_lines.push(line.clone());
        if i + 1 < lines.len() {
            display_lines.push(Line::from(""));
        }
    }

    let width = text_area.width.max(1);
    let viewport = text_area.height.max(1) as usize;
    let (line_starts, line_heights) = verse_layout(&lines, width, true);
    app.scroll_top = ensure_verse_visible(
        &line_starts,
        &line_heights,
        app.verse_index,
        app.scroll_top,
        viewport,
    );

    let paragraph = Paragraph::new(display_lines)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::White));
    frame.render_widget(paragraph.scroll((app.scroll_top as u16, 0)), text_area);

    let mut scrollbar_state =
        ratatui::widgets::ScrollbarState::new(app.chapter.verses.len()).position(app.verse_index);
    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .thumb_symbol("█")
            .track_symbol(Some("│")),
        inner,
        &mut scrollbar_state,
    );
}

fn truncate_to_width(text: &str, width: u16) -> String {
    let max = width.max(1) as usize;
    let char_count = text.chars().count();
    if char_count <= max {
        return text.to_string();
    }
    if max == 1 {
        return "…".to_string();
    }
    let mut s: String = text.chars().take(max - 1).collect();
    s.push('…');
    s
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    let help = match app.mode {
        Mode::Reading => {
            "y copy · V verse range · v word range · H highlight · u underline · b book · s Strong · n note · / search · ? help · q quit"
        }
        Mode::StrongPopup => "Esc close │ Enter jump │ ↑/↓ occurrence",
        Mode::NoteEditor => "Tab field │ Ctrl+S save │ Esc cancel",
        Mode::Search => "type to search │ Enter jump │ ↑/↓ │ Esc cancel",
        Mode::BookPicker => "/ filter │ Enter open │ ↑/↓ │ Esc cancel",
        Mode::ChapterPicker => "/ filter │ Enter open │ ↑/↓ │ Esc cancel",
        Mode::Goto => "Enter go │ Esc cancel",
        Mode::NotesList => "/ filter │ Enter edit │ ↑/↓ │ Esc cancel",
        Mode::Help => "Esc close",
    };
    if !app.status.is_empty() {
        let status = truncate_to_width(app.status.as_str(), chunks[0].width);
        frame.render_widget(
            Paragraph::new(status).style(Style::default().fg(Color::Cyan)),
            chunks[0],
        );
    }
    frame.render_widget(
        Paragraph::new(help).style(Style::default().fg(Color::DarkGray)),
        chunks[1],
    );
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn draw_strong_popup(frame: &mut Frame, app: &mut App) {
    let area = centered_rect(70, 75, frame.area());
    frame.render_widget(Clear, area);

    let Some(entry) = &app.strong_entry else {
        return;
    };

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Min(4)])
        .split(area);

    let mut lines = vec![Line::from(vec![
        Span::styled(
            format!("{} ", entry.key),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled(&entry.lang, Style::default().fg(Color::Cyan)),
    ])];
    if let Some(l) = &entry.lemma {
        lines.push(Line::from(format!("lemma: {l}")));
    }
    if let Some(t) = &entry.translit {
        lines.push(Line::from(format!("translit: {t}")));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(entry.definition.as_str()));
    if let Some(g) = &entry.kjv_gloss {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("KJV: {g}"),
            Style::default().fg(Color::DarkGray),
        )));
    }

    let def_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(format!(" Strong's {} ", entry.key));
    frame.render_widget(
        Paragraph::new(lines)
            .block(def_block)
            .wrap(Wrap { trim: true }),
        layout[0],
    );

    let occ_items: Vec<ListItem> = app
        .strong_occurrences
        .iter()
        .map(|occ| {
            ListItem::new(format!(
                "{} {}:{} token {}",
                occ.book_abbrev, occ.chapter, occ.verse, occ.token_idx
            ))
        })
        .collect();
    let occ_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(format!(
            " Occurrences ({}) — ↑/↓ Enter jump ",
            app.strong_occ_total
        ));
    ensure_list_visible(
        &mut app.strong_occ_list_state,
        occ_items.len(),
        inner_height(layout[1], &occ_block),
    );
    let occ_list = List::new(occ_items)
        .block(occ_block)
        .scroll_padding(1)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(occ_list, layout[1], &mut app.strong_occ_list_state);
}

fn draw_note_editor(frame: &mut Frame, app: &mut App) {
    let area = centered_rect(85, 85, frame.area());
    frame.render_widget(Clear, area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(8),
        ])
        .split(area);

    let title_style = if app.note_focus == NoteEditFocus::Title {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default()
    };
    let title_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta))
        .title(" Title (Tab to switch) ");
    frame.render_widget(
        Paragraph::new(app.edit_title.as_str())
            .style(title_style)
            .block(title_block),
        layout[0],
    );

    let body_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta))
        .title(" Body — Markdown (Ctrl+S save) ");
    app.note_body.set_block(body_block);
    frame.render_widget(&app.note_body, layout[1]);

    let preview = to_plain(&app.note_body.lines().join("\n"));
    let preview_block = Block::default().borders(Borders::ALL).title(" Preview ");
    frame.render_widget(
        Paragraph::new(preview)
            .block(preview_block)
            .wrap(Wrap { trim: true }),
        layout[2],
    );
}

fn draw_search_overlay(frame: &mut Frame, app: &mut App) {
    let area = centered_rect(80, 60, frame.area());
    frame.render_widget(Clear, area);

    let query = app.search_query.as_str();
    let items: Vec<ListItem> = if app.search_results.is_empty() {
        let hint = if query.trim().len() < SEARCH_MIN_CHARS {
            "Type 2+ characters to search…"
        } else if query.trim() == app.search_ran_query() {
            "No results"
        } else {
            "Searching…"
        };
        vec![ListItem::new(hint)]
    } else {
        app.search_results
            .iter()
            .map(|hit| {
                let prefix = format!("{} {}:{} — ", hit.book_abbrev, hit.chapter, hit.verse);
                let mut line = highlight_line(&hit.snippet, query);
                if !prefix.is_empty() {
                    line.spans.insert(0, Span::raw(prefix));
                }
                ListItem::new(line)
            })
            .collect()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .title(format!(" Search: {} ", app.search_query));
    ensure_list_visible(
        &mut app.search_list_state,
        items.len(),
        inner_height(area, &block),
    );
    let list = List::new(items)
        .block(block)
        .scroll_padding(1)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(list, area, &mut app.search_list_state);
}

fn draw_book_picker(frame: &mut Frame, app: &mut App) {
    let area = centered_rect(50, 70, frame.area());
    frame.render_widget(Clear, area);

    let filtered = app.filtered_book_indices();
    let items: Vec<ListItem> = if filtered.is_empty() {
        vec![ListItem::new("No matching books")]
    } else {
        filtered
            .iter()
            .map(|&index| {
                let b = &app.books_available[index];
                let row = format!(
                    "{:>3} {:<4} {} ({})",
                    b.sort_order, b.abbrev, b.name, b.testament
                );
                let line = if app.book_search_active && !app.book_filter.is_empty() {
                    highlight_filter_line(&row, &app.book_filter)
                } else {
                    Line::from(row)
                };
                ListItem::new(line)
            })
            .collect()
    };

    let title = if app.book_search_active {
        format!(" Search: {} ", app.book_filter)
    } else {
        " Select book (/ filter) ".to_string()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title);
    ensure_list_visible(
        &mut app.book_list_state,
        items.len(),
        inner_height(area, &block),
    );
    let list = List::new(items)
        .block(block)
        .scroll_padding(1)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(list, area, &mut app.book_list_state);
}

fn draw_goto_bar(frame: &mut Frame, app: &App) {
    let area = Rect {
        x: frame.area().x + 2,
        y: frame.area().y + 2,
        width: frame.area().width.saturating_sub(4),
        height: 3,
    };
    frame.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Go to (chapter:verse) ");
    frame.render_widget(
        Paragraph::new(app.goto_input.as_str())
            .block(block)
            .style(Style::default().add_modifier(Modifier::REVERSED)),
        area,
    );
}

fn draw_chapter_picker(frame: &mut Frame, app: &mut App) {
    let area = centered_rect(40, 60, frame.area());
    frame.render_widget(Clear, area);

    let chapters = app.filtered_chapters();
    let items: Vec<ListItem> = if chapters.is_empty() {
        vec![ListItem::new("No matching chapters")]
    } else {
        chapters
            .iter()
            .map(|ch| {
                let label = format!("Chapter {ch}");
                let line = if app.chapter_search_active && !app.chapter_filter.is_empty() {
                    highlight_filter_line(&label, &app.chapter_filter)
                } else {
                    Line::from(label)
                };
                ListItem::new(line)
            })
            .collect()
    };

    let title = if app.chapter_search_active {
        format!(" Search: {} ", app.chapter_filter)
    } else {
        format!(" {} — chapter (/ filter) ", app.chapter.book.name)
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title);
    ensure_list_visible(
        &mut app.chapter_list_state,
        items.len(),
        inner_height(area, &block),
    );
    let list = List::new(items)
        .block(block)
        .scroll_padding(1)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(list, area, &mut app.chapter_list_state);
}

fn draw_notes_list(frame: &mut Frame, app: &mut App) {
    let area = centered_rect(70, 70, frame.area());
    frame.render_widget(Clear, area);

    let filtered = app.filtered_note_indices();
    let items: Vec<ListItem> = if app.all_notes.is_empty() {
        vec![ListItem::new(
            "No notes yet — press n on a word to create one",
        )]
    } else if filtered.is_empty() {
        vec![ListItem::new("No matching notes")]
    } else {
        filtered
            .iter()
            .map(|&index| {
                let entry = &app.all_notes[index];
                let title = entry
                    .note
                    .title
                    .as_deref()
                    .filter(|t| !t.is_empty())
                    .unwrap_or("(untitled)");
                let preview = to_plain(&entry.note.body);
                let preview_line = preview.lines().next().unwrap_or("");
                let location = Span::styled(
                    format!("{}  ", entry.location),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                );
                let rest = format!("#{id} {title} — {preview_line}", id = entry.note.id);
                let mut spans = vec![location];
                if app.notes_search_active && !app.notes_filter.is_empty() {
                    spans.extend(highlight_filter_line(&rest, &app.notes_filter).spans);
                } else {
                    spans.push(Span::raw(rest));
                }
                ListItem::new(Line::from(spans))
            })
            .collect()
    };

    let title = if app.notes_search_active {
        format!(" Search: {} ", app.notes_filter)
    } else {
        " Notes (/ filter) ".to_string()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta))
        .title(title);
    ensure_list_visible(
        &mut app.notes_list_state,
        items.len(),
        inner_height(area, &block),
    );
    let list = List::new(items)
        .block(block)
        .scroll_padding(1)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(list, area, &mut app.notes_list_state);
}

fn draw_help(frame: &mut Frame) {
    let area = centered_rect(70, 75, frame.area());
    frame.render_widget(Clear, area);

    let text = "\
Navigation
  ←/→ or h/l     previous / next word
  ↑/↓ or j/k     previous / next verse
  PgUp/PgDn      jump 5 verses
  [ / ]          previous / next chapter
  Home / End     first / last verse

Places
  b              pick book (/ to search in list)
  c              pick chapter (/ to search in list)
  g              go to chapter:verse

Study
  s              Strong's dictionary (Enter jump occurrence)
  n / e          new / edit note on word
  N              all notes (/ to search in list)
  D              delete note on word
  y              copy verse(s) with reference (KJV text)
  V              verse anchor — j/k to end of range, then y
  v              word anchor — h/l, then H highlight or u underline
  /              search scripture

Other
  ?              this help
  q              quit";

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Help ");
    frame.render_widget(
        Paragraph::new(text).block(block).wrap(Wrap { trim: true }),
        area,
    );
}
