use fontes_core::{Database, KJV_TRANSLATION_ID, verse_id};

fn fixture_db() -> Database {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixtures = manifest_dir.join("../../data/fixtures");
    Database::open_fixture_dir(&fixtures).expect("open fixtures")
}

#[test]
fn loads_john_chapter_one() {
    let db = fixture_db();
    let ch = db.get_chapter_kjv("Jhn", 1).expect("chapter");
    assert_eq!(ch.book.abbrev, "Jhn");
    assert_eq!(ch.verses.len(), 51);
    let v1 = &ch.verses[0];
    assert!(v1.text.starts_with("In the beginning"));
    assert!(
        !v1.text.contains('<'),
        "verse text must not contain HTML from upstream KJV"
    );
    let word = v1.tokens.iter().find(|t| t.strong_key.as_deref() == Some("G3056"));
    assert!(word.is_some(), "expected Word tagged G3056");
}

#[test]
fn scripture_has_no_html_markup() {
    let db = fixture_db();
    for chapter in 1..=3 {
        let ch = db.get_chapter_kjv("Jhn", chapter).expect("chapter");
        for verse in &ch.verses {
            assert!(
                !verse.text.contains('<'),
                "verse {}:{} text contains HTML",
                chapter,
                verse.reference.verse
            );
            for token in &verse.tokens {
                assert!(
                    !token.surface.contains('<'),
                    "verse {}:{} token {:?} contains HTML",
                    chapter,
                    verse.reference.verse,
                    token.surface
                );
            }
        }
    }
}

#[test]
fn strong_lookup_and_occurrences() {
    let db = fixture_db();
    let entry = db.get_strong("G3056").expect("strong");
    assert_eq!(entry.lang, "greek");
    assert!(!entry.definition.is_empty());

    let count = db.count_occurrences("G3056", KJV_TRANSLATION_ID).expect("count");
    assert!(count >= 3);

    let occ = db
        .list_occurrences_kjv("G3056", 10, 0)
        .expect("occurrences");
    assert!(!occ.is_empty());
}

#[test]
fn fts_search() {
    let db = fixture_db();
    let hits = db
        .search_verses("beginning", KJV_TRANSLATION_ID, 5)
        .expect("search");
    assert!(!hits.is_empty());
    assert!(hits[0].snippet.to_lowercase().contains("beginning"));
}

#[test]
fn note_crud() {
    let db = fixture_db();
    let id = db
        .create_note(Some("Test"), "# Hello\n\nA **note**.")
        .expect("create");
    let note = db.get_note(id).expect("get");
    assert_eq!(note.body, "# Hello\n\nA **note**.");

    db.add_note_anchor_token_range(id, verse_id(43, 1, 1), 0, 3, None)
        .expect("anchor");

    let anchors = db
        .note_anchors_for_verses(&[verse_id(43, 1, 1)])
        .expect("anchors");
    assert_eq!(anchors.len(), 1);

    let listed = db.list_notes_for_display().expect("list for display");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].location, "Jhn 1:1");

    db.delete_note(id).expect("delete");
}
