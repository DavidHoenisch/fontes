use crate::Chapter;

/// Plain verse text for export (no markup).
pub fn verse_plain_text(text: &str) -> &str {
    text.trim()
}

/// Clipboard text with full reference and KJV body for one or more verses in a chapter.
pub fn format_verses_clipboard(chapter: &Chapter, start_index: usize, end_index: usize) -> String {
    let verses = &chapter.verses[start_index..=end_index];
    let book = chapter.book.name.as_str();
    let ch = chapter.chapter;
    let v_start = verses[0].reference.verse;
    let v_end = verses[verses.len() - 1].reference.verse;

    let reference = if v_start == v_end {
        format!("{book} {ch}:{v_start}")
    } else {
        format!("{book} {ch}:{v_start}-{v_end}")
    };
    let header = format!("{reference} (KJV)");

    if verses.len() == 1 {
        return format!("{header} {}", verse_plain_text(&verses[0].text));
    }

    let mut out = format!("{header}\n");
    for verse in verses {
        out.push_str(&format!(
            "{} {}\n",
            verse.reference.verse,
            verse_plain_text(&verse.text)
        ));
    }
    out.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Book, Verse, VerseRef};

    fn sample_chapter() -> Chapter {
        Chapter {
            book: Book {
                id: 43,
                osis: "John".into(),
                abbrev: "Jhn".into(),
                name: "John".into(),
                testament: "NT".into(),
                sort_order: 43,
            },
            chapter: 3,
            translation_id: 1,
            verses: vec![
                Verse {
                    id: 43_003_016,
                    reference: VerseRef {
                        book_id: 43,
                        chapter: 3,
                        verse: 16,
                    },
                    text: "For God so loved the world.".into(),
                    tokens: vec![],
                },
                Verse {
                    id: 43_003_017,
                    reference: VerseRef {
                        book_id: 43,
                        chapter: 3,
                        verse: 17,
                    },
                    text: "For God sent not his Son.".into(),
                    tokens: vec![],
                },
            ],
        }
    }

    #[test]
    fn formats_single_verse() {
        let ch = sample_chapter();
        let text = format_verses_clipboard(&ch, 0, 0);
        assert_eq!(text, "John 3:16 (KJV) For God so loved the world.");
    }

    #[test]
    fn formats_verse_range() {
        let ch = sample_chapter();
        let text = format_verses_clipboard(&ch, 0, 1);
        assert!(text.starts_with("John 3:16-17 (KJV)\n"));
        assert!(text.contains("16 For God so loved"));
        assert!(text.contains("17 For God sent not"));
    }
}
