-- Mutable user data (notes, annotations, reading state).
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;

CREATE TABLE note (
  id            INTEGER PRIMARY KEY,
  title         TEXT,
  body          TEXT NOT NULL,
  created_at    TEXT NOT NULL,
  updated_at    TEXT NOT NULL
);

CREATE TABLE note_anchor (
  id              INTEGER PRIMARY KEY,
  note_id         INTEGER NOT NULL REFERENCES note(id) ON DELETE CASCADE,
  verse_id        INTEGER,
  start_token     INTEGER,
  end_token       INTEGER,
  strong_key      TEXT,
  translation_id  INTEGER,
  CHECK (
    (verse_id IS NOT NULL AND start_token IS NOT NULL AND end_token IS NOT NULL)
    OR strong_key IS NOT NULL
  )
);

CREATE INDEX idx_note_anchor_verse ON note_anchor (verse_id);
CREATE INDEX idx_note_anchor_note ON note_anchor (note_id);

CREATE TABLE annotation (
  id              INTEGER PRIMARY KEY,
  kind            TEXT NOT NULL CHECK (kind IN ('highlight', 'underline')),
  verse_id        INTEGER NOT NULL,
  start_token     INTEGER NOT NULL,
  end_token       INTEGER NOT NULL,
  translation_id  INTEGER,
  color           INTEGER,
  created_at      TEXT NOT NULL,
  updated_at      TEXT NOT NULL
);

CREATE INDEX idx_annotation_verse ON annotation (verse_id);

CREATE TABLE reading_state (
  id              INTEGER PRIMARY KEY CHECK (id = 1),
  translation_id  INTEGER NOT NULL DEFAULT 1,
  book_id         INTEGER NOT NULL,
  chapter         INTEGER NOT NULL,
  verse           INTEGER,
  token_idx       INTEGER,
  updated_at      TEXT NOT NULL
);
