use crate::{
    Result,
    config::Config,
    input::SearchState,
    model::{Match, Viewport, cell_width},
    render::render,
    syntax_input::SyntaxState,
};

pub fn render_syntax(view: &Viewport, state: &SyntaxState, config: &Config) -> Result<String> {
    let mut search = SearchState::new(crate::model::ScreenPoint::default());
    search.matches = state.hits.clone();
    if let Some(target) = state.current.and_then(|index| state.targets.get(index)) {
        for row in target.range.start.row..=target.range.end.row {
            let start = if row == target.range.start.row {
                target.range.start.cell
            } else {
                0
            };
            let end = if row == target.range.end.row {
                target.range.end.cell + 1
            } else {
                cell_width(&view.rows[row])
            };
            search.matches.push(Match {
                point: crate::model::ScreenPoint { row, cell: start },
                start: 0,
                end: 0,
                end_cell: end,
            });
        }
    }
    let mut occupied = std::collections::HashSet::new();
    for target in &state.targets {
        let Some(label) = target.label else {
            continue;
        };
        let candidates = [
            crate::model::ScreenPoint {
                row: target.range.start.row,
                cell: target.range.start.cell.saturating_sub(1),
            },
            crate::model::ScreenPoint {
                row: target.range.end.row,
                cell: target.range.end.cell + 1,
            },
        ];
        for point in candidates {
            if point.cell >= view.width || occupied.contains(&(point.row, point.cell)) {
                continue;
            }
            let mut cell = 0;
            let mut safe = false;
            for grapheme in unicode_segmentation::UnicodeSegmentation::graphemes(
                view.rows[point.row].as_str(),
                true,
            ) {
                let width = crate::model::grapheme_width(grapheme, cell);
                if cell == point.cell && width == 1 {
                    safe = true;
                    break;
                }
                cell += width;
            }
            safe |= point.cell >= cell;
            if !safe {
                continue;
            }
            occupied.insert((point.row, point.cell));
            search.labels.push(crate::model::LabeledMatch {
                label,
                position: point,
                target: Match {
                    point: target.range.start,
                    start: 0,
                    end: 0,
                    end_cell: target.range.end.cell + 1,
                },
            });
            break;
        }
    }
    // Coincident boundaries also have prompt legend entries; source cells never shift.
    render(view, &search, config)
}

pub fn syntax_prompt(state: &SyntaxState, width: usize) -> String {
    let current = state
        .current
        .and_then(|index| state.targets.get(index))
        .map_or("no node", |target| target.node.kind.as_str());
    let mut text = format!("TS {} {} [{}]", state.language.name(), state.query, current);
    for target in &state.targets {
        if let Some(label) = target.label {
            text.push_str(&format!(" {label}:{}", target.node.kind));
        }
    }
    text.chars()
        .filter(|ch| !ch.is_control())
        .take(width)
        .collect()
}
