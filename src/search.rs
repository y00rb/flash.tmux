use crate::model::{LabeledMatch, Match, ScreenPoint, Viewport, cell_width, grapheme_width};
use std::collections::HashSet;
use unicode_segmentation::UnicodeSegmentation;

pub fn find_matches(view: &Viewport, query: &str, origin: ScreenPoint) -> Vec<Match> {
    if query.is_empty() {
        return vec![];
    }
    let insensitive = !query.chars().any(char::is_uppercase);
    let needle = if insensitive {
        query.to_lowercase()
    } else {
        query.into()
    };
    let mut result = vec![];
    for (row, line) in view.rows.iter().enumerate() {
        // Keep transformed character boundaries tied to original UTF-8 boundaries.
        let mut haystack = String::new();
        let mut boundaries = vec![(0, 0)];
        for (offset, ch) in line.char_indices() {
            if insensitive {
                haystack.extend(ch.to_lowercase());
            } else {
                haystack.push(ch);
            }
            boundaries.push((haystack.len(), offset + ch.len_utf8()));
        }
        for (start_index, &(fold_start, start)) in boundaries.iter().enumerate() {
            if !haystack[fold_start..].starts_with(&needle) {
                continue;
            }
            if let Some(&(_, end)) = boundaries[start_index..]
                .iter()
                .find(|(fold, _)| *fold == fold_start + needle.len())
            {
                // A substring can start inside a combining/ZWJ cluster, but
                // the terminal cursor can only address the cluster's first cell.
                let graphemes: Vec<_> = line.grapheme_indices(true).collect();
                let screen_start = graphemes
                    .iter()
                    .rev()
                    .find(|(i, _)| *i <= start)
                    .map_or(start, |(i, _)| *i);
                let screen_end = graphemes
                    .iter()
                    .find(|(i, g)| *i < end && *i + g.len() >= end)
                    .map_or(end, |(i, g)| *i + g.len());
                let point = ScreenPoint {
                    row,
                    cell: cell_width(&line[..screen_start]),
                };
                if point.cell < view.width {
                    result.push(Match {
                        point,
                        start,
                        end,
                        end_cell: cell_width(&line[..screen_end]),
                    });
                }
            }
        }
    }
    result.sort_by_key(|m| {
        (
            m.point.row.abs_diff(origin.row),
            m.point.cell.abs_diff(origin.cell),
            m.point.row,
            m.point.cell,
        )
    });
    result
}

pub fn assign_labels(
    view: &Viewport,
    query: &str,
    matches: &[Match],
    alphabet: &str,
) -> Vec<LabeledMatch> {
    let insensitive = !query.chars().any(char::is_uppercase);
    let blocked: HashSet<char> = matches
        .iter()
        .filter_map(|m| view.rows[m.point.row][m.end..].chars().next())
        .flat_map(|ch| {
            if insensitive {
                ch.to_lowercase().collect::<Vec<_>>()
            } else {
                vec![ch]
            }
        })
        .collect();
    let mut available = alphabet.chars().filter(|ch| {
        if insensitive {
            !blocked.contains(&ch.to_ascii_lowercase())
        } else {
            !blocked.contains(ch)
        }
    });
    let mut occupied = HashSet::new();
    let mut result = vec![];
    for m in matches {
        // Prefer after-match labels, but use a safe single-cell position at edges.
        let line = &view.rows[m.point.row];
        let mut cells = vec![];
        let mut col = 0;
        for grapheme in line.graphemes(true) {
            let width = grapheme_width(grapheme, col);
            if width == 1 {
                cells.push(col);
            }
            col += width;
        }
        cells.extend(col..view.width);
        let position = [m.end_cell, m.point.cell.saturating_sub(1), m.point.cell]
            .into_iter()
            .find(|cell| {
                *cell < view.width
                    && cells.contains(cell)
                    && !occupied.contains(&(m.point.row, *cell))
            });
        if let Some(cell) = position {
            let Some(label) = available.next() else {
                break;
            };
            occupied.insert((m.point.row, cell));
            result.push(LabeledMatch {
                label,
                target: m.clone(),
                position: ScreenPoint {
                    row: m.point.row,
                    cell,
                },
            });
        }
    }
    result
}
