use crate::{
    Result,
    model::{ScreenPoint, ScreenRange, Viewport, grapheme_width},
};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

// -J preserves trailing cells even for a single-row capture. Comparing the
// same row with a two-row joined capture exposes tmux's actual wrap flag.
pub fn decode_capture_pair(single: &str, pair: &str) -> Result<(String, bool)> {
    let row = single
        .strip_suffix('\n')
        .ok_or("incomplete syntax row capture")?;
    if row.contains('\n') || !pair.ends_with('\n') || !pair.starts_with(row) {
        return Err("syntax rows changed during capture".into());
    }
    let remainder = &pair[row.len()..];
    match remainder.bytes().filter(|ch| *ch == b'\n').count() {
        1 => Ok((row.into(), true)),
        2 if remainder.starts_with('\n') => Ok((row.into(), false)),
        _ => Err("ambiguous syntax wrap metadata".into()),
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CapturedDocument {
    pub view: Viewport,
    pub wraps: Vec<bool>,
}

struct SourceCell {
    bytes: Range<usize>,
    point: ScreenPoint,
    width: usize,
}

pub struct SourceDocument {
    view: Viewport,
    text: String,
    cells: Vec<SourceCell>,
    row_starts: Vec<usize>,
}

impl SourceDocument {
    pub fn from_capture(capture: &CapturedDocument) -> Result<Self> {
        let rows = &capture.view.rows;
        if capture.wraps.len() != rows.len() || capture.wraps.last() == Some(&true) {
            return Err("invalid or clipped syntax wrap metadata".into());
        }
        let mut text = String::new();
        let mut cells = Vec::new();
        let mut row_starts = Vec::with_capacity(rows.len());
        for (row, line) in rows.iter().enumerate() {
            row_starts.push(text.len());
            let mut cell = 0;
            for grapheme in line.graphemes(true) {
                if grapheme.chars().any(|ch| ch.is_control() && ch != '\t') {
                    return Err("unsupported control character in syntax capture".into());
                }
                let width = grapheme_width(grapheme, cell);
                if cell + width > capture.view.width {
                    return Err("syntax capture extends past viewport".into());
                }
                let start = text.len();
                text.push_str(grapheme);
                cells.push(SourceCell {
                    bytes: start..text.len(),
                    point: ScreenPoint { row, cell },
                    width,
                });
                cell += width;
            }
            if row + 1 < rows.len() && !capture.wraps[row] {
                text.push('\n');
            }
        }
        if text.len() > super::MAX_SOURCE_BYTES {
            return Err("syntax source exceeds 256 KiB".into());
        }
        Ok(Self {
            view: capture.view.clone(),
            text,
            cells,
            row_starts,
        })
    }

    pub fn view(&self) -> &Viewport {
        &self.view
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn byte_at(&self, point: ScreenPoint) -> Option<usize> {
        self.cells
            .iter()
            .find(|cell| {
                cell.point.row == point.row
                    && point.cell >= cell.point.cell
                    && point.cell < cell.point.cell + cell.width
            })
            .map(|cell| cell.bytes.start)
    }

    pub fn cursor_byte_at(&self, point: ScreenPoint) -> Option<usize> {
        if point.cell >= self.view.width {
            return None;
        }
        self.byte_at(point).or_else(|| {
            let start = *self.row_starts.get(point.row)?;
            let end = start + self.view.rows[point.row].len();
            // Empty lines and screen padding have no grapheme cell. Their
            // hard newline still has a byte position inside enclosing nodes.
            (self.text.as_bytes().get(end) == Some(&b'\n')).then_some(end)
        })
    }

    pub fn screen_range(&self, bytes: Range<usize>) -> Option<ScreenRange> {
        if bytes.is_empty()
            || bytes.end > self.text.len()
            || !self.text.is_char_boundary(bytes.start)
            || !self.text.is_char_boundary(bytes.end)
        {
            return None;
        }
        let mut included = self
            .cells
            .iter()
            .filter(|cell| cell.bytes.end > bytes.start && cell.bytes.start < bytes.end);
        let first = included.next()?;
        let last = included.next_back().unwrap_or(first);
        Some(ScreenRange {
            start: first.point,
            end: last.point,
        })
    }
}
