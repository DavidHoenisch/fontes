use fontes_core::{Annotation, AnnotationKind, NoteAnchor};

pub fn token_has_note(anchors: &[NoteAnchor], token_idx: i32) -> bool {
    anchors.iter().any(|a| anchor_covers_token(a, token_idx))
}

pub fn token_annotation_kind(annotations: &[Annotation], token_idx: i32) -> Option<AnnotationKind> {
    annotations
        .iter()
        .find(|a| token_idx >= a.start_token && token_idx < a.end_token)
        .map(|a| a.kind)
}

fn anchor_covers_token(anchor: &NoteAnchor, token_idx: i32) -> bool {
    match (anchor.start_token, anchor.end_token) {
        (Some(start), Some(end)) => token_idx >= start && token_idx < end,
        _ => false,
    }
}

pub fn token_in_pending_selection(
    selection_anchor: Option<(usize, usize)>,
    verse_index: usize,
    cursor_verse_index: usize,
    cursor_token_index: usize,
    token_ti: usize,
) -> bool {
    let (anchor_verse, anchor_ti) = match selection_anchor {
        Some(pair) => pair,
        None => return false,
    };
    if anchor_verse != verse_index || anchor_verse != cursor_verse_index {
        return false;
    }
    let lo = anchor_ti.min(cursor_token_index);
    let hi = anchor_ti.max(cursor_token_index);
    token_ti >= lo && token_ti <= hi
}

pub fn note_id_at_token(anchors: &[NoteAnchor], token_idx: i32) -> Option<i64> {
    anchors
        .iter()
        .find(|a| anchor_covers_token(a, token_idx))
        .map(|a| a.note_id)
}
