use flash_tmux::{
    config::Config,
    input::{Input, SearchState},
    model::{ScreenPoint, Viewport},
    render::render,
};
use std::collections::BTreeMap;

#[test]
fn frame_keeps_last_row_and_sanitizes_control_sequences() {
    let c = Config::from_options(&BTreeMap::new()).unwrap();
    let v = Viewport::new(vec!["top\u{1b}[2J".into(), "bottom".into()], 20);
    let mut s = SearchState::new(ScreenPoint::default());
    s.reduce(Input::Text("bot".into()), &v, &c);
    let frame = render(&v, &s, &c).unwrap();
    assert!(frame.contains("\u{1b}[2;1H"));
    assert!(!frame.contains("top\u{1b}[2J"));
    assert!(frame.contains("38;2;48;52;70")); // label foreground
    assert!(frame.contains("48;2;244;184;228")); // label background
}

#[test]
fn match_foreground_does_not_recolor_unmatched_text() {
    let defaults = Config::from_options(&BTreeMap::new()).unwrap();
    let custom = Config::from_options(&BTreeMap::from([
        ("flash-match-fg".into(), "#303446".into()),
        ("flash-match-bg".into(), "#f4b8e4".into()),
        ("flash-label-bg".into(), "#8caaee".into()),
    ]))
    .unwrap();
    let view = Viewport::new(vec!["visible target".into()], 30);
    let mut state = SearchState::new(ScreenPoint::default());
    // An empty query must still display the viewport, regardless of match colors.
    assert_eq!(
        render(&view, &state, &defaults).unwrap(),
        render(&view, &state, &custom).unwrap()
    );
    state.reduce(Input::Text("missing".into()), &view, &custom);
    assert_eq!(
        render(&view, &state, &defaults).unwrap(),
        render(&view, &state, &custom).unwrap()
    );

    let mut state = SearchState::new(ScreenPoint::default());
    state.reduce(Input::Text("target".into()), &view, &custom);
    let default_frame = render(&view, &state, &defaults).unwrap();
    let custom_frame = render(&view, &state, &custom).unwrap();
    // The prefix before the first match is unchanged, but match colors still apply.
    assert_eq!(
        default_frame.split("\x1b[0m\x1b[38;2;").next(),
        custom_frame.split("\x1b[0m\x1b[38;2;").next()
    );
    assert!(custom_frame.contains("\x1b[38;2;48;52;70m\x1b[48;2;244;184;228m"));
}

#[test]
fn motion_counts_wide_and_combining_characters() {
    use flash_tmux::model::motion_count;
    assert_eq!(motion_count("\u{4e2d}\u{6587} target", 5), 3);
    assert_eq!(motion_count("e\u{301} target", 2), 2);
    assert_eq!(flash_tmux::model::cell_width("ab\t target"), 15);
    assert_eq!(motion_count("ab\t target", 9), 4);
}
