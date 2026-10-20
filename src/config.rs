use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

pub const DEFAULT_LABELS: &str = "asdfghjklASDFHJKLqwertyuiopzxcvbnmQWERTYUIOPZXCVBNM";
pub const DEFAULTS: [(&str, &str); 10] = [
    ("flash-key", "s"),
    ("flash-labels", DEFAULT_LABELS),
    ("flash-label-bg", "#f4b8e4"),
    ("flash-label-fg", "#303446"),
    ("flash-match-fg", "#c6d0f5"),
    ("flash-match-bg", "#51576d"),
    ("flash-prompt-bg", "#8caaee"),
    ("flash-prompt-fg", "#303446"),
    ("flash-treesitter-key", "S"),
    ("flash-treesitter-search-key", "R"),
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub key: String,
    pub treesitter_key: String,
    pub treesitter_search_key: String,
    pub language: crate::syntax::LanguageChoice,
    pub labels: String,
    pub label_bg: String,
    pub label_fg: String,
    pub match_fg: String,
    pub match_bg: String,
    pub prompt_bg: String,
    pub prompt_fg: String,
}

impl Config {
    pub fn from_options(options: &BTreeMap<String, String>) -> Result<Self> {
        let values: Vec<String> = DEFAULTS
            .iter()
            .map(|(key, default)| {
                options
                    .get(*key)
                    .cloned()
                    .unwrap_or_else(|| default.to_string())
            })
            .collect();
        if [0, 8, 9]
            .iter()
            .any(|i| values[*i].is_empty() || values[*i].chars().any(char::is_control))
        {
            return Err("@flash-key must be a nonempty tmux key name".into());
        }
        let canonical = |key: &str| match key {
            "C-i" | "C-I" | "Tab" => "Tab".to_string(),
            "C-m" | "C-M" | "Enter" => "Enter".to_string(),
            "C-[" | "Escape" => "Escape".to_string(),
            "Space" => " ".to_string(),
            other => other.to_string(),
        };
        let keys: HashSet<_> = [0, 8, 9].iter().map(|i| canonical(&values[*i])).collect();
        if keys.len() != 3 {
            return Err("flash trigger keys collide; change/unset @flash-key (old default S) or remap @flash-treesitter-key / @flash-treesitter-search-key".into());
        }
        let mut unique = HashSet::new();
        if values[1].is_empty()
            || !values[1]
                .chars()
                .all(|ch| ch.is_ascii_graphic() && unique.insert(ch))
        {
            return Err(
                "@flash-labels must contain unique printable non-space ASCII characters".into(),
            );
        }
        for (i, color) in values.iter().enumerate().take(8).skip(2) {
            if color.is_empty() && (i == 5 || i == 6) {
                continue;
            }
            if color.len() != 7
                || !color.starts_with('#')
                || !color[1..].bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err(format!(
                    "@{} must be #RRGGBB{}",
                    DEFAULTS[i].0,
                    if i == 5 || i == 6 { " or empty" } else { "" }
                )
                .into());
            }
        }
        Ok(Self {
            key: values[0].clone(),
            treesitter_key: values[8].clone(),
            treesitter_search_key: values[9].clone(),
            language: options
                .get("flash-treesitter-language")
                .map(String::as_str)
                .unwrap_or("auto")
                .parse()?,
            labels: values[1].clone(),
            label_bg: values[2].clone(),
            label_fg: values[3].clone(),
            match_fg: values[4].clone(),
            match_bg: values[5].clone(),
            prompt_bg: values[6].clone(),
            prompt_fg: values[7].clone(),
        })
    }
}
