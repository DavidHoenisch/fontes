use std::path::PathBuf;
use std::time::{Duration, Instant};

use fontes_core::{
    format_verses_clipboard, Annotation, AnnotationKind, Book, Chapter, Database,
    KJV_TRANSLATION_ID, NoteAnchor, NoteListEntry, Result, SearchHit, StrongEntry,
    StrongOccurrence, Verse, verse_ref_from_id,
};
use ratatui::widgets::ListState;
use tui_textarea::TextArea;

use crate::clipboard::ClipboardStore;
use crate::overlay::note_id_at_token;

/// Delay before running a live search after the query changes.
pub const SEARCH_DEBOUNCE: Duration = Duration::from_millis(300);

/// Minimum query length (characters) before auto-search runs.
pub const SEARCH_MIN_CHARS: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Reading,
    StrongPopup,
    NoteEditor,
    Search,
    BookPicker,
    ChapterPicker,
    Goto,
    NotesList,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteEditFocus {
    Title,
    Body,
}

pub struct App {
    pub db: Database,
    pub mode: Mode,
    pub book_abbrev: String,
    pub chapter_num: i32,
    pub chapter: Chapter,
    pub verse_index: usize,
    pub token_index: usize,
    /// Multi-word mark anchor: (verse index in chapter, token index in that verse).
    pub selection_anchor: Option<(usize, usize)>,
    /// Verse-range anchor for copy (index in `chapter.verses`).
    pub verse_anchor: Option<usize>,
    /// Top visible row in the chapter view (in wrapped display lines).
    pub scroll_top: usize,
    pub verse_list_state: ListState,
    pub annotations: Vec<Annotation>,
    pub note_anchors: Vec<NoteAnchor>,
    pub status: String,
    // Strong popup
    pub strong_entry: Option<StrongEntry>,
    pub strong_occurrences: Vec<StrongOccurrence>,
    pub strong_occ_total: usize,
    pub strong_occ_list_state: ListState,
    // Note editor
    pub editing_note_id: Option<i64>,
    pub edit_title: String,
    pub edit_body: String,
    pub note_focus: NoteEditFocus,
    pub note_body: TextArea<'static>,
    // Search
    pub search_query: String,
    pub search_results: Vec<SearchHit>,
    pub search_list_state: ListState,
    /// Last query string passed to the database (for debounced refresh).
    search_ran_query: String,
    search_last_edit: Option<Instant>,
    // Book picker
    pub books_available: Vec<Book>,
    pub book_filter: String,
    pub book_search_active: bool,
    pub book_list_state: ListState,
    // Goto
    pub goto_input: String,
    // Chapter picker
    pub max_chapter: i32,
    pub chapter_filter: String,
    pub chapter_search_active: bool,
    pub chapter_list_state: ListState,
    // Notes list
    pub all_notes: Vec<NoteListEntry>,
    pub notes_filter: String,
    pub notes_search_active: bool,
    pub notes_list_state: ListState,
    clipboard: ClipboardStore,
}

impl App {
    pub fn open(data_dir: PathBuf, book_abbrev: &str, chapter: i32, resume: bool) -> Result<Self> {
        let db = Database::open_data_dir(&data_dir)?;
        let (book_abbrev, chapter_num) = if resume {
            if let Ok(Some(state)) = db.get_reading_state() {
                if let Ok(book) = db.book_by_id(state.book_id) {
                    (book.abbrev, state.chapter)
                } else {
                    (book_abbrev.to_string(), chapter)
                }
            } else {
                (book_abbrev.to_string(), chapter)
            }
        } else {
            (book_abbrev.to_string(), chapter)
        };
        let mut app = Self {
            db,
            mode: Mode::Reading,
            book_abbrev,
            chapter_num,
            chapter: empty_chapter(chapter),
            verse_index: 0,
            token_index: 0,
            selection_anchor: None,
            verse_anchor: None,
            scroll_top: 0,
            verse_list_state: ListState::default(),
            annotations: Vec::new(),
            note_anchors: Vec::new(),
            status: String::new(),
            strong_entry: None,
            strong_occurrences: Vec::new(),
            strong_occ_total: 0,
            strong_occ_list_state: ListState::default(),
            editing_note_id: None,
            edit_title: String::new(),
            edit_body: String::new(),
            note_focus: NoteEditFocus::Body,
            note_body: TextArea::default(),
            search_query: String::new(),
            search_results: Vec::new(),
            search_list_state: ListState::default(),
            search_ran_query: String::new(),
            search_last_edit: None,
            books_available: Vec::new(),
            book_filter: String::new(),
            book_search_active: false,
            book_list_state: ListState::default(),
            goto_input: String::new(),
            max_chapter: 1,
            chapter_filter: String::new(),
            chapter_search_active: false,
            chapter_list_state: ListState::default(),
            all_notes: Vec::new(),
            notes_filter: String::new(),
            notes_search_active: false,
            notes_list_state: ListState::default(),
            clipboard: ClipboardStore::new()?,
        };
        app.reload_chapter()?;
        if resume {
            app.restore_reading_state();
        }
        Ok(app)
    }

    pub fn current_verse(&self) -> &Verse {
        &self.chapter.verses[self.verse_index]
    }

    pub fn cursor_verse_id(&self) -> i64 {
        self.current_verse().id
    }

    pub fn cursor_token_idx(&self) -> i32 {
        self.current_verse().tokens[self.token_index].idx
    }

    /// Token range `[start, end)` for annotations, using the selection anchor when set in this verse.
    pub fn annotation_token_range(&self) -> (i32, i32) {
        let verse = self.current_verse();
        if verse.tokens.is_empty() {
            return (0, 0);
        }
        if let Some((avi, ati)) = self.selection_anchor {
            if avi == self.verse_index {
                let lo = ati.min(self.token_index);
                let hi = ati.max(self.token_index);
                let start = verse.tokens[lo].idx;
                let end = verse.tokens[hi].idx + 1;
                return (start, end);
            }
        }
        let idx = self.cursor_token_idx();
        (idx, idx + 1)
    }

    pub fn set_selection_anchor(&mut self) {
        if self.chapter.verses.is_empty() {
            return;
        }
        self.selection_anchor = Some((self.verse_index, self.token_index));
        self.status =
            "Selection set — move with h/l, then H (highlight) or u (underline). Esc clears.".into();
    }

    pub fn clear_selection_anchor(&mut self) {
        let had_word = self.selection_anchor.take().is_some();
        let had_verse = self.verse_anchor.take().is_some();
        if had_word || had_verse {
            self.status = "Selection cleared.".into();
        }
    }

    pub fn set_verse_anchor(&mut self) {
        if self.chapter.verses.is_empty() {
            return;
        }
        self.verse_anchor = Some(self.verse_index);
        self.status =
            "Verse anchor set — move with j/k, then y to copy. Esc clears.".into();
    }

    pub fn verse_copy_indices(&self) -> (usize, usize) {
        if let Some(anchor) = self.verse_anchor {
            let lo = anchor.min(self.verse_index);
            let hi = anchor.max(self.verse_index);
            (lo, hi)
        } else {
            (self.verse_index, self.verse_index)
        }
    }

    pub fn copy_verses_to_clipboard(&mut self) -> Result<()> {
        if self.chapter.verses.is_empty() {
            return Ok(());
        }
        let (lo, hi) = self.verse_copy_indices();
        let text = format_verses_clipboard(&self.chapter, lo, hi);
        self.clipboard.set_text(&text)?;
        let v_lo = self.chapter.verses[lo].reference.verse;
        let v_hi = self.chapter.verses[hi].reference.verse;
        self.status = if lo == hi {
            format!("Copied {v_lo} to clipboard.")
        } else {
            format!("Copied {v_lo}-{v_hi} to clipboard.")
        };
        Ok(())
    }

    fn clear_selection_if_left_anchor_verse(&mut self, verse_index: usize) {
        if let Some((avi, _)) = self.selection_anchor {
            if avi != verse_index {
                self.selection_anchor = None;
            }
        }
    }

    pub fn reload_chapter(&mut self) -> Result<()> {
        self.chapter = self
            .db
            .get_chapter_kjv(&self.book_abbrev, self.chapter_num)?;
        let verse_ids: Vec<i64> = self.chapter.verses.iter().map(|v| v.id).collect();
        self.annotations = self.db.annotations_for_verses(&verse_ids)?;
        self.note_anchors = self.db.note_anchors_for_verses(&verse_ids)?;
        self.selection_anchor = None;
        self.verse_anchor = None;
        self.clamp_cursor();
        self.scroll_top = 0;
        self.verse_list_state.select(Some(self.verse_index));
        self.save_reading_state();
        Ok(())
    }

    pub fn clamp_cursor(&mut self) {
        if self.chapter.verses.is_empty() {
            return;
        }
        self.verse_index = self.verse_index.min(self.chapter.verses.len() - 1);
        let n = self.chapter.verses[self.verse_index].tokens.len();
        self.token_index = if n == 0 { 0 } else { self.token_index.min(n - 1) };
        self.verse_list_state.select(Some(self.verse_index));
    }

    pub fn move_token(&mut self, delta: i32) {
        if self.chapter.verses.is_empty() {
            return;
        }
        let mut vi = self.verse_index;
        let mut ti = self.token_index as i32 + delta;

        loop {
            let tokens = &self.chapter.verses[vi].tokens;
            if tokens.is_empty() {
                if delta > 0 {
                    if vi + 1 < self.chapter.verses.len() {
                        vi += 1;
                        ti = 0;
                        continue;
                    }
                } else if vi > 0 {
                    vi -= 1;
                    ti = self.chapter.verses[vi].tokens.len() as i32 - 1;
                    continue;
                }
                break;
            }
            if ti < 0 {
                if vi == 0 {
                    ti = 0;
                    break;
                }
                vi -= 1;
                ti = self.chapter.verses[vi].tokens.len() as i32 - 1;
                continue;
            }
            if ti >= tokens.len() as i32 {
                if vi + 1 < self.chapter.verses.len() {
                    vi += 1;
                    ti = 0;
                    continue;
                }
                ti = tokens.len() as i32 - 1;
            }
            break;
        }

        self.verse_index = vi;
        self.token_index = ti.max(0) as usize;
        self.clear_selection_if_left_anchor_verse(self.verse_index);
        self.verse_list_state.select(Some(self.verse_index));
        self.save_reading_state();
    }

    pub fn move_verse(&mut self, delta: i32) {
        if self.chapter.verses.is_empty() {
            return;
        }
        let n = self.chapter.verses.len() as i32;
        let vi = (self.verse_index as i32 + delta).clamp(0, n - 1) as usize;
        self.verse_index = vi;
        self.clear_selection_if_left_anchor_verse(self.verse_index);
        self.clamp_cursor();
        self.save_reading_state();
    }

    pub fn change_chapter(&mut self, delta: i32) -> Result<()> {
        let next = self.chapter_num + delta;
        if next < 1 {
            self.status = "Already at first chapter.".into();
            return Ok(());
        }
        let prev = self.chapter_num;
        self.chapter_num = next;
        self.verse_index = 0;
        self.token_index = 0;
        if let Err(e) = self.reload_chapter() {
            self.chapter_num = prev;
            self.reload_chapter()?;
            self.status = format!("Chapter {next} not available: {e}");
        } else {
            self.status = format!("{} {}", self.chapter.book.name, self.chapter_num);
        }
        Ok(())
    }

    pub fn open_book_picker(&mut self) -> Result<()> {
        self.books_available = self.db.list_books_with_content()?;
        self.book_filter.clear();
        self.book_search_active = false;
        self.book_list_state.select(Some(0));
        self.mode = Mode::BookPicker;
        Ok(())
    }

    pub fn filtered_book_indices(&self) -> Vec<usize> {
        if self.book_filter.is_empty() {
            return (0..self.books_available.len()).collect();
        }
        let q = self.book_filter.to_lowercase();
        self.books_available
            .iter()
            .enumerate()
            .filter(|(_, book)| {
                book.name.to_lowercase().contains(&q)
                    || book.abbrev.to_lowercase().contains(&q)
                    || book.osis.to_lowercase().contains(&q)
            })
            .map(|(index, _)| index)
            .collect()
    }

    pub(crate) fn clamp_book_list_selection(&mut self) {
        let count = self.filtered_book_indices().len();
        if count == 0 {
            self.book_list_state.select(None);
            return;
        }
        let selected = self.book_list_state.selected().unwrap_or(0).min(count - 1);
        self.book_list_state.select(Some(selected));
    }

    pub fn select_book(&mut self, filtered_index: usize) -> Result<()> {
        let Some(book_index) = self.filtered_book_indices().get(filtered_index).copied() else {
            return Ok(());
        };
        let Some(book) = self.books_available.get(book_index).cloned() else {
            return Ok(());
        };
        self.book_abbrev = book.abbrev;
        self.chapter_num = 1;
        self.verse_index = 0;
        self.token_index = 0;
        self.reload_chapter()?;
        self.mode = Mode::Reading;
        self.status = format!("Opened {}", book.name);
        Ok(())
    }

    pub fn open_goto(&mut self) {
        self.goto_input.clear();
        self.mode = Mode::Goto;
    }

    pub fn open_chapter_picker(&mut self) -> Result<()> {
        self.max_chapter = self.db.max_chapter(self.chapter.book.id)?;
        if self.max_chapter < 1 {
            self.status = "No chapters in bundle for this book.".into();
            return Ok(());
        }
        self.chapter_filter.clear();
        self.chapter_search_active = false;
        let chapters = self.filtered_chapters();
        let sel = chapters
            .iter()
            .position(|&ch| ch == self.chapter_num)
            .unwrap_or(0);
        self.chapter_list_state.select(Some(sel));
        self.mode = Mode::ChapterPicker;
        Ok(())
    }

    pub fn filtered_chapters(&self) -> Vec<i32> {
        if self.chapter_filter.is_empty() {
            return (1..=self.max_chapter).collect();
        }
        let q = self.chapter_filter.trim();
        (1..=self.max_chapter)
            .filter(|ch| ch.to_string().contains(q))
            .collect()
    }

    pub(crate) fn clamp_chapter_list_selection(&mut self) {
        let chapters = self.filtered_chapters();
        if chapters.is_empty() {
            self.chapter_list_state.select(None);
            return;
        }
        let selected = self
            .chapter_list_state
            .selected()
            .unwrap_or(0)
            .min(chapters.len() - 1);
        self.chapter_list_state.select(Some(selected));
    }

    pub fn select_chapter_from_picker(&mut self, filtered_index: usize) -> Result<()> {
        let Some(&chapter) = self.filtered_chapters().get(filtered_index) else {
            return Ok(());
        };
        self.select_chapter(chapter)
    }

    pub fn select_chapter(&mut self, chapter: i32) -> Result<()> {
        self.chapter_num = chapter;
        self.verse_index = 0;
        self.token_index = 0;
        self.reload_chapter()?;
        self.mode = Mode::Reading;
        self.status = format!("{} {}", self.chapter.book.name, chapter);
        Ok(())
    }

    pub fn open_notes_list(&mut self) -> Result<()> {
        self.all_notes = self.db.list_notes_for_display()?;
        self.notes_filter.clear();
        self.notes_search_active = false;
        self.notes_list_state.select(Some(0));
        self.mode = Mode::NotesList;
        Ok(())
    }

    pub fn filtered_note_indices(&self) -> Vec<usize> {
        if self.notes_filter.is_empty() {
            return (0..self.all_notes.len()).collect();
        }
        let terms: Vec<String> = self
            .notes_filter
            .to_lowercase()
            .split_whitespace()
            .filter(|t| !t.is_empty())
            .map(str::to_string)
            .collect();
        self.all_notes
            .iter()
            .enumerate()
            .filter(|(_, entry)| note_entry_matches(&terms, entry))
            .map(|(index, _)| index)
            .collect()
    }

    pub(crate) fn clamp_notes_list_selection(&mut self) {
        let count = self.filtered_note_indices().len();
        if count == 0 {
            self.notes_list_state.select(None);
            return;
        }
        let selected = self.notes_list_state.selected().unwrap_or(0).min(count - 1);
        self.notes_list_state.select(Some(selected));
    }

    pub fn open_selected_note(&mut self) -> Result<()> {
        let Some(idx) = self.notes_list_state.selected() else {
            return Ok(());
        };
        let Some(note_index) = self.filtered_note_indices().get(idx).copied() else {
            return Ok(());
        };
        let Some(entry) = self.all_notes.get(note_index) else {
            return Ok(());
        };
        let note_id = entry.note.id;
        if let Some(anchor) = self.db.first_note_anchor(note_id)? {
            if let Some(verse_id) = anchor.verse_id {
                self.goto_note_anchor(verse_id, anchor.start_token)?;
            }
        }
        self.open_note_editor_existing(note_id)
    }

    fn goto_note_anchor(&mut self, verse_id: i64, start_token: Option<i32>) -> Result<()> {
        let reference = verse_ref_from_id(verse_id);
        let book = self.db.book_by_id(reference.book_id)?;
        let need_reload =
            self.book_abbrev != book.abbrev || self.chapter_num != reference.chapter;
        self.book_abbrev = book.abbrev;
        self.chapter_num = reference.chapter;
        if need_reload {
            self.reload_chapter()?;
        }
        if let Some(vi) = self
            .chapter
            .verses
            .iter()
            .position(|v| v.reference.verse == reference.verse)
        {
            self.verse_index = vi;
        }
        if let Some(ti) = start_token {
            if let Some(pos) = self.chapter.verses[self.verse_index]
                .tokens
                .iter()
                .position(|t| t.idx == ti)
            {
                self.token_index = pos;
            }
        }
        self.clamp_cursor();
        Ok(())
    }

    pub fn submit_goto(&mut self) -> Result<()> {
        let input = self.goto_input.trim();
        let Some((chapter, verse)) = parse_goto(input) else {
            self.status = "Use chapter:verse (e.g. 3:16) or chapter number.".into();
            self.mode = Mode::Reading;
            return Ok(());
        };
        self.chapter_num = chapter;
        self.reload_chapter()?;
        if let Some(vi) = self
            .chapter
            .verses
            .iter()
            .position(|v| v.reference.verse == verse)
        {
            self.verse_index = vi;
            self.token_index = 0;
        } else {
            self.verse_index = 0;
            self.token_index = 0;
        }
        self.clamp_cursor();
        self.mode = Mode::Reading;
        self.status = format!("{} {}:{}", self.chapter.book.name, chapter, verse);
        Ok(())
    }

    pub fn open_strong_popup(&mut self) -> Result<()> {
        let token = &self.current_verse().tokens[self.token_index];
        let Some(ref key) = token.strong_key else {
            self.status = "No Strong's tag on this word.".into();
            return Ok(());
        };
        let entry = self.db.get_strong(key)?;
        let total = self.db.count_occurrences(key, KJV_TRANSLATION_ID)?;
        let occ = self.db.list_occurrences_kjv(key, 12, 0)?;
        self.strong_entry = Some(entry);
        self.strong_occ_total = total;
        self.strong_occurrences = occ;
        self.strong_occ_list_state.select(Some(0));
        self.mode = Mode::StrongPopup;
        Ok(())
    }

    pub fn close_popup(&mut self) {
        self.mode = Mode::Reading;
        self.strong_entry = None;
        self.strong_occurrences.clear();
        self.strong_occ_list_state.select(Some(0));
    }

    pub fn jump_to_selected_occurrence(&mut self) -> Result<()> {
        let sel = self.strong_occ_list_state.selected().unwrap_or(0);
        let Some(occ) = self.strong_occurrences.get(sel).cloned() else {
            return Ok(());
        };
        self.jump_to_occurrence(&occ)
    }

    pub fn jump_to_occurrence(&mut self, occ: &StrongOccurrence) -> Result<()> {
        self.book_abbrev = occ.book_abbrev.clone();
        self.chapter_num = occ.chapter;
        self.reload_chapter()?;
        if let Some(vi) = self
            .chapter
            .verses
            .iter()
            .position(|v| v.reference.verse == occ.verse)
        {
            self.verse_index = vi;
            if let Some(ti) = self.chapter.verses[vi]
                .tokens
                .iter()
                .position(|t| t.idx == occ.token_idx)
            {
                self.token_index = ti;
            } else {
                self.token_index = 0;
            }
        }
        self.clamp_cursor();
        self.close_popup();
        self.status = format!(
            "Jumped to {} {}:{}",
            occ.book_abbrev, occ.chapter, occ.verse
        );
        Ok(())
    }

    fn reset_note_editor(&mut self, body: &str) {
        let lines: Vec<String> = if body.is_empty() {
            vec![String::new()]
        } else {
            body.lines().map(String::from).collect()
        };
        self.note_body = TextArea::new(lines);
    }

    pub fn open_note_editor_new(&mut self) {
        self.editing_note_id = None;
        self.edit_title.clear();
        let body = "# Note\n\n".to_string();
        self.edit_body = body.clone();
        self.reset_note_editor(&body);
        self.note_focus = NoteEditFocus::Body;
        self.mode = Mode::NoteEditor;
    }

    pub fn open_note_editor_existing(&mut self, note_id: i64) -> Result<()> {
        let note = self.db.get_note(note_id)?;
        self.editing_note_id = Some(note_id);
        self.edit_title = note.title.unwrap_or_default();
        self.edit_body = note.body.clone();
        self.reset_note_editor(&note.body);
        self.note_focus = NoteEditFocus::Body;
        self.mode = Mode::NoteEditor;
        Ok(())
    }

    pub fn open_note_at_cursor(&mut self) -> Result<()> {
        let ti = self.cursor_token_idx();
        if let Some(note_id) = note_id_at_token(&self.note_anchors, ti) {
            return self.open_note_editor_existing(note_id);
        }
        self.open_note_editor_new();
        Ok(())
    }

    pub fn save_note(&mut self) -> Result<()> {
        self.edit_body = self.note_body.lines().join("\n");
        let title = if self.edit_title.is_empty() {
            None
        } else {
            Some(self.edit_title.as_str())
        };
        let note_id = if let Some(id) = self.editing_note_id {
            self.db.update_note(id, title, &self.edit_body)?;
            id
        } else {
            let id = self.db.create_note(title, &self.edit_body)?;
            let idx = self.cursor_token_idx();
            self.db.add_note_anchor_token_range(
                id,
                self.cursor_verse_id(),
                idx,
                idx + 1,
                None,
            )?;
            id
        };
        self.reload_chapter()?;
        self.mode = Mode::Reading;
        if let Ok(location) = self.db.note_location_label(note_id) {
            self.status = format!("Saved note #{note_id} at {location}");
        } else {
            self.status = format!("Saved note #{note_id}");
        }
        Ok(())
    }

    pub fn delete_note_at_cursor(&mut self) -> Result<()> {
        let ti = self.cursor_token_idx();
        let Some(note_id) = note_id_at_token(&self.note_anchors, ti) else {
            self.status = "No note on this word.".into();
            return Ok(());
        };
        self.db.delete_note(note_id)?;
        self.reload_chapter()?;
        self.status = format!("Deleted note #{note_id}");
        Ok(())
    }

    pub fn toggle_highlight(&mut self) -> Result<()> {
        let verse_id = self.cursor_verse_id();
        let (start, end) = self.annotation_token_range();
        let range_words = (end - start) as usize;
        if let Some(existing) = self
            .annotations
            .iter()
            .find(|a| {
                a.verse_id == verse_id
                    && a.kind == AnnotationKind::Highlight
                    && a.start_token == start
                    && a.end_token == end
            })
            .map(|a| a.id)
        {
            self.db.delete_annotation(existing)?;
            self.status = "Removed highlight.".into();
        } else {
            self.db.create_annotation(
                AnnotationKind::Highlight,
                verse_id,
                start,
                end,
                None,
                Some(1),
            )?;
            self.status = if range_words > 1 {
                format!("Highlighted {range_words} words.")
            } else {
                "Highlighted word.".into()
            };
        }
        self.refresh_annotations();
        Ok(())
    }

    pub fn toggle_underline(&mut self) -> Result<()> {
        let verse_id = self.cursor_verse_id();
        let (start, end) = self.annotation_token_range();
        let range_words = (end - start) as usize;
        if let Some(existing) = self
            .annotations
            .iter()
            .find(|a| {
                a.verse_id == verse_id
                    && a.kind == AnnotationKind::Underline
                    && a.start_token == start
                    && a.end_token == end
            })
            .map(|a| a.id)
        {
            self.db.delete_annotation(existing)?;
            self.status = "Removed underline.".into();
        } else {
            self.db
                .create_annotation(AnnotationKind::Underline, verse_id, start, end, None, None)?;
            self.status = if range_words > 1 {
                format!("Underlined {range_words} words.")
            } else {
                "Underlined word.".into()
            };
        }
        self.refresh_annotations();
        Ok(())
    }

    fn refresh_annotations(&mut self) {
        let verse_ids: Vec<i64> = self.chapter.verses.iter().map(|v| v.id).collect();
        if let Ok(ann) = self.db.annotations_for_verses(&verse_ids) {
            self.annotations = ann;
        }
    }

    pub fn search_ran_query(&self) -> &str {
        &self.search_ran_query
    }

    pub fn touch_search_query(&mut self) {
        self.search_last_edit = Some(Instant::now());
    }

    pub fn reset_search_session(&mut self) {
        self.search_query.clear();
        self.search_results.clear();
        self.search_ran_query.clear();
        self.search_last_edit = None;
        self.search_list_state.select(Some(0));
    }

    /// Run search when the debounce timer has elapsed and the query changed.
    pub fn tick_search(&mut self) -> Result<()> {
        if self.mode != Mode::Search {
            return Ok(());
        }
        let query = self.search_query.trim();
        if query == self.search_ran_query.as_str() {
            return Ok(());
        }
        if query.is_empty() {
            self.search_results.clear();
            self.search_ran_query.clear();
            self.search_list_state.select(Some(0));
            return Ok(());
        }
        if query.len() < SEARCH_MIN_CHARS {
            return Ok(());
        }
        let Some(edited) = self.search_last_edit else {
            return Ok(());
        };
        if edited.elapsed() < SEARCH_DEBOUNCE {
            return Ok(());
        }
        let query_owned = query.to_string();
        self.run_search()?;
        self.search_ran_query = query_owned;
        self.status = format!("{} hits", self.search_results.len());
        Ok(())
    }

    pub fn run_search(&mut self) -> Result<()> {
        let query = self.search_query.trim();
        if query.is_empty() {
            self.search_results.clear();
            self.search_ran_query.clear();
            return Ok(());
        }
        self.search_results = self
            .db
            .search_verses(query, KJV_TRANSLATION_ID, 25)?;
        self.search_list_state.select(Some(0));
        self.search_ran_query = query.to_string();
        Ok(())
    }

    pub fn jump_to_search_hit(&mut self) -> Result<()> {
        let sel = self.search_list_state.selected().unwrap_or(0);
        let Some(hit) = self.search_results.get(sel).cloned() else {
            return Ok(());
        };
        if hit.book_abbrev != self.book_abbrev {
            self.book_abbrev = hit.book_abbrev.clone();
        }
        self.chapter_num = hit.chapter;
        self.reload_chapter()?;
        let target_verse = hit.verse;
        if let Some(vi) = self
            .chapter
            .verses
            .iter()
            .position(|v| v.reference.verse == target_verse)
        {
            self.verse_index = vi;
            self.token_index = 0;
        }
        self.clamp_cursor();
        self.mode = Mode::Reading;
        self.status = format!(
            "Jumped to {} {}:{}",
            hit.book_name, hit.chapter, hit.verse
        );
        Ok(())
    }

    fn save_reading_state(&self) {
        if self.chapter.verses.is_empty() {
            return;
        }
        let _ = self.db.set_reading_state(
            KJV_TRANSLATION_ID,
            self.chapter.book.id,
            self.chapter_num,
            Some(self.current_verse().reference.verse),
            Some(self.cursor_token_idx()),
        );
    }

    fn restore_reading_state(&mut self) {
        let Ok(Some(state)) = self.db.get_reading_state() else {
            return;
        };
        if state.book_id != self.chapter.book.id || state.chapter != self.chapter_num {
            return;
        }
        if let Some(v) = state.verse {
            if let Some(vi) = self
                .chapter
                .verses
                .iter()
                .position(|verse| verse.reference.verse == v)
            {
                self.verse_index = vi;
            }
        }
        if let Some(ti) = state.token_idx {
            if let Some(pos) = self.chapter.verses[self.verse_index]
                .tokens
                .iter()
                .position(|t| t.idx == ti)
            {
                self.token_index = pos;
            }
        }
        self.clamp_cursor();
    }
}

fn empty_chapter(chapter: i32) -> Chapter {
    Chapter {
        book: Book {
            id: 0,
            osis: String::new(),
            abbrev: String::new(),
            name: String::new(),
            testament: String::new(),
            sort_order: 0,
        },
        chapter,
        translation_id: KJV_TRANSLATION_ID,
        verses: Vec::new(),
    }
}

fn note_entry_matches(terms: &[String], entry: &NoteListEntry) -> bool {
    let title = entry
        .note
        .title
        .as_deref()
        .unwrap_or("")
        .to_lowercase();
    let body = crate::markdown::to_plain(&entry.note.body).to_lowercase();
    let haystack = format!(
        "{} {} {} #{}",
        entry.location.to_lowercase(),
        title,
        body,
        entry.note.id
    );
    terms.iter().all(|term| haystack.contains(term))
}

fn parse_goto(input: &str) -> Option<(i32, i32)> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }
    if let Some((ch, vs)) = input.split_once(':') {
        Some((ch.parse().ok()?, vs.parse().ok()?))
    } else {
        Some((input.parse().ok()?, 1))
    }
}
