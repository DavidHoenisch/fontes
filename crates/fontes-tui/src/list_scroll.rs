//! Keep ratatui [`ListState`] selection inside the visible viewport.

use ratatui::layout::Rect;
use ratatui::widgets::{Block, ListState};

/// Adjust `state.offset` so `state.selected()` stays within `visible_rows` lines.
pub fn ensure_list_visible(state: &mut ListState, item_count: usize, visible_rows: u16) {
    if item_count == 0 {
        state.select(None);
        return;
    }
    let view = visible_rows.max(1) as usize;
    let selected = state.selected().unwrap_or(0).min(item_count - 1);
    state.select(Some(selected));

    let mut offset = (*state.offset_mut()).min(item_count.saturating_sub(1));
    if selected < offset {
        offset = selected;
    } else if selected >= offset.saturating_add(view) {
        offset = selected + 1 - view;
    }
    *state.offset_mut() = offset;
}

pub fn inner_height(area: Rect, block: &Block<'_>) -> u16 {
    block.inner(area).height
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrolls_down_when_selection_below_viewport() {
        let mut state = ListState::default().with_selected(Some(8));
        ensure_list_visible(&mut state, 20, 5);
        assert_eq!(state.selected(), Some(8));
        assert_eq!(state.offset(), 4);
    }

    #[test]
    fn scrolls_up_when_selection_above_viewport() {
        let mut state = ListState::default().with_selected(Some(1)).with_offset(5);
        ensure_list_visible(&mut state, 20, 5);
        assert_eq!(state.selected(), Some(1));
        assert_eq!(state.offset(), 1);
    }
}
