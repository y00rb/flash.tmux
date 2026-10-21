use flash_tmux::{
    config::Config,
    model::{InteractionMode, ScreenPoint, Viewport},
    syntax::{
        LanguageId, ParsedSyntax,
        document::{CapturedDocument, SourceDocument},
    },
    syntax_input::SyntaxState,
    syntax_render::{render_syntax, syntax_prompt},
};
use std::{collections::BTreeMap, time::Duration};
#[test]
fn syntax_preview_and_bounded_prompt() {
    let view = Viewport::new(vec!["fn main() { let value = 1; }".into()], 40);
    let source = SourceDocument::from_capture(&CapturedDocument {
        view: view.clone(),
        wraps: vec![false],
    })
    .unwrap();
    let parsed =
        ParsedSyntax::parse(LanguageId::Rust, &source, Duration::from_millis(250)).unwrap();
    let config = Config::from_options(&BTreeMap::new()).unwrap();
    let state = SyntaxState::new(
        InteractionMode::Treesitter,
        source,
        parsed,
        ScreenPoint { row: 0, cell: 24 },
        &config,
    )
    .unwrap();
    assert!(
        render_syntax(&view, &state, &config)
            .unwrap()
            .contains("\x1b[")
    );
    for width in [0, 1, 20, 80] {
        let prompt = syntax_prompt(&state, width);
        assert!(prompt.chars().count() <= width);
        assert!(!prompt.contains('\x1b'));
    }
    assert!(syntax_prompt(&state, 200).contains("TS rust"));
}
