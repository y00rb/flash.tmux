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
fn cursor_local_detection_handles_cat_output_among_shell_text() {
    use flash_tmux::model::ScreenPoint;
    use flash_tmux::syntax::detect::resolve_at;
    let text = "paul@host ~/checkout $ cat tests/snippets/rust.rs\nfn main() {\n    let value = 42;\n    if value > 0 {\n        println!(\"{}\", value);\n    }\n}\npaul@host ~/checkout $ tmux set-option -p @flash-treesitter-language auto\n";
    let doc = source(text);
    for cell in [8, 16] {
        let parsed = resolve_at(
            &doc,
            LanguageChoice::Auto,
            ScreenPoint { row: 2, cell },
            Duration::from_millis(250),
        )
        .unwrap();
        assert_eq!(parsed.language, LanguageId::Rust);
        let nodes = parsed.ancestors_at(doc.byte_at(ScreenPoint { row: 2, cell }).unwrap());
        assert!(
            nodes
                .iter()
                .all(|node| node.bytes.start >= text.find("fn main").unwrap())
        );
        assert!(nodes.iter().any(|node| node.kind == "function_item"));
    }
    assert!(
        resolve_at(
            &doc,
            LanguageChoice::Auto,
            ScreenPoint { row: 0, cell: 0 },
            Duration::from_millis(250)
        )
        .is_err()
    );
    for text in ["value", "{}", "value + 1;"] {
        assert!(
            resolve_at(
                &source(text),
                LanguageChoice::Auto,
                ScreenPoint { row: 0, cell: 0 },
                Duration::from_millis(250)
            )
            .is_err()
        );
    }
}

#[test]
fn cursor_local_detection_handles_dense_terminal_history_within_budget() {
    use flash_tmux::{model::ScreenPoint, syntax::detect::resolve_at};
    let history = "user@host ~/checkout $ tmux set-option -p @flash-treesitter-language auto && cat tests/snippets/rust.rs\n".repeat(48);
    let text = format!("{history}fn main() {{\n    let value = 42;\n}}\nuser@host ~/checkout $\n");
    let doc = source(&text);
    let parsed = resolve_at(
        &doc,
        LanguageChoice::Auto,
        ScreenPoint { row: 49, cell: 8 },
        Duration::from_millis(250),
    )
    .unwrap();
    assert_eq!(parsed.language, LanguageId::Rust);
}

#[test]
fn cursor_detection_handles_go_among_prior_code_and_shell_history() {
    use flash_tmux::{model::ScreenPoint, syntax::detect::resolve_at};
    let history = "user@host ~/checkout $ tmux set-option -p @flash-treesitter-language auto\nfn old() { let value = 1; }\n".repeat(20);
    let text = format!(
        "{history}package sample\n\nfunc describe(value int) int {{\n    if value > 0 {{\n        return value + 1\n    }}\n    return 0\n}}\nuser@host ~/checkout $\n"
    );
    let doc = source(&text);
    let parsed = resolve_at(
        &doc,
        LanguageChoice::Auto,
        ScreenPoint { row: 44, cell: 15 },
        Duration::from_millis(250),
    )
    .unwrap();
    assert_eq!(parsed.language, LanguageId::Go);
    let nodes = parsed.ancestors_at(doc.byte_at(ScreenPoint { row: 44, cell: 15 }).unwrap());
    assert!(nodes.iter().any(|node| node.kind == "function_declaration"));
    assert!(
        nodes
            .iter()
            .all(|node| node.bytes.start >= text.find("func describe").unwrap())
    );
}

#[test]
fn selection_detection_does_not_build_unrelated_viewport_fallback() {
    use flash_tmux::{
        model::ScreenPoint,
        syntax::detect::{resolve_at, resolve_selection_at},
    };
    let mut rows = vec!["old terminal output that is not Rust".to_string(); 40];
    rows.extend([
        "fn main() {".into(),
        "    let value = 42;".into(),
        "}".into(),
    ]);
    rows.extend(vec!["".into(); 20]);
    rows.extend(["fn later() { let other = 7; }".into()]);
    let text = rows.join("\n");
    let doc = source(&text);
    let origin = ScreenPoint { row: 41, cell: 8 };
    let local = resolve_selection_at(
        &doc,
        LanguageChoice::Auto,
        origin,
        Duration::from_millis(250),
    )
    .unwrap();
    let search = resolve_at(
        &doc,
        LanguageChoice::Auto,
        origin,
        Duration::from_millis(250),
    )
    .unwrap();
    let later = text.find("other").unwrap();
    assert!(local.ancestors_at(later).is_empty());
    assert!(!search.ancestors_at(later).is_empty());
    let nodes = local.ancestors_at(doc.byte_at(origin).unwrap());
    assert!(nodes.iter().any(|node| node.kind == "function_item"));
    assert_eq!(&text[nodes[0].bytes.clone()], "value");
}

#[test]
fn no_detection_with_expired_budget() {
    assert!(detect(&source("fn main() {}"), Duration::ZERO).is_err());
}
