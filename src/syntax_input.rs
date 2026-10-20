use crate::{
    Result,
    config::Config,
    input::Input,
    model::{InteractionMode, Match, ScreenPoint, ScreenRange},
    search::find_matches,
    syntax::{LanguageId, ParsedSyntax, SyntaxNode, document::SourceDocument},
};
use std::collections::HashSet;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SyntaxAction {
    Continue,
    Cancel,
    Select(ScreenRange),
}

pub struct RangeTarget {
    pub node: SyntaxNode,
    pub range: ScreenRange,
    pub label: Option<char>,
    pub label_position: Option<ScreenPoint>,
}

pub struct SyntaxState {
    pub query: String,
    pub targets: Vec<RangeTarget>,
    pub current: Option<usize>,
    pub language: LanguageId,
    pub hits: Vec<Match>,
    pub mode: InteractionMode,
    source: SourceDocument,
    parsed: ParsedSyntax,
    origin: ScreenPoint,
}

impl SyntaxState {
    pub fn new(
        mode: InteractionMode,
        source: SourceDocument,
        parsed: ParsedSyntax,
        origin: ScreenPoint,
        config: &Config,
    ) -> Result<Self> {
        if mode == InteractionMode::Search {
            return Err("syntax state requires a syntax mode".into());
        }
        let mut state = Self {
            query: String::new(),
            targets: vec![],
            current: None,
            language: parsed.language,
            hits: vec![],
            mode,
            source,
            parsed,
            origin,
        };
        state.refresh(config);
        Ok(state)
    }

    pub fn view(&self) -> &crate::model::Viewport {
        self.source.view()
    }

    pub fn reduce(&mut self, input: Input, config: &Config) -> SyntaxAction {
        match input {
            Input::Escape | Input::Interrupt => return SyntaxAction::Cancel,
            Input::Enter => {
                return self
                    .current
                    .and_then(|index| self.targets.get(index))
                    .map_or(SyntaxAction::Continue, |target| {
                        SyntaxAction::Select(target.range)
                    });
            }
            Input::Backspace if self.mode == InteractionMode::TreesitterSearch => {
                if let Some((offset, _)) = self.query.grapheme_indices(true).next_back() {
                    self.query.truncate(offset);
                }
                self.refresh(config);
            }
            Input::Text(text) => {
                for ch in text.chars().filter(|ch| !ch.is_control()) {
                    if self.mode == InteractionMode::Treesitter && matches!(ch, ';' | ',') {
                        if let Some(current) = self.current {
                            let count = self.targets.len();
                            self.current = Some(if ch == ';' {
                                (current + 1) % count
                            } else {
                                (current + count - 1) % count
                            });
                        }
                        continue;
                    }
                    if let Some(target) =
                        self.targets.iter().find(|target| target.label == Some(ch))
                    {
                        return SyntaxAction::Select(target.range);
                    }
                    if self.mode == InteractionMode::TreesitterSearch {
                        self.query.push(ch);
                        self.refresh(config);
                    }
                }
            }
            _ => {}
        }
        SyntaxAction::Continue
    }

    fn refresh(&mut self, config: &Config) {
        let old_labels: Vec<_> = self
            .targets
            .iter()
            .filter_map(|target| target.label.map(|label| (target.node.bytes.clone(), label)))
            .collect();
        let mut nodes = vec![];
        if self.mode == InteractionMode::Treesitter {
            if let Some(byte) = self.source.byte_at(self.origin) {
                nodes = self.parsed.ancestors_at(byte);
            }
        } else {
            self.hits = find_matches(self.source.view(), &self.query, self.origin);
            for hit in &self.hits {
                if let Some(byte) = self.source.byte_at(hit.point) {
                    nodes.extend(self.parsed.ancestors_at(byte));
                }
            }
        }
        let mut seen = HashSet::new();
        self.targets = nodes
            .into_iter()
            .filter(|node| seen.insert((node.bytes.start, node.bytes.end)))
            .filter_map(|node| {
                self.source
                    .screen_range(node.bytes.clone())
                    .map(|range| RangeTarget {
                        node,
                        range,
                        label: None,
                        label_position: None,
                    })
            })
            .collect();
        self.current = (!self.targets.is_empty()).then_some(0);
        let insensitive = !self.query.chars().any(char::is_uppercase);
        let blocked: HashSet<char> = self
            .hits
            .iter()
            .filter_map(|hit| {
                self.source.view().rows[hit.point.row][hit.end..]
                    .chars()
                    .next()
            })
            .flat_map(|ch| {
                if insensitive {
                    ch.to_lowercase().collect::<Vec<_>>()
                } else {
                    vec![ch]
                }
            })
            .collect();
        let allowed: Vec<char> = config
            .labels
            .chars()
            .filter(|ch| {
                if self.mode == InteractionMode::Treesitter {
                    !matches!(ch, ';' | ',')
                } else {
                    !blocked.contains(&if insensitive {
                        ch.to_ascii_lowercase()
                    } else {
                        *ch
                    })
                }
            })
            .collect();
        let mut used = HashSet::new();
        // Reuse existing labels before allocating new ones, so later stable targets
        // cannot lose their labels to newly introduced earlier ranges.
        for target in &mut self.targets {
            if let Some((_, label)) = old_labels.iter().find(|(bytes, label)| {
                *bytes == target.node.bytes && allowed.contains(label) && !used.contains(label)
            }) {
                target.label = Some(*label);
                used.insert(*label);
            }
        }
        for target in &mut self.targets {
            if target.label.is_none()
                && let Some(label) = allowed.iter().find(|label| !used.contains(*label))
            {
                target.label = Some(*label);
                used.insert(*label);
            }
        }
    }
}
