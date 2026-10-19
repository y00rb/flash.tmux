use crate::{
    config::Config,
    model::{LabeledMatch, Match, ScreenPoint, Viewport},
    search::{assign_labels, find_matches},
};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Debug)]
pub enum Input {
    Text(String),
    Backspace,
    Enter,
    Escape,
    Interrupt,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Continue,
    Cancel,
    Select(ScreenPoint),
}

pub struct SearchState {
    pub query: String,
    pub origin: ScreenPoint,
    pub matches: Vec<Match>,
    pub labels: Vec<LabeledMatch>,
}
impl SearchState {
    pub fn new(origin: ScreenPoint) -> Self {
        Self {
            query: String::new(),
            origin,
            matches: vec![],
            labels: vec![],
        }
    }
    pub fn reduce(&mut self, input: Input, view: &Viewport, config: &Config) -> Action {
        match input {
            Input::Escape | Input::Interrupt => return Action::Cancel,
            Input::Enter => {
                return self
                    .matches
                    .first()
                    .map_or(Action::Continue, |m| Action::Select(m.point));
            }
            Input::Backspace => {
                if let Some((index, _)) = self.query.grapheme_indices(true).next_back() {
                    self.query.truncate(index);
                }
            }
            Input::Text(text) => {
                for ch in text.chars().filter(|ch| !ch.is_control()) {
                    if let Some(target) = self.labels.iter().find(|label| label.label == ch) {
                        return Action::Select(target.target.point);
                    }
                    self.query.push(ch);
                    self.refresh(view, config);
                }
            }
        }
        self.refresh(view, config);
        Action::Continue
    }
    fn refresh(&mut self, view: &Viewport, config: &Config) {
        self.matches = find_matches(view, &self.query, self.origin);
        self.labels = assign_labels(view, &self.query, &self.matches, &config.labels);
    }
}
