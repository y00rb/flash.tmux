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
