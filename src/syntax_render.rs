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
    // Coincident nested boundaries remain unambiguous in the prompt legend;
    // no virtual cells are inserted into the captured source.
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
