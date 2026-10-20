use flash_tmux::{
    model::{ScreenPoint, Viewport},
    syntax::document::{CapturedDocument, SourceDocument},
};

fn document(rows: &[&str], wraps: Vec<bool>) -> SourceDocument {
    SourceDocument::from_capture(&CapturedDocument {
        view: Viewport::new(rows.iter().map(|s| s.to_string()).collect(), 80),
        wraps,
    })
    .unwrap()
}
#[test]
fn newline_end_does_not_include_next_line() {
    let doc = document(&["abc", "def"], vec![false, false]);
    assert_eq!(doc.text(), "abc\ndef");
    let range = doc.screen_range(0..4).unwrap();
    assert_eq!(range.end, ScreenPoint { row: 0, cell: 2 });
    assert!(doc.screen_range(1..1).is_none());
    assert!(doc.screen_range(0..100).is_none());
}
#[test]
fn soft_wraps_join_source_and_map_both_rows() {
    let doc = document(&["abc", "def"], vec![true, false]);
    assert_eq!(doc.text(), "abcdef");
    assert_eq!(doc.byte_at(ScreenPoint { row: 1, cell: 1 }), Some(4));
    let range = doc.screen_range(1..5).unwrap();
    assert_eq!(range.start, ScreenPoint { row: 0, cell: 1 });
    assert_eq!(range.end, ScreenPoint { row: 1, cell: 1 });
}
#[test]
fn unicode_and_tabs_land_on_grapheme_starts() {
    let doc = document(&["中\te\u{301}👩‍💻"], vec![false]);
    assert_eq!(doc.byte_at(ScreenPoint { row: 0, cell: 1 }), Some(0));
    assert_eq!(doc.byte_at(ScreenPoint { row: 0, cell: 7 }), Some(3));
    assert_eq!(doc.screen_range(0..3).unwrap().end.cell, 0);
    assert_eq!(doc.screen_range(4..7).unwrap().end.cell, 8);
    assert_eq!(doc.screen_range(7..18).unwrap().end.cell, 9);
}
#[test]
fn invalid_metadata_and_control_bytes_are_rejected() {
    let capture = |row: &str, wraps| CapturedDocument {
        view: Viewport::new(vec![row.into()], 80),
        wraps,
    };
    assert!(SourceDocument::from_capture(&capture("abc", vec![])).is_err());
    assert!(SourceDocument::from_capture(&capture("abc", vec![true])).is_err());
    assert!(SourceDocument::from_capture(&capture("\u{1b}[2J", vec![false])).is_err());
}
