use rusqlite::params;

use crate::db::{Database, KJV_TRANSLATION_ID};
use crate::error::{Error, Result};
use crate::model::{StrongEntry, StrongOccurrence, Token};

impl Database {
    pub fn get_strong(&self, key: &str) -> Result<StrongEntry> {
        let mut stmt = self.content().prepare(
            "SELECT key, lang, lemma, translit, definition, kjv_gloss, derivation
             FROM strong_entry WHERE key = ?1",
        )?;
        stmt.query_row([key], |row| {
            Ok(StrongEntry {
                key: row.get(0)?,
                lang: row.get(1)?,
                lemma: row.get(2)?,
                translit: row.get(3)?,
                definition: row.get(4)?,
                kjv_gloss: row.get(5)?,
                derivation: row.get(6)?,
            })
        })
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Error::NotFound("strong", key.to_string()),
            other => other.into(),
        })
    }

    pub fn strong_xrefs(&self, from_key: &str) -> Result<Vec<String>> {
        let mut stmt = self
            .content()
            .prepare("SELECT to_key FROM strong_xref WHERE from_key = ?1 ORDER BY to_key")?;
        let rows = stmt.query_map([from_key], |row| row.get(0))?;
        rows.collect::<std::result::Result<_, _>>()
            .map_err(Into::into)
    }

    pub fn list_occurrences(
        &self,
        strong_key: &str,
        translation_id: i64,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<StrongOccurrence>> {
        let mut stmt = self.content().prepare(
            "SELECT o.verse_id, b.abbrev, b.name, v.chapter, v.verse, o.token_idx
             FROM strong_occurrence o
             JOIN verse v ON v.id = o.verse_id
             JOIN book b ON b.id = v.book_id
             WHERE o.strong_key = ?1 AND o.translation_id = ?2
             ORDER BY b.sort_order, v.chapter, v.verse, o.token_idx
             LIMIT ?3 OFFSET ?4",
        )?;
        let rows = stmt.query_map(
            params![
                strong_key,
                translation_id,
                limit.try_into().unwrap_or(i64::MAX),
                offset.try_into().unwrap_or(0)
            ],
            |row| {
                Ok(StrongOccurrence {
                    verse_id: row.get(0)?,
                    book_abbrev: row.get(1)?,
                    book_name: row.get(2)?,
                    chapter: row.get(3)?,
                    verse: row.get(4)?,
                    token_idx: row.get(5)?,
                })
            },
        )?;
        rows.collect::<std::result::Result<_, _>>()
            .map_err(Into::into)
    }

    pub fn list_occurrences_kjv(
        &self,
        strong_key: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<StrongOccurrence>> {
        self.list_occurrences(strong_key, KJV_TRANSLATION_ID, limit, offset)
    }

    pub fn count_occurrences(&self, strong_key: &str, translation_id: i64) -> Result<usize> {
        let count: i64 = self.content().query_row(
            "SELECT COUNT(*) FROM strong_occurrence
             WHERE strong_key = ?1 AND translation_id = ?2",
            params![strong_key, translation_id],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Strong's entry for the token at `token_idx` in a verse, if tagged.
    pub fn strong_at_token(
        &self,
        verse_id: i64,
        token_idx: i32,
        translation_id: i64,
    ) -> Result<Option<StrongEntry>> {
        let key: Option<String> = match self.content().query_row(
            "SELECT strong_key FROM token
             WHERE verse_id = ?1 AND translation_id = ?2 AND idx = ?3",
            params![verse_id, translation_id, token_idx],
            |row| row.get::<_, Option<String>>(0),
        ) {
            Ok(k) => k,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        match key {
            Some(k) => self.get_strong(&k).map(Some),
            None => Ok(None),
        }
    }

    pub fn token_at(
        &self,
        verse_id: i64,
        token_idx: i32,
        translation_id: i64,
    ) -> Result<Option<Token>> {
        let mut stmt = self.content().prepare(
            "SELECT idx, surface, strong_key, flags FROM token
             WHERE verse_id = ?1 AND translation_id = ?2 AND idx = ?3",
        )?;
        let result = stmt.query_row(params![verse_id, translation_id, token_idx], |row| {
            Ok(Token {
                idx: row.get(0)?,
                surface: row.get(1)?,
                strong_key: row.get(2)?,
                flags: row.get(3)?,
            })
        });
        match result {
            Ok(t) => Ok(Some(t)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
