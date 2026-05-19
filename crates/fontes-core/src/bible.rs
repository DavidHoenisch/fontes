use rusqlite::params;

use crate::db::{Database, KJV_TRANSLATION_ID};
use crate::error::{Error, Result};
use crate::model::{Book, Chapter, Token, Verse, VerseRef};

impl Database {
    pub fn list_books(&self) -> Result<Vec<Book>> {
        let mut stmt = self.content().prepare(
            "SELECT id, osis, abbrev, name, testament, sort_order
             FROM book ORDER BY sort_order",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Book {
                id: row.get(0)?,
                osis: row.get(1)?,
                abbrev: row.get(2)?,
                name: row.get(3)?,
                testament: row.get(4)?,
                sort_order: row.get(5)?,
            })
        })?;
        rows.collect::<std::result::Result<_, _>>().map_err(Into::into)
    }

    /// Books that have at least one verse in the content bundle.
    pub fn list_books_with_content(&self) -> Result<Vec<Book>> {
        let mut stmt = self.content().prepare(
            "SELECT DISTINCT b.id, b.osis, b.abbrev, b.name, b.testament, b.sort_order
             FROM book b
             JOIN verse v ON v.book_id = b.id
             ORDER BY b.sort_order",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Book {
                id: row.get(0)?,
                osis: row.get(1)?,
                abbrev: row.get(2)?,
                name: row.get(3)?,
                testament: row.get(4)?,
                sort_order: row.get(5)?,
            })
        })?;
        rows.collect::<std::result::Result<_, _>>().map_err(Into::into)
    }

    pub fn max_chapter(&self, book_id: i64) -> Result<i32> {
        let max: Option<i32> = self.content().query_row(
            "SELECT MAX(chapter) FROM verse WHERE book_id = ?1",
            [book_id],
            |row| row.get(0),
        )?;
        Ok(max.unwrap_or(0))
    }

    pub fn book_by_id(&self, book_id: i64) -> Result<Book> {
        let mut stmt = self.content().prepare(
            "SELECT id, osis, abbrev, name, testament, sort_order
             FROM book WHERE id = ?1",
        )?;
        stmt.query_row([book_id], |row| {
            Ok(Book {
                id: row.get(0)?,
                osis: row.get(1)?,
                abbrev: row.get(2)?,
                name: row.get(3)?,
                testament: row.get(4)?,
                sort_order: row.get(5)?,
            })
        })
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                Error::NotFound("book", book_id.to_string())
            }
            other => other.into(),
        })
    }

    pub fn book_by_abbrev(&self, abbrev: &str) -> Result<Book> {
        let mut stmt = self.content().prepare(
            "SELECT id, osis, abbrev, name, testament, sort_order
             FROM book WHERE abbrev = ?1",
        )?;
        stmt.query_row([abbrev], |row| {
            Ok(Book {
                id: row.get(0)?,
                osis: row.get(1)?,
                abbrev: row.get(2)?,
                name: row.get(3)?,
                testament: row.get(4)?,
                sort_order: row.get(5)?,
            })
        })
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                Error::NotFound("book", abbrev.to_string())
            }
            other => other.into(),
        })
    }

    pub fn get_chapter(
        &self,
        book_abbrev: &str,
        chapter: i32,
        translation_id: i64,
    ) -> Result<Chapter> {
        let book = self.book_by_abbrev(book_abbrev)?;
        let mut verse_stmt = self.content().prepare(
            "SELECT v.id, v.verse, vt.text
             FROM verse v
             JOIN verse_text vt
               ON vt.verse_id = v.id AND vt.translation_id = ?1
             WHERE v.book_id = ?2 AND v.chapter = ?3
             ORDER BY v.verse",
        )?;

        let verse_rows = verse_stmt.query_map(params![translation_id, book.id, chapter], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i32>(1)?, row.get::<_, String>(2)?))
        })?;

        let mut verses = Vec::new();
        for row in verse_rows {
            let (verse_id, verse_num, text) = row?;
            let tokens = self.tokens_for_verse(verse_id, translation_id)?;
            verses.push(Verse {
                id: verse_id,
                reference: VerseRef {
                    book_id: book.id,
                    chapter,
                    verse: verse_num,
                },
                text,
                tokens,
            });
        }

        Ok(Chapter {
            book,
            chapter,
            translation_id,
            verses,
        })
    }

    pub fn get_chapter_kjv(&self, book_abbrev: &str, chapter: i32) -> Result<Chapter> {
        self.get_chapter(book_abbrev, chapter, KJV_TRANSLATION_ID)
    }

    fn tokens_for_verse(&self, verse_id: i64, translation_id: i64) -> Result<Vec<Token>> {
        let mut stmt = self.content().prepare(
            "SELECT idx, surface, strong_key, flags
             FROM token
             WHERE verse_id = ?1 AND translation_id = ?2
             ORDER BY idx",
        )?;
        let rows = stmt.query_map(params![verse_id, translation_id], |row| {
            Ok(Token {
                idx: row.get(0)?,
                surface: row.get(1)?,
                strong_key: row.get(2)?,
                flags: row.get(3)?,
            })
        })?;
        rows.collect::<std::result::Result<_, _>>().map_err(Into::into)
    }
}
