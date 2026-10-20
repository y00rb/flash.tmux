use crate::{
    Result,
    config::{Config, DEFAULTS},
    model::{ScreenPoint, Viewport, motion_count},
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, process::Command};

pub fn command(args: &[&str]) -> Result<String> {
    let output = Command::new("tmux").args(args).output()?;
    if !output.status.success() {
        return Err(format!(
            "tmux {}: {}",
            args.first().unwrap_or(&""),
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

pub fn config() -> Result<Config> {
    let mut options = BTreeMap::new();
    for (name, _) in DEFAULTS
        .into_iter()
        .chain([("flash-treesitter-language", "auto")])
    {
        let option = format!("@{name}");
        if !command(&["show-option", "-gq", &option])?.is_empty() {
            options.insert(
                name.into(),
                command(&["show-option", "-gqv", &option])?
                    .trim_end_matches('\n')
                    .into(),
            );
        }
    }
    Config::from_options(&options)
}

pub fn config_for_pane(pane: &str) -> Result<Config> {
    let mut config = config()?;
    let local = command(&[
        "show-options",
        "-p",
        "-qv",
        "-t",
        pane,
        "@flash-treesitter-language",
    ])?;
    if !local.is_empty() {
        config.language = local.trim_end_matches('\n').parse()?;
    }
    Ok(config)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: u32,
    pub pane: String,
    pub client: String,
    pub viewport: Viewport,
    pub cursor: ScreenPoint,
    pub state: String,
    pub left: usize,
    pub top: usize,
    pub client_width: usize,
    pub client_height: usize,
}

const STATE: &str = "#{pane_id};#{pane_mode};#{scroll_position};#{copy_cursor_x};#{copy_cursor_y};#{pane_width};#{pane_height};#{selection_present};#{selection_start_x};#{selection_start_y};#{selection_end_x};#{selection_end_y};#{pane_left};#{pane_top};#{window_id};#{session_id};#{rectangle_toggle}";

pub fn snapshot(pane: &str, client: &str) -> Result<Snapshot> {
    if !pane.starts_with('%') || !pane[1..].bytes().all(|b| b.is_ascii_digit()) {
        return Err("invalid pane ID".into());
    }
    let state = command(&["display-message", "-p", "-t", pane, STATE])?
        .trim_end_matches('\n')
        .to_owned();
    let parts: Vec<&str> = state.split(';').collect();
    if parts.len() != 17 || parts[1] != "copy-mode" {
        return Err("target is not in copy-mode".into());
    }
    let number = |i: usize| -> Result<usize> { Ok(parts[i].parse()?) };
    let width = number(5)?;
    let height = number(6)?;
    let scroll: i64 = parts[2].parse().unwrap_or(0);
    let start = (-scroll).to_string();
    let end = (height as i64 - scroll - 1).to_string();
    let text = command(&["capture-pane", "-p", "-t", pane, "-S", &start, "-E", &end])?;
    let mut rows: Vec<String> = text
        .trim_end_matches('\n')
        .split('\n')
        .map(str::to_owned)
        .collect();
    rows.resize(height, String::new());
    rows.truncate(height);
    let dimensions = command(&[
        "display-message",
        "-p",
        "-c",
        client,
        "#{client_width};#{client_height};#{client_session}",
    ])?;
    let dimensions: Vec<&str> = dimensions.trim().split(';').collect();
    if dimensions.len() != 3 || dimensions[2].is_empty() {
        return Err("target client is not attached".into());
    }
    let status = command(&[
        "display-message",
        "-p",
        "-c",
        client,
        "#{status};#{status-position}",
    ])?;
    let status_parts: Vec<&str> = status.trim().split(';').collect();
    let status_height = match status_parts[0] {
        "on" => 1,
        "off" => 0,
        value => value.parse::<usize>()?,
    };
    if status_height == 0 {
        return Err(
            "a visible tmux status line is required for the search prompt (set -g status on)"
                .into(),
        );
    }
    let top_offset = if status_parts.get(1) == Some(&"top") {
        status_height
    } else {
        0
    };
    let snapshot = Snapshot {
        version: 1,
        pane: pane.into(),
        client: client.into(),
        viewport: Viewport::new(rows, width),
        cursor: ScreenPoint {
            cell: number(3)?,
            row: number(4)?,
        },
        left: number(12)?,
        top: number(13)? + top_offset,
        client_width: dimensions[0].parse()?,
        client_height: dimensions[1].parse()?,
        state: state.clone(),
    };
    if state != command(&["display-message", "-p", "-t", pane, STATE])?.trim_end_matches('\n') {
        return Err("copy-mode state changed during capture; try again".into());
    }
    Ok(snapshot)
}

pub fn validate_snapshot(original: &Snapshot) -> Result<bool> {
    Ok(snapshot(&original.pane, &original.client).is_ok_and(|current| current == *original))
}

pub fn syntax_capture(original: &Snapshot) -> Result<crate::syntax::document::CapturedDocument> {
    use crate::syntax::document::{CapturedDocument, decode_capture_pair};
    if !validate_snapshot(original)? {
        return Err("copy-mode viewport changed; syntax capture cancelled".into());
    }
    let parts: Vec<_> = original.state.split(';').collect();
    let scroll: i64 = parts[2].parse()?;
    let height = original.viewport.rows.len();
    let history: i64 = command(&[
        "display-message",
        "-p",
        "-t",
        &original.pane,
        "#{history_size}",
    ])?
    .trim()
    .parse()?;
    let capture = |first: i64, last: i64| {
        command(&[
            "capture-pane",
            "-p",
            "-J",
            "-t",
            &original.pane,
            "-S",
            &first.to_string(),
            "-E",
            &last.to_string(),
        ])
    };
    if -scroll > -history {
        let previous = capture(-scroll - 1, -scroll - 1)?;
        let pair = capture(-scroll - 1, -scroll)?;
        if decode_capture_pair(&previous, &pair)?.1 {
            return Err("syntax viewport starts inside a wrapped line; scroll to its start".into());
        }
    }
    let mut rows = Vec::with_capacity(height);
    let mut wraps = Vec::with_capacity(height);
    for index in 0..height {
        let absolute = index as i64 - scroll;
        let single = capture(absolute, absolute)?;
        let (row, wrapped) = if absolute + 1 < height as i64 {
            decode_capture_pair(&single, &capture(absolute, absolute + 1)?)?
        } else {
            (
                single
                    .strip_suffix('\n')
                    .ok_or("incomplete syntax capture")?
                    .into(),
                false,
            )
        };
        if row.trim_end_matches(' ') != original.viewport.rows[index].trim_end_matches(' ') {
            return Err("syntax rows changed during capture".into());
        }
        rows.push(row);
        wraps.push(wrapped);
    }
    if wraps.last() == Some(&true) {
        return Err("syntax viewport ends inside a wrapped line; scroll to include its end".into());
    }
    if !validate_snapshot(original)? {
        return Err("copy-mode viewport changed during syntax capture".into());
    }
    Ok(CapturedDocument {
        view: Viewport::new(rows, original.viewport.width),
        wraps,
    })
}

pub fn apply_jump(original: &Snapshot, point: ScreenPoint) -> Result<()> {
    if point.row >= original.viewport.rows.len() || point.cell >= original.viewport.width {
        return Err("selected coordinate outside viewport".into());
    }
    if !validate_snapshot(original)? {
        return Err("copy-mode viewport changed; jump cancelled".into());
    }
    let mut args = vec![
        "send-keys".to_string(),
        "-t".into(),
        original.pane.clone(),
        "-X".into(),
        "top-line".into(),
        ";".into(),
        "send-keys".into(),
        "-t".into(),
        original.pane.clone(),
        "-X".into(),
        "start-of-line".into(),
    ];
    if point.row > 0 {
        args.extend([
            ";".into(),
            "send-keys".into(),
            "-t".into(),
            original.pane.clone(),
            "-X".into(),
            "-N".into(),
            point.row.to_string(),
            "cursor-down".into(),
            ";".into(),
            "send-keys".into(),
            "-t".into(),
            original.pane.clone(),
            "-X".into(),
            "start-of-line".into(),
        ]);
    }
    let motions = motion_count(&original.viewport.rows[point.row], point.cell);
    if motions > 0 {
        args.extend([
            ";".into(),
            "send-keys".into(),
            "-t".into(),
            original.pane.clone(),
            "-X".into(),
            "-N".into(),
            motions.to_string(),
            "cursor-right".into(),
        ]);
    }
    command(&args.iter().map(String::as_str).collect::<Vec<_>>())?;
    Ok(())
}

pub fn message(client: &str, text: &str, config: &Config) -> Result<()> {
    // display-message interprets tmux formats, so escape # supplied by the query.
    let safe = text.replace('#', "##").replace(['\n', '\r', '\x1b'], " ");
    let style = format!(
        "#[fg={},bg={}]{}",
        config.prompt_fg,
        if config.prompt_bg.is_empty() {
            "default"
        } else {
            &config.prompt_bg
        },
        safe
    );
    command(&["display-message", "-c", client, "-d", "0", &style])?;
    Ok(())
}
