use crate::{
    Result,
    config::Config,
    input::SearchState,
    model::{Viewport, grapheme_width},
};
use unicode_segmentation::UnicodeSegmentation;

fn color(value: &str, background: bool) -> Result<String> {
    if value.is_empty() {
        return Ok(format!("\x1b[{}m", if background { 49 } else { 39 }));
    }
    let r = u8::from_str_radix(&value[1..3], 16)?;
    let g = u8::from_str_radix(&value[3..5], 16)?;
    let b = u8::from_str_radix(&value[5..7], 16)?;
    Ok(format!(
        "\x1b[{};2;{r};{g};{b}m",
        if background { 48 } else { 38 }
    ))
}

pub fn render(view: &Viewport, state: &SearchState, config: &Config) -> Result<String> {
    let label = format!(
        "\x1b[0;1m{}{}",
        color(&config.label_fg, false)?,
        color(&config.label_bg, true)?
    );
    let matched = format!(
        "\x1b[0m{}{}",
        color(&config.match_fg, false)?,
        color(&config.match_bg, true)?
    );
    // Match foregrounds may be dark on a bright highlight; never reuse them
    // for unhighlighted text on the terminal's default background.
    let backdrop = "\x1b[0;2;39m";
    let mut out = String::from("\x1b[?25l\x1b[?7l\x1b[H");
    for (row, text) in view.rows.iter().enumerate() {
        out.push_str(&format!("\x1b[{};1H\x1b[0m\x1b[2K", row + 1));
        let mut cell = 0;
        for grapheme in text.graphemes(true) {
            let width = grapheme_width(grapheme, cell);
            if cell + width > view.width {
                break;
            }
            if let Some(l) = state
                .labels
                .iter()
                .find(|l| l.position.row == row && l.position.cell == cell)
            {
                out.push_str(&label);
                out.push(l.label);
            } else {
                let is_match = state
                    .matches
                    .iter()
                    .any(|m| m.point.row == row && cell >= m.point.cell && cell < m.end_cell);
                out.push_str(if is_match { &matched } else { backdrop });
                if grapheme == "\t" {
                    out.push_str(&" ".repeat(width));
                } else {
                    for ch in grapheme.chars() {
                        out.push(if ch.is_control() { '?' } else { ch });
                    }
                }
            }
            cell += width;
        }
        // End-of-line labels can be placed in otherwise empty cells.
        for l in state
            .labels
            .iter()
            .filter(|l| l.position.row == row && l.position.cell >= cell)
        {
            out.push_str(&format!(
                "\x1b[{};{}H{}{}",
                row + 1,
                l.position.cell + 1,
                label,
                l.label
            ));
        }
    }
    out.push_str("\x1b[0m");
    Ok(out)
}
