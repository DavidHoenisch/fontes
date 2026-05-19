use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui_textarea::Input;

use fontes_core::Result;

use crate::app::{App, Mode, NoteEditFocus};

pub fn handle_key(app: &mut App, key: KeyEvent) -> Result<bool> {
    if key.kind != crossterm::event::KeyEventKind::Press {
        return Ok(false);
    }

    match app.mode {
        Mode::Reading => handle_reading(app, key),
        Mode::StrongPopup => handle_strong(app, key),
        Mode::NoteEditor => handle_note_editor(app, key),
        Mode::Search => handle_search(app, key),
        Mode::BookPicker => handle_book_picker(app, key),
        Mode::ChapterPicker => handle_chapter_picker(app, key),
        Mode::Goto => handle_goto(app, key),
        Mode::NotesList => handle_notes_list(app, key),
        Mode::Help => handle_help(app, key),
    }
}

fn handle_reading(app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Char('b') => app.open_book_picker()?,
        KeyCode::Char('c') => app.open_chapter_picker()?,
        KeyCode::Char('g') => app.open_goto(),
        KeyCode::Char('?') => app.mode = Mode::Help,
        KeyCode::Char('N') => app.open_notes_list()?,
        KeyCode::Char('s') => app.open_strong_popup()?,
        KeyCode::Char('n') => app.open_note_at_cursor()?,
        KeyCode::Char('D') => app.delete_note_at_cursor()?,
        KeyCode::Char('e') => {
            if let Some(id) = crate::overlay::note_id_at_token(
                &app.note_anchors,
                app.cursor_token_idx(),
            ) {
                app.open_note_editor_existing(id)?;
            } else {
                app.status = "No note here — press n to create.".into();
            }
        }
        KeyCode::Char('v') => app.set_selection_anchor(),
        KeyCode::Char('V') => app.set_verse_anchor(),
        KeyCode::Char('y') => {
            if let Err(e) = app.copy_verses_to_clipboard() {
                app.status = e.to_string();
            }
        }
        KeyCode::Char('H') => app.toggle_highlight()?,
        KeyCode::Char('u') => app.toggle_underline()?,
        KeyCode::Esc => app.clear_selection_anchor(),
        KeyCode::Char('/') => {
            app.mode = Mode::Search;
            app.reset_search_session();
        }
        KeyCode::Left | KeyCode::Char('h') => app.move_token(-1),
        KeyCode::Right | KeyCode::Char('l') => app.move_token(1),
        KeyCode::Up | KeyCode::Char('k') => app.move_verse(-1),
        KeyCode::Down | KeyCode::Char('j') => app.move_verse(1),
        KeyCode::PageUp => app.move_verse(-5),
        KeyCode::PageDown => app.move_verse(5),
        KeyCode::Char('[') => app.change_chapter(-1)?,
        KeyCode::Char(']') => app.change_chapter(1)?,
        KeyCode::Home => {
            app.verse_index = 0;
            app.token_index = 0;
            app.clamp_cursor();
        }
        KeyCode::End => {
            app.verse_index = app.chapter.verses.len().saturating_sub(1);
            app.token_index = 0;
            app.clamp_cursor();
        }
        _ => {}
    }
    Ok(false)
}

fn handle_strong(app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Esc => app.close_popup(),
        KeyCode::Enter => app.jump_to_selected_occurrence()?,
        KeyCode::Up => {
            if !app.strong_occurrences.is_empty() {
                let i = app.strong_occ_list_state.selected().unwrap_or(0);
                if i > 0 {
                    app.strong_occ_list_state.select(Some(i - 1));
                }
            }
        }
        KeyCode::Down => {
            if !app.strong_occurrences.is_empty() {
                let i = app.strong_occ_list_state.selected().unwrap_or(0);
                if i + 1 < app.strong_occurrences.len() {
                    app.strong_occ_list_state.select(Some(i + 1));
                }
            }
        }
        _ => {}
    }
    Ok(false)
}

fn handle_note_editor(app: &mut App, key: KeyEvent) -> Result<bool> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
        app.save_note()?;
        return Ok(false);
    }
    if key.code == KeyCode::Tab {
        app.note_focus = match app.note_focus {
            NoteEditFocus::Title => NoteEditFocus::Body,
            NoteEditFocus::Body => NoteEditFocus::Title,
        };
        return Ok(false);
    }
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Reading;
            app.status = "Note edit cancelled.".into();
        }
        _ if app.note_focus == NoteEditFocus::Body => {
            app.note_body.input(Input::from(key));
        }
        KeyCode::Backspace => {
            app.edit_title.pop();
        }
        KeyCode::Char(c) => {
            app.edit_title.push(c);
        }
        _ => {}
    }
    Ok(false)
}

fn handle_search(app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Reading;
            app.status = "Search cancelled.".into();
        }
        KeyCode::Enter => {
            if app.search_results.is_empty() {
                app.run_search()?;
                app.status = format!("{} hits", app.search_results.len());
            } else {
                app.jump_to_search_hit()?;
            }
        }
        KeyCode::Up => {
            if !app.search_results.is_empty() {
                let i = app.search_list_state.selected().unwrap_or(0);
                if i > 0 {
                    app.search_list_state.select(Some(i - 1));
                }
            }
        }
        KeyCode::Down => {
            if !app.search_results.is_empty() {
                let i = app.search_list_state.selected().unwrap_or(0);
                if i + 1 < app.search_results.len() {
                    app.search_list_state.select(Some(i + 1));
                }
            }
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            app.touch_search_query();
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.touch_search_query();
        }
        _ => {}
    }
    Ok(false)
}

fn handle_book_picker(app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Esc => {
            if !app.book_filter.is_empty() {
                app.book_filter.clear();
                app.clamp_book_list_selection();
                app.status = "Book search cleared.".into();
            } else if app.book_search_active {
                app.book_search_active = false;
                app.status = "Book search cancelled.".into();
            } else {
                app.mode = Mode::Reading;
                app.status = "Book picker cancelled.".into();
            }
        }
        KeyCode::Enter => {
            let sel = app.book_list_state.selected().unwrap_or(0);
            app.select_book(sel)?;
        }
        KeyCode::Up => {
            let i = app.book_list_state.selected().unwrap_or(0);
            if i > 0 {
                app.book_list_state.select(Some(i - 1));
            }
        }
        KeyCode::Down => {
            let i = app.book_list_state.selected().unwrap_or(0);
            let count = app.filtered_book_indices().len();
            if i + 1 < count {
                app.book_list_state.select(Some(i + 1));
            }
        }
        KeyCode::Char('/') => {
            app.book_search_active = true;
            app.book_filter.clear();
            app.clamp_book_list_selection();
        }
        KeyCode::Backspace => {
            if app.book_search_active {
                app.book_filter.pop();
                app.clamp_book_list_selection();
            }
        }
        KeyCode::Char(c) => {
            if app.book_search_active {
                app.book_filter.push(c);
                app.clamp_book_list_selection();
            }
        }
        _ => {}
    }
    Ok(false)
}

fn handle_chapter_picker(app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Esc => {
            if !app.chapter_filter.is_empty() {
                app.chapter_filter.clear();
                app.clamp_chapter_list_selection();
                app.status = "Chapter search cleared.".into();
            } else if app.chapter_search_active {
                app.chapter_search_active = false;
                app.status = "Chapter search cancelled.".into();
            } else {
                app.mode = Mode::Reading;
                app.status = "Chapter picker cancelled.".into();
            }
        }
        KeyCode::Enter => {
            let sel = app.chapter_list_state.selected().unwrap_or(0);
            app.select_chapter_from_picker(sel)?;
        }
        KeyCode::Up => {
            let i = app.chapter_list_state.selected().unwrap_or(0);
            if i > 0 {
                app.chapter_list_state.select(Some(i - 1));
            }
        }
        KeyCode::Down => {
            let i = app.chapter_list_state.selected().unwrap_or(0);
            let count = app.filtered_chapters().len();
            if i + 1 < count {
                app.chapter_list_state.select(Some(i + 1));
            }
        }
        KeyCode::Char('/') => {
            app.chapter_search_active = true;
            app.chapter_filter.clear();
            app.clamp_chapter_list_selection();
        }
        KeyCode::Backspace => {
            if app.chapter_search_active {
                app.chapter_filter.pop();
                app.clamp_chapter_list_selection();
            }
        }
        KeyCode::Char(c) => {
            if app.chapter_search_active {
                app.chapter_filter.push(c);
                app.clamp_chapter_list_selection();
            }
        }
        _ => {}
    }
    Ok(false)
}

fn handle_notes_list(app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Esc => {
            if !app.notes_filter.is_empty() {
                app.notes_filter.clear();
                app.clamp_notes_list_selection();
                app.status = "Notes search cleared.".into();
            } else if app.notes_search_active {
                app.notes_search_active = false;
                app.status = "Notes search cancelled.".into();
            } else {
                app.mode = Mode::Reading;
                app.status = "Notes list closed.".into();
            }
        }
        KeyCode::Enter => app.open_selected_note()?,
        KeyCode::Up => {
            let i = app.notes_list_state.selected().unwrap_or(0);
            if i > 0 {
                app.notes_list_state.select(Some(i - 1));
            }
        }
        KeyCode::Down => {
            let i = app.notes_list_state.selected().unwrap_or(0);
            let count = app.filtered_note_indices().len();
            if i + 1 < count {
                app.notes_list_state.select(Some(i + 1));
            }
        }
        KeyCode::Char('/') => {
            app.notes_search_active = true;
            app.notes_filter.clear();
            app.clamp_notes_list_selection();
        }
        KeyCode::Backspace => {
            if app.notes_search_active {
                app.notes_filter.pop();
                app.clamp_notes_list_selection();
            }
        }
        KeyCode::Char(c) => {
            if app.notes_search_active {
                app.notes_filter.push(c);
                app.clamp_notes_list_selection();
            }
        }
        _ => {}
    }
    Ok(false)
}

fn handle_help(app: &mut App, key: KeyEvent) -> Result<bool> {
    if key.code == KeyCode::Esc {
        app.mode = Mode::Reading;
    }
    Ok(false)
}

fn handle_goto(app: &mut App, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Reading;
            app.status = "Go-to cancelled.".into();
        }
        KeyCode::Enter => app.submit_goto()?,
        KeyCode::Backspace => {
            app.goto_input.pop();
        }
        KeyCode::Char(c) => {
            app.goto_input.push(c);
        }
        _ => {}
    }
    Ok(false)
}
