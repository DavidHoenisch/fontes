-- Immutable scripture + lexicon database (replaced on sync).
PRAGMA foreign_keys = ON;

CREATE TABLE translation (
  id            INTEGER PRIMARY KEY,
  code          TEXT NOT NULL UNIQUE,
  name          TEXT NOT NULL,
  language      TEXT NOT NULL DEFAULT 'en',
  license       TEXT
);

CREATE TABLE book (
  id            INTEGER PRIMARY KEY,
  osis          TEXT NOT NULL UNIQUE,
  abbrev        TEXT NOT NULL UNIQUE,
  name          TEXT NOT NULL,
  testament     TEXT NOT NULL CHECK (testament IN ('OT', 'NT')),
  sort_order    INTEGER NOT NULL UNIQUE
);

CREATE TABLE verse (
  id            INTEGER PRIMARY KEY,
  book_id       INTEGER NOT NULL REFERENCES book(id),
  chapter       INTEGER NOT NULL,
  verse         INTEGER NOT NULL,
  UNIQUE (book_id, chapter, verse)
);

CREATE INDEX idx_verse_book_chapter ON verse (book_id, chapter);

CREATE TABLE verse_text (
  verse_id        INTEGER NOT NULL REFERENCES verse(id),
  translation_id  INTEGER NOT NULL REFERENCES translation(id),
  text            TEXT NOT NULL,
  PRIMARY KEY (verse_id, translation_id)
);

CREATE TABLE token (
  verse_id        INTEGER NOT NULL REFERENCES verse(id),
  translation_id  INTEGER NOT NULL REFERENCES translation(id),
  idx             INTEGER NOT NULL,
  surface         TEXT NOT NULL,
  strong_key      TEXT,
  flags           INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (verse_id, translation_id, idx)
);

CREATE INDEX idx_token_strong
  ON token (translation_id, strong_key)
  WHERE strong_key IS NOT NULL;

CREATE TABLE strong_entry (
  key           TEXT PRIMARY KEY,
  lang          TEXT NOT NULL CHECK (lang IN ('hebrew', 'greek')),
  lemma         TEXT,
  translit      TEXT,
  definition    TEXT NOT NULL,
  kjv_gloss     TEXT,
  derivation    TEXT
);

CREATE TABLE strong_xref (
  from_key      TEXT NOT NULL REFERENCES strong_entry(key),
  to_key        TEXT NOT NULL REFERENCES strong_entry(key),
  PRIMARY KEY (from_key, to_key)
);

CREATE INDEX idx_strong_xref_to ON strong_xref (to_key);

CREATE TABLE strong_occurrence (
  strong_key      TEXT NOT NULL,
  translation_id  INTEGER NOT NULL REFERENCES translation(id),
  verse_id        INTEGER NOT NULL REFERENCES verse(id),
  token_idx       INTEGER NOT NULL,
  PRIMARY KEY (strong_key, translation_id, verse_id, token_idx)
);

CREATE INDEX idx_occ_strong ON strong_occurrence (translation_id, strong_key);
CREATE INDEX idx_occ_verse ON strong_occurrence (verse_id);

-- Full-text search over plain verse text (filter by translation_id).
CREATE VIRTUAL TABLE verse_fts USING fts5(
  text,
  translation_id UNINDEXED,
  verse_id UNINDEXED,
  tokenize='unicode61 remove_diacritics 2'
);

CREATE TRIGGER verse_text_ai AFTER INSERT ON verse_text BEGIN
  INSERT INTO verse_fts (text, translation_id, verse_id)
  VALUES (new.text, new.translation_id, new.verse_id);
END;

CREATE TRIGGER verse_text_ad AFTER DELETE ON verse_text BEGIN
  INSERT INTO verse_fts (verse_fts, text, translation_id, verse_id)
  VALUES ('delete', old.text, old.translation_id, old.verse_id);
END;

CREATE TRIGGER verse_text_au AFTER UPDATE ON verse_text BEGIN
  INSERT INTO verse_fts (verse_fts, text, translation_id, verse_id)
  VALUES ('delete', old.text, old.translation_id, old.verse_id);
  INSERT INTO verse_fts (text, translation_id, verse_id)
  VALUES (new.text, new.translation_id, new.verse_id);
END;

-- Bundle metadata (written at build time).
CREATE TABLE bundle_meta (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
