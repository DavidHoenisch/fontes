#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VerseRef {
    pub book_id: i64,
    pub chapter: i32,
    pub verse: i32,
}

impl VerseRef {
    pub fn verse_id(self) -> i64 {
        verse_id(self.book_id, self.chapter, self.verse)
    }
}

/// Matches the ingest formula in `tools/data/build_content.py`.
pub fn verse_id(book_id: i64, chapter: i32, verse: i32) -> i64 {
    book_id * 1_000_000 + i64::from(chapter) * 1_000 + i64::from(verse)
}

pub fn verse_ref_from_id(id: i64) -> VerseRef {
    let book_id = id / 1_000_000;
    let chapter = ((id % 1_000_000) / 1_000) as i32;
    let verse = (id % 1_000) as i32;
    VerseRef {
        book_id,
        chapter,
        verse,
    }
}

#[derive(Debug, Clone)]
pub struct Book {
    pub id: i64,
    pub osis: String,
    pub abbrev: String,
    pub name: String,
    pub testament: String,
    pub sort_order: i32,
}

#[derive(Debug, Clone)]
pub struct Translation {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub language: String,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub idx: i32,
    pub surface: String,
    pub strong_key: Option<String>,
    pub flags: i32,
}

#[derive(Debug, Clone)]
pub struct Verse {
    pub id: i64,
    pub reference: VerseRef,
    pub text: String,
    pub tokens: Vec<Token>,
}

#[derive(Debug, Clone)]
pub struct Chapter {
    pub book: Book,
    pub chapter: i32,
    pub translation_id: i64,
    pub verses: Vec<Verse>,
}

#[derive(Debug, Clone)]
pub struct StrongEntry {
    pub key: String,
    pub lang: String,
    pub lemma: Option<String>,
    pub translit: Option<String>,
    pub definition: String,
    pub kjv_gloss: Option<String>,
    pub derivation: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StrongOccurrence {
    pub verse_id: i64,
    pub book_abbrev: String,
    pub book_name: String,
    pub chapter: i32,
    pub verse: i32,
    pub token_idx: i32,
}

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub verse_id: i64,
    pub book_abbrev: String,
    pub book_name: String,
    pub chapter: i32,
    pub verse: i32,
    pub snippet: String,
}

#[derive(Debug, Clone)]
pub struct Note {
    pub id: i64,
    pub title: Option<String>,
    pub body: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Note with a resolved anchor for display (e.g. notes list).
#[derive(Debug, Clone)]
pub struct NoteListEntry {
    pub note: Note,
    /// e.g. `Jhn 3:16`, `Strong G3056`, or `—` when unanchored.
    pub location: String,
}

#[derive(Debug, Clone)]
pub struct NoteAnchor {
    pub id: i64,
    pub note_id: i64,
    pub verse_id: Option<i64>,
    pub start_token: Option<i32>,
    pub end_token: Option<i32>,
    pub strong_key: Option<String>,
    pub translation_id: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnotationKind {
    Highlight,
    Underline,
}

impl AnnotationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Highlight => "highlight",
            Self::Underline => "underline",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "highlight" => Some(Self::Highlight),
            "underline" => Some(Self::Underline),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Annotation {
    pub id: i64,
    pub kind: AnnotationKind,
    pub verse_id: i64,
    pub start_token: i32,
    pub end_token: i32,
    pub translation_id: Option<i64>,
    pub color: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct ReadingState {
    pub translation_id: i64,
    pub book_id: i64,
    pub chapter: i32,
    pub verse: Option<i32>,
    pub token_idx: Option<i32>,
    pub updated_at: String,
}
