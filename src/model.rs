use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenPoint {
    pub row: usize,
    pub cell: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenRange {
    pub start: ScreenPoint,
    pub end: ScreenPoint,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Viewport {
    pub rows: Vec<String>,
    pub width: usize,
}
impl Viewport {
    pub fn new(rows: Vec<String>, width: usize) -> Self {
        Self { rows, width }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Match {
    pub point: ScreenPoint,
    pub start: usize,
    pub end: usize,
    pub end_cell: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LabeledMatch {
    pub label: char,
    pub target: Match,
    pub position: ScreenPoint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InteractionMode {
    Search,
    Treesitter,
    TreesitterSearch,
}
impl std::str::FromStr for InteractionMode {
    type Err = crate::Error;
    fn from_str(value: &str) -> crate::Result<Self> {
        match value {
            "search" => Ok(Self::Search),
            "treesitter" => Ok(Self::Treesitter),
            "treesitter-search" => Ok(Self::TreesitterSearch),
            _ => Err("mode must be search, treesitter, or treesitter-search".into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Cancelled,
    Selected { point: ScreenPoint },
    RangeSelected { range: ScreenRange },
}

pub fn grapheme_width(text: &str, column: usize) -> usize {
    if text == "\t" {
        8 - column % 8
    } else {
        UnicodeWidthStr::width(text)
    }
}

pub fn cell_width(text: &str) -> usize {
    text.graphemes(true)
        .fold(0, |column, g| column + grapheme_width(g, column))
}

// tmux's cursor-right skips the continuation cells of wide characters.
pub fn motion_count(line: &str, cell: usize) -> usize {
    let mut column = 0;
    let mut motions = 0;
    for grapheme in line.graphemes(true) {
        if column >= cell {
            break;
        }
        column += grapheme_width(grapheme, column);
        motions += 1;
    }
    motions + cell.saturating_sub(column)
}
