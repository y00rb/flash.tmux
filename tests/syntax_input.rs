use flash_tmux::{
    config::Config,
    input::Input,
    model::{InteractionMode, ScreenPoint, Viewport},
    syntax::{
        LanguageId, ParsedSyntax,
        document::{CapturedDocument, SourceDocument},
    },
    syntax_input::{SyntaxAction, SyntaxState},
};
use std::{collections::BTreeMap, time::Duration};

fn state(mode: InteractionMode) -> (SyntaxState, Config) {
    let text = "fn main() { let value = outer(inner(1)); let next = value; }";
    let view = Viewport::new(vec![text.into()], 100);
    let source = SourceDocument::from_capture(&CapturedDocument {
        view,
        wraps: vec![false],
    })
    .unwrap();
    let parsed =
        ParsedSyntax::parse(LanguageId::Rust, &source, Duration::from_millis(250)).unwrap();
    let config = Config::from_options(&BTreeMap::new()).unwrap();
    let origin = ScreenPoint {
        row: 0,
        cell: text.find('1').unwrap(),
    };
    (
        SyntaxState::new(mode, source, parsed, origin, &config).unwrap(),
        config,
    )
}
#[test]
fn s_on_blank_line_previews_enclosing_block() {
    let view = Viewport::new(
        vec![
            "fn main() {".into(),
            "".into(),
            "    let value = 42;".into(),
            "}".into(),
        ],
        80,
    );
    let source = SourceDocument::from_capture(&CapturedDocument {
        view,
        wraps: vec![false; 4],
    })
    .unwrap();
    let parsed =
        ParsedSyntax::parse(LanguageId::Rust, &source, Duration::from_millis(250)).unwrap();
    let config = Config::from_options(&BTreeMap::new()).unwrap();
    let mut state = SyntaxState::new(
        InteractionMode::Treesitter,
        source,
        parsed,
        ScreenPoint { row: 1, cell: 0 },
        &config,
    )
    .unwrap();
    assert_eq!(state.targets[0].node.kind, "block");
    assert_eq!(
        state.reduce(Input::Enter, &config),
        SyntaxAction::Select(state.targets[0].range)
    );
}

#[test]
fn s_previews_smallest_cycles_and_confirms() {
    let (mut state, config) = state(InteractionMode::Treesitter);
    let first = state.targets[0].range;
    assert_eq!(first.start, first.end);
    assert_eq!(state.current, Some(0));
    assert_eq!(
        state.reduce(Input::Text(";".into()), &config),
        SyntaxAction::Continue
    );
    assert_eq!(state.current, Some(1));
    state.reduce(Input::Text(",".into()), &config);
    assert_eq!(state.current, Some(0));
    assert_eq!(
        state.reduce(Input::Enter, &config),
        SyntaxAction::Select(first)
    );
    state.reduce(Input::Text(",".into()), &config);
    assert_eq!(state.current, Some(state.targets.len() - 1));
    state.reduce(Input::Text(";".into()), &config);
    assert_eq!(state.current, Some(0));
}
#[test]
fn labels_confirm_and_cancel_never_selects() {
    let (mut state, config) = state(InteractionMode::Treesitter);
    let label = state.targets[1].label.unwrap();
    let range = state.targets[1].range;
    assert_eq!(
        state.reduce(Input::Text(label.to_string()), &config),
        SyntaxAction::Select(range)
    );
    assert_eq!(state.reduce(Input::Escape, &config), SyntaxAction::Cancel);
    assert_eq!(
        state.reduce(Input::Interrupt, &config),
        SyntaxAction::Cancel
    );
}
#[test]
fn s_cycles_when_labels_run_out() {
    let (mut state, mut config) = state(InteractionMode::Treesitter);
    config.labels = "a".into();
    state.reduce(Input::Text(";".into()), &config);
    assert_eq!(state.current, Some(1));
    assert!(matches!(
        state.reduce(Input::Enter, &config),
        SyntaxAction::Select(_)
    ));
}
#[test]
fn r_searches_nodes_deduplicates_and_handles_backspace() {
    let (mut state, config) = state(InteractionMode::TreesitterSearch);
    assert!(state.targets.is_empty());
    state.reduce(Input::Text("value".into()), &config);
    assert_eq!(state.hits.len(), 2);
    assert_eq!(state.query, "value");
    let ranges: Vec<_> = state.targets.iter().map(|t| t.node.bytes.clone()).collect();
    let unique: std::collections::HashSet<_> = ranges.iter().collect();
    assert_eq!(unique.len(), ranges.len());
    let selected = state.targets[0].range;
    assert_eq!(
        state.reduce(Input::Enter, &config),
        SyntaxAction::Select(selected)
    );
    state.reduce(Input::Backspace, &config);
    assert_eq!(state.query, "valu");
    state.reduce(Input::Text("XYZ".into()), &config);
    assert!(state.targets.is_empty());
    assert_eq!(state.reduce(Input::Enter, &config), SyntaxAction::Continue);
}
#[test]
fn r_continuation_characters_are_not_labels() {
    let (mut state, config) = state(InteractionMode::TreesitterSearch);
    state.reduce(Input::Text("val".into()), &config);
    assert!(
        state
            .targets
            .iter()
            .all(|target| target.label != Some('u') && target.label != Some('U'))
    );
    assert_eq!(
        state.reduce(Input::Text("u".into()), &config),
        SyntaxAction::Continue
    );
    assert_eq!(state.query, "valu");
}
