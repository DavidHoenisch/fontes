use rusqlite::params;

use crate::db::Database;
use crate::error::Result;
use crate::model::SearchHit;

impl Database {
    /// Full-text search over verse text for a translation.
    pub fn search_verses(
        &self,
        query: &str,
        translation_id: i64,
        limit: usize,
    ) -> Result<Vec<SearchHit>> {
        let fts_query = escape_fts_query(query);
        let mut stmt = self.content().prepare(
            "SELECT f.verse_id, b.abbrev, b.name, v.chapter, v.verse, vt.text
             FROM verse_fts f
             JOIN verse v ON v.id = f.verse_id
             JOIN book b ON b.id = v.book_id
             JOIN verse_text vt
               ON vt.verse_id = f.verse_id AND vt.translation_id = f.translation_id
             WHERE verse_fts MATCH ?1 AND f.translation_id = ?2
             ORDER BY b.sort_order, v.chapter, v.verse
             LIMIT ?3",
        )?;
        let rows = stmt.query_map(
            params![
                fts_query,
                translation_id,
                limit.try_into().unwrap_or(i64::MAX)
            ],
            |row| {
                let text: String = row.get(5)?;
                Ok(SearchHit {
                    verse_id: row.get(0)?,
                    book_abbrev: row.get(1)?,
                    book_name: row.get(2)?,
                    chapter: row.get(3)?,
                    verse: row.get(4)?,
                    snippet: text,
                })
            },
        )?;
        rows.collect::<std::result::Result<_, _>>()
            .map_err(Into::into)
    }
}

/// Terms used for FTS and UI highlighting (alphanumeric chunks, min length 2).
pub fn search_terms(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .filter_map(sanitize_fts_token)
        .filter(|t| t.len() >= 2)
        .collect()
}

fn sanitize_fts_token(token: &str) -> Option<String> {
    let cleaned: String = token.chars().filter(|c| c.is_alphanumeric()).collect();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

/// Build an FTS5 query with prefix matching on each term (`term*`).
fn escape_fts_query(query: &str) -> String {
    let terms = search_terms(query);
    if terms.is_empty() {
        return String::new();
    }
    if terms.len() == 1 {
        format!("{}*", terms[0])
    } else {
        terms
            .iter()
            .map(|t| format!("{t}*"))
            .collect::<Vec<_>>()
            .join(" AND ")
    }
}

#[cfg(test)]
mod tests {
    use super::{escape_fts_query, search_terms};

    #[test]
    fn prefix_single_term() {
        assert_eq!(escape_fts_query("begin"), "begin*");
    }

    #[test]
    fn prefix_multiple_terms() {
        assert_eq!(escape_fts_query("word God"), "word* AND God*");
    }

    #[test]
    fn search_terms_skips_single_char_tokens() {
        assert_eq!(search_terms("a of God"), vec!["of", "God"]);
    }
}
