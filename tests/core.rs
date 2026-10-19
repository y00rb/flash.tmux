use flash_tmux::{
    config::Config,
    input::{Action, Input, SearchState},
    model::{ScreenPoint, Viewport},
    search::{assign_labels, find_matches},
};
use std::collections::BTreeMap;

#[test]
fn config_defaults_and_validation() {
    let config = Config::from_options(&BTreeMap::new()).unwrap();
    assert_eq!(config.key, "S");
    assert_eq!(
        config.labels,
        "asdfghjklASDFHJKLqwertyuiopzxcvbnmQWERTYUIOPZXCVBNM"
    );
    assert_eq!(config.label_bg, "#f4b8e4");
    assert_eq!(config.label_fg, "#303446");
    assert_eq!(config.match_fg, "#c6d0f5");
    assert_eq!(config.match_bg, "#51576d");
    assert_eq!(config.prompt_bg, "#8caaee");
    assert_eq!(config.prompt_fg, "#303446");
    for invalid in ["", "aa", "\u{4e2d}", "a\n"] {
        assert!(
            Config::from_options(&BTreeMap::from([("flash-labels".into(), invalid.into())]))
                .is_err()
        );
    }
    assert!(
        Config::from_options(&BTreeMap::from([("flash-label-bg".into(), "blue".into())])).is_err()
    );
    assert!(Config::from_options(&BTreeMap::from([("flash-match-bg".into(), "".into())])).is_ok());
}

#[test]
fn matching_maps_original_unicode_offsets_and_smartcase() {
    let v = Viewport::new(
        vec![
            "foo Foo FOO".into(),
            "\u{4e2d}\u{6587} target".into(),
            "\u{130} X".into(),
        ],
        80,
    );
    let origin = ScreenPoint { row: 0, cell: 4 };
    let matches = find_matches(&v, "foo", origin);
    assert_eq!(matches.len(), 3);
    assert_eq!(matches[0].point.cell, 4);
    assert_eq!(find_matches(&v, "Foo", origin).len(), 1);
    assert_eq!(find_matches(&v, "target", origin)[0].point.cell, 5);
    assert_eq!(find_matches(&v, "x", origin)[0].point.cell, 2);
    assert!(find_matches(&v, "", origin).is_empty());
    assert!(find_matches(&v, "FOO\n", origin).is_empty());
    assert_eq!(
        find_matches(&Viewport::new(vec!["aaaa".into()], 80), "aa", origin).len(),
        3
    );
}

#[test]
fn matches_inside_graphemes_jump_to_the_containing_cell() {
    let v = Viewport::new(
        vec![
            "\u{1f469}\u{200d}\u{1f4bb} target".into(),
            "e\u{301} target".into(),
        ],
        80,
    );
    let emoji = find_matches(&v, "\u{1f4bb}", ScreenPoint::default());
    assert_eq!(emoji[0].point.cell, 0);
    assert_eq!(emoji[0].end_cell, 2);
    let accent = find_matches(&v, "\u{301}", ScreenPoint::default());
    assert_eq!(accent[0].point.cell, 0);
    assert_eq!(accent[0].end_cell, 1);
}

#[test]
fn labels_do_not_consume_search_continuations() {
    let v = Viewport::new(vec!["sea see set".into()], 80);
    let m = find_matches(&v, "se", ScreenPoint::default());
    let labels = assign_labels(&v, "se", &m, "aetjk");
    assert_eq!(labels.len(), 2);
    assert_eq!(labels[0].label, 'j');
    assert_eq!(labels[1].label, 'k');
}

#[test]
fn input_edits_graphemes_and_preserves_q_as_text() {
    let v = Viewport::new(vec!["target q".into()], 80);
    let c = Config::from_options(&BTreeMap::new()).unwrap();
    let mut s = SearchState::new(ScreenPoint::default());
    assert_eq!(s.reduce(Input::Enter, &v, &c), Action::Continue);
    s.reduce(Input::Text("e\u{301}".into()), &v, &c);
    s.reduce(Input::Backspace, &v, &c);
    assert_eq!(s.query, "");
    s.reduce(Input::Text("q".into()), &v, &c);
    assert_eq!(s.query, "q");
    assert_eq!(
        s.reduce(Input::Enter, &v, &c),
        Action::Select(ScreenPoint { row: 0, cell: 7 })
    );
    assert_eq!(s.reduce(Input::Escape, &v, &c), Action::Cancel);
}
