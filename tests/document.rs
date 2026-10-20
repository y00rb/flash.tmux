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
fn joined_capture_distinguishes_wraps_from_full_width_hard_lines() {
    use flash_tmux::syntax::document::decode_capture_pair;
    assert_eq!(
        decode_capture_pair("abc  \n", "abc  def\n").unwrap(),
        ("abc  ".into(), true)
    );
    assert_eq!(
        decode_capture_pair("abc\n", "abc\ndef\n").unwrap(),
        ("abc".into(), false)
    );
    assert!(decode_capture_pair("abc\n", "changed\n").is_err());
    assert!(decode_capture_pair("abc", "abcdef\n").is_err());
}

#[test]
fn cursor_on_blank_line_maps_to_newline_but_not_a_confirmed_endpoint() {
    let doc = document(&["fn main() {", "", "}"], vec![false; 3]);
    let point = ScreenPoint { row: 1, cell: 0 };
    assert_eq!(doc.byte_at(point), None);
    assert_eq!(doc.cursor_byte_at(point), Some(12));
    assert_eq!(
        doc.cursor_byte_at(ScreenPoint { row: 1, cell: 79 }),
        Some(12)
    );
    assert_eq!(doc.cursor_byte_at(ScreenPoint { row: 1, cell: 80 }), None);
    assert_eq!(doc.cursor_byte_at(ScreenPoint { row: 99, cell: 0 }), None);
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
