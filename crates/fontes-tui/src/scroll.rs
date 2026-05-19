use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Wrap};

/// Wrapped row layout per verse. When `gap_after_verse` is true, each verse except the
/// last is followed by one blank display row in the paragraph.
pub fn verse_layout(lines: &[Line], width: u16, gap_after_verse: bool) -> (Vec<usize>, Vec<usize>) {
    let mut starts = Vec::with_capacity(lines.len());
    let mut heights = Vec::with_capacity(lines.len());
    let mut acc = 0usize;
    let n = lines.len();
    for (i, line) in lines.iter().enumerate() {
        starts.push(acc);
        let wrapped = wrapped_line_count(line, width);
        let gap = usize::from(gap_after_verse && i + 1 < n);
        heights.push(wrapped + gap);
        acc += wrapped + gap;
    }
    (starts, heights)
}

pub fn wrapped_line_count(line: &Line, width: u16) -> usize {
    if width < 1 {
        return 1;
    }
    Paragraph::new(line.clone())
        .wrap(Wrap { trim: false })
        .line_count(width)
        .max(1)
}

/// Keep the verse at `verse_index` within the viewport (handles multi-line wraps).
pub fn ensure_verse_visible(
    line_starts: &[usize],
    line_heights: &[usize],
    verse_index: usize,
    scroll_top: usize,
    viewport_height: usize,
) -> usize {
    let viewport = viewport_height.max(1);
    let Some(&start) = line_starts.get(verse_index) else {
        return scroll_top;
    };
    let height = line_heights.get(verse_index).copied().unwrap_or(1).max(1);
    let end = start + height - 1;

    if height >= viewport {
        return start;
    }
    if start < scroll_top {
        start
    } else if end >= scroll_top + viewport {
        end + 1 - viewport
    } else {
        scroll_top
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pins_tall_verse_to_top() {
        let starts = vec![0, 5, 25];
        let heights = vec![5, 20, 3];
        assert_eq!(ensure_verse_visible(&starts, &heights, 1, 0, 10), 5);
    }

    #[test]
    fn scrolls_up_when_verse_above_viewport() {
        let starts = vec![0, 15, 30];
        let heights = vec![15, 15, 5];
        assert_eq!(ensure_verse_visible(&starts, &heights, 0, 10, 10), 0);
    }

    #[test]
    fn keeps_scroll_when_verse_fits() {
        let starts = vec![0, 2, 12];
        let heights = vec![2, 5, 5];
        assert_eq!(ensure_verse_visible(&starts, &heights, 1, 0, 10), 0);
    }

    #[test]
    fn scrolls_down_when_verse_below_viewport() {
        let starts = vec![0, 2, 20];
        let heights = vec![2, 5, 5];
        assert_eq!(ensure_verse_visible(&starts, &heights, 2, 0, 10), 15);
    }
}
