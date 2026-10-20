use flash_tmux::{
    config::Config,
    model::Viewport,
    syntax::{
        LanguageChoice, LanguageId,
        detect::detect,
        document::{CapturedDocument, SourceDocument},
    },
};
use std::{collections::BTreeMap, time::Duration};

fn source(text: &str) -> SourceDocument {
    let rows: Vec<String> = text.lines().map(str::to_owned).collect();
    let wraps = vec![false; rows.len()];
    SourceDocument::from_capture(&CapturedDocument {
        view: Viewport::new(rows, 200),
        wraps,
    })
    .unwrap()
}
#[test]
fn language_names_and_aliases_are_explicit() {
    for lang in LanguageId::ALL {
        assert_eq!(
            lang.name().parse::<LanguageChoice>().unwrap(),
            LanguageChoice::Explicit(lang)
        );
    }
    for (alias, lang) in [
        ("js", LanguageId::Javascript),
        ("ts", LanguageId::Typescript),
        ("sh", LanguageId::Bash),
        ("rb", LanguageId::Ruby),
        ("c++", LanguageId::Cpp),
    ] {
        assert_eq!(
            alias.parse::<LanguageChoice>().unwrap(),
            LanguageChoice::Explicit(lang)
        );
    }
    assert!("unknown".parse::<LanguageChoice>().is_err());
    let mut options = BTreeMap::new();
    options.insert("flash-treesitter-language".into(), "unknown".into());
    assert!(Config::from_options(&options).is_err());
}
#[test]
fn conservative_detection_rejects_prose_logs_and_common_syntax() {
    for text in [
        "value",
        "{}",
        "this is ordinary prose",
        "INFO started process 42",
        "value + 1;",
    ] {
        assert!(
            detect(&source(text), Duration::from_millis(250)).is_err(),
            "{text}"
        );
    }
}
#[test]
fn distinctive_fixtures_are_detected() {
    for lang in [
        LanguageId::Rust,
        LanguageId::Python,
        LanguageId::Go,
        LanguageId::Php,
        LanguageId::Json,
    ] {
        let text = std::fs::read_to_string(format!("tests/snippets/{}", lang.fixture())).unwrap();
        assert_eq!(
            detect(&source(&text), Duration::from_millis(250)).unwrap(),
            lang
        );
    }
}
#[test]
fn no_detection_with_expired_budget() {
    assert!(detect(&source("fn main() {}"), Duration::ZERO).is_err());
}
