use crate::{
    Result,
    config::Config,
    input::{Action, Input, SearchState},
    model::{Outcome, ScreenPoint, Viewport},
    render::render,
    tmux,
};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal,
};
use std::{
    io::{Write, stdout},
    time::Duration,
};

pub fn run_syntax(
    capture: &crate::syntax::document::CapturedDocument,
    config: &Config,
    mode: crate::model::InteractionMode,
    snapshot: &tmux::Snapshot,
) -> Result<Outcome> {
    use crate::{
        syntax::{
            PARSE_BUDGET,
            detect::{resolve_at, resolve_selection_at},
            document::SourceDocument,
        },
        syntax_input::{SyntaxAction, SyntaxState},
        syntax_render::{render_syntax, syntax_prompt},
    };
    let origin = snapshot.cursor;
    let client = &snapshot.client;
    let prompt_width = snapshot.client_width;
    let source = SourceDocument::from_capture(capture)?;
    let parsed = if mode == crate::model::InteractionMode::Treesitter {
        resolve_selection_at(&source, config.language, origin, PARSE_BUDGET)?
    } else {
        resolve_at(&source, config.language, origin, PARSE_BUDGET)?
    };
    let mut state = SyntaxState::new(mode, source, parsed, origin, config)?;
    terminal::enable_raw_mode()?;
    let _guard = TerminalGuard;
    loop {
        write!(
            stdout(),
            "{}",
            render_syntax(&capture.view, &state, config)?
        )?;
        stdout().flush()?;
        tmux::message(client, &syntax_prompt(&state, prompt_width), config)?;
        let input = loop {
            // A fixed-size popup need not receive SIGWINCH when the outer
            // client grows, so also observe the original viewport while idle.
            if !event::poll(Duration::from_millis(150))? {
                if !tmux::validate_snapshot(snapshot)? {
                    return Ok(Outcome::Cancelled);
                }
                continue;
            }
            match event::read()? {
                Event::Resize(_, _) => return Ok(Outcome::Cancelled),
                Event::Paste(text) => break Input::Text(text),
                Event::Key(key) if key.kind != KeyEventKind::Release => match key.code {
                    KeyCode::Esc => break Input::Escape,
                    KeyCode::Char('c' | 'd') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        break Input::Interrupt;
                    }
                    KeyCode::Char(ch)
                        if !key
                            .modifiers
                            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                    {
                        break Input::Text(ch.into());
                    }
                    KeyCode::Backspace => break Input::Backspace,
                    KeyCode::Enter => break Input::Enter,
                    _ => {}
                },
                _ => {}
            }
        };
        match state.reduce(input, config) {
            SyntaxAction::Cancel => return Ok(Outcome::Cancelled),
            SyntaxAction::Select(range) => return Ok(Outcome::RangeSelected { range }),
            SyntaxAction::Continue => {}
        }
    }
}

struct TerminalGuard;
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = write!(stdout(), "\x1b[0m\x1b[?7h\x1b[?25h");
        let _ = stdout().flush();
    }
}

pub fn run(view: &Viewport, origin: ScreenPoint, client: &str, config: &Config) -> Result<Outcome> {
    terminal::enable_raw_mode()?;
    let _guard = TerminalGuard;
    let mut state = SearchState::new(origin);
    let mut dirty = true;
    loop {
        if dirty {
            write!(stdout(), "{}", render(view, &state, config)?)?;
            stdout().flush()?;
            let prompt = format!(
                "\u{26a1} {}  [{} matches / {} labels]",
                state.query,
                state.matches.len(),
                state.labels.len()
            );
            tmux::message(client, &prompt, config)?;
            dirty = false;
        }
        if !event::poll(Duration::from_millis(150))? {
            continue;
        }
        let input = match event::read()? {
            Event::Resize(_, _) => return Ok(Outcome::Cancelled),
            Event::Key(key) if key.kind != KeyEventKind::Release => match key.code {
                KeyCode::Esc => Some(Input::Escape),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    Some(Input::Interrupt)
                }
                KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    Some(Input::Interrupt)
                }
                KeyCode::Char(ch)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    Some(Input::Text(ch.into()))
                }
                KeyCode::Backspace => Some(Input::Backspace),
                KeyCode::Enter => Some(Input::Enter),
                _ => None,
            },
            Event::Paste(text) => Some(Input::Text(text)),
            _ => None,
        };
        if let Some(input) = input {
            match state.reduce(input, view, config) {
                Action::Cancel => return Ok(Outcome::Cancelled),
                Action::Select(point) => return Ok(Outcome::Selected { point }),
                Action::Continue => dirty = true,
            }
        }
    }
}
