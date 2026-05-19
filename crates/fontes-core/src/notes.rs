use rusqlite::params;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::db::Database;
use crate::error::{Error, Result};
use crate::model::{
    verse_ref_from_id, Annotation, AnnotationKind, Note, NoteAnchor, NoteListEntry, ReadingState,
};

fn now_rfc3339() -> Result<String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|e| Error::Message(e.to_string()))
}

impl Database {
    // --- Notes ---

    pub fn create_note(&self, title: Option<&str>, body: &str) -> Result<i64> {
        let now = now_rfc3339()?;
        self.user().execute(
            "INSERT INTO note (title, body, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
            params![title, body, now],
        )?;
        Ok(self.user().last_insert_rowid())
    }

    pub fn get_note(&self, id: i64) -> Result<Note> {
        let mut stmt = self
            .user()
            .prepare("SELECT id, title, body, created_at, updated_at FROM note WHERE id = ?1")?;
        stmt.query_row([id], map_note).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Error::NotFound("note", id.to_string()),
            other => other.into(),
        })
    }

    pub fn update_note(&self, id: i64, title: Option<&str>, body: &str) -> Result<()> {
        let now = now_rfc3339()?;
        let n = self.user().execute(
            "UPDATE note SET title = ?1, body = ?2, updated_at = ?3 WHERE id = ?4",
            params![title, body, now, id],
        )?;
        if n == 0 {
            return Err(Error::NotFound("note", id.to_string()));
        }
        Ok(())
    }

    pub fn delete_note(&self, id: i64) -> Result<()> {
        let n = self
            .user()
            .execute("DELETE FROM note WHERE id = ?1", [id])?;
        if n == 0 {
            return Err(Error::NotFound("note", id.to_string()));
        }
        Ok(())
    }

    pub fn list_notes(&self) -> Result<Vec<Note>> {
        let mut stmt = self.user().prepare(
            "SELECT id, title, body, created_at, updated_at FROM note ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], map_note)?;
        rows.collect::<std::result::Result<_, _>>()
            .map_err(Into::into)
    }

    pub fn list_notes_for_display(&self) -> Result<Vec<NoteListEntry>> {
        self.list_notes()?
            .into_iter()
            .map(|note| {
                let location = self.note_location_label(note.id)?;
                Ok(NoteListEntry { note, location })
            })
            .collect()
    }

    pub fn note_location_label(&self, note_id: i64) -> Result<String> {
        let Some(anchor) = self.first_note_anchor(note_id)? else {
            return Ok("—".to_string());
        };
        if let Some(verse_id) = anchor.verse_id {
            let reference = verse_ref_from_id(verse_id);
            let book = self.book_by_id(reference.book_id)?;
            return Ok(format!(
                "{} {}:{}",
                book.abbrev, reference.chapter, reference.verse
            ));
        }
        if let Some(ref key) = anchor.strong_key {
            return Ok(format!("Strong {key}"));
        }
        Ok("—".to_string())
    }

    pub fn first_note_anchor(&self, note_id: i64) -> Result<Option<NoteAnchor>> {
        let mut stmt = self.user().prepare(
            "SELECT id, note_id, verse_id, start_token, end_token, strong_key, translation_id
             FROM note_anchor
             WHERE note_id = ?1 AND verse_id IS NOT NULL
             ORDER BY id
             LIMIT 1",
        )?;
        let mut rows = stmt.query([note_id])?;
        if let Some(row) = rows.next()? {
            return map_note_anchor(&row).map(Some).map_err(Into::into);
        }
        Ok(None)
    }

    pub fn add_note_anchor_token_range(
        &self,
        note_id: i64,
        verse_id: i64,
        start_token: i32,
        end_token: i32,
        translation_id: Option<i64>,
    ) -> Result<i64> {
        self.user().execute(
            "INSERT INTO note_anchor
               (note_id, verse_id, start_token, end_token, strong_key, translation_id)
             VALUES (?1, ?2, ?3, ?4, NULL, ?5)",
            params![note_id, verse_id, start_token, end_token, translation_id],
        )?;
        Ok(self.user().last_insert_rowid())
    }

    pub fn add_note_anchor_strong(&self, note_id: i64, strong_key: &str) -> Result<i64> {
        self.user().execute(
            "INSERT INTO note_anchor
               (note_id, verse_id, start_token, end_token, strong_key, translation_id)
             VALUES (?1, NULL, NULL, NULL, ?2, NULL)",
            params![note_id, strong_key],
        )?;
        Ok(self.user().last_insert_rowid())
    }

    pub fn note_anchors_for_verses(&self, verse_ids: &[i64]) -> Result<Vec<NoteAnchor>> {
        if verse_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = verse_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let sql = format!(
            "SELECT id, note_id, verse_id, start_token, end_token, strong_key, translation_id
             FROM note_anchor WHERE verse_id IN ({placeholders})"
        );
        let mut stmt = self.user().prepare(&sql)?;
        let params: Vec<&dyn rusqlite::types::ToSql> = verse_ids
            .iter()
            .map(|id| id as &dyn rusqlite::types::ToSql)
            .collect();
        let rows = stmt.query_map(params.as_slice(), map_note_anchor)?;
        rows.collect::<std::result::Result<_, _>>()
            .map_err(Into::into)
    }

    // --- Annotations ---

    pub fn create_annotation(
        &self,
        kind: AnnotationKind,
        verse_id: i64,
        start_token: i32,
        end_token: i32,
        translation_id: Option<i64>,
        color: Option<i32>,
    ) -> Result<i64> {
        let now = now_rfc3339()?;
        self.user().execute(
            "INSERT INTO annotation
               (kind, verse_id, start_token, end_token, translation_id, color, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![
                kind.as_str(),
                verse_id,
                start_token,
                end_token,
                translation_id,
                color,
                now
            ],
        )?;
        Ok(self.user().last_insert_rowid())
    }

    pub fn delete_annotation(&self, id: i64) -> Result<()> {
        let n = self
            .user()
            .execute("DELETE FROM annotation WHERE id = ?1", [id])?;
        if n == 0 {
            return Err(Error::NotFound("annotation", id.to_string()));
        }
        Ok(())
    }

    pub fn annotations_for_verses(&self, verse_ids: &[i64]) -> Result<Vec<Annotation>> {
        if verse_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = verse_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let sql = format!(
            "SELECT id, kind, verse_id, start_token, end_token, translation_id, color, created_at, updated_at
             FROM annotation WHERE verse_id IN ({placeholders})
             ORDER BY verse_id, start_token"
        );
        let mut stmt = self.user().prepare(&sql)?;
        let params: Vec<&dyn rusqlite::types::ToSql> = verse_ids
            .iter()
            .map(|id| id as &dyn rusqlite::types::ToSql)
            .collect();
        let rows = stmt.query_map(params.as_slice(), |row| {
            let kind_str: String = row.get(1)?;
            let kind = AnnotationKind::parse(&kind_str).ok_or_else(|| {
                rusqlite::Error::InvalidColumnType(1, kind_str, rusqlite::types::Type::Text)
            })?;
            Ok(Annotation {
                id: row.get(0)?,
                kind,
                verse_id: row.get(2)?,
                start_token: row.get(3)?,
                end_token: row.get(4)?,
                translation_id: row.get(5)?,
                color: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;
        rows.collect::<std::result::Result<_, _>>()
            .map_err(Into::into)
    }

    // --- Reading state ---

    pub fn get_reading_state(&self) -> Result<Option<ReadingState>> {
        let mut stmt = self.user().prepare(
            "SELECT translation_id, book_id, chapter, verse, token_idx, updated_at
             FROM reading_state WHERE id = 1",
        )?;
        let result = stmt.query_row([], |row| {
            Ok(ReadingState {
                translation_id: row.get(0)?,
                book_id: row.get(1)?,
                chapter: row.get(2)?,
                verse: row.get(3)?,
                token_idx: row.get(4)?,
                updated_at: row.get(5)?,
            })
        });
        match result {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn set_reading_state(
        &self,
        translation_id: i64,
        book_id: i64,
        chapter: i32,
        verse: Option<i32>,
        token_idx: Option<i32>,
    ) -> Result<()> {
        let now = now_rfc3339()?;
        self.user().execute(
            "INSERT INTO reading_state (id, translation_id, book_id, chapter, verse, token_idx, updated_at)
             VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
               translation_id = excluded.translation_id,
               book_id = excluded.book_id,
               chapter = excluded.chapter,
               verse = excluded.verse,
               token_idx = excluded.token_idx,
               updated_at = excluded.updated_at",
            params![translation_id, book_id, chapter, verse, token_idx, now],
        )?;
        Ok(())
    }
}

fn map_note(row: &rusqlite::Row<'_>) -> rusqlite::Result<Note> {
    Ok(Note {
        id: row.get(0)?,
        title: row.get(1)?,
        body: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

fn map_note_anchor(row: &rusqlite::Row<'_>) -> rusqlite::Result<NoteAnchor> {
    Ok(NoteAnchor {
        id: row.get(0)?,
        note_id: row.get(1)?,
        verse_id: row.get(2)?,
        start_token: row.get(3)?,
        end_token: row.get(4)?,
        strong_key: row.get(5)?,
        translation_id: row.get(6)?,
    })
}
