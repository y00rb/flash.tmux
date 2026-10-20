use flash_tmux::syntax::{LanguageId, ParsedSyntax};
use std::time::Duration;

#[test]
fn every_fixture_has_expected_nested_named_nodes() {
    assert_eq!(LanguageId::ALL.len(), 15);
    for language in LanguageId::ALL {
        let path = format!("tests/snippets/{}", language.fixture());
        let text = std::fs::read_to_string(path).unwrap();
        let tree = ParsedSyntax::parse_text(language, &text, Duration::from_millis(250)).unwrap();
        assert!(!tree.has_errors(), "{}", language.name());
        let byte = text.find("value").unwrap();
        let nodes = tree.ancestors_at(byte);
        assert!(nodes.len() >= 3, "{}: {nodes:?}", language.name());
        assert_eq!(
            &text[nodes[0].bytes.clone()],
            "value",
            "{}",
            language.name()
        );
        assert!(
            nodes
                .iter()
                .all(|n| n.kind != "ERROR" && !n.bytes.is_empty())
        );
        for pair in nodes.windows(2) {
            assert_ne!(pair[0].bytes, pair[1].bytes);
            assert!(pair[1].bytes.start <= pair[0].bytes.start);
            assert!(pair[1].bytes.end >= pair[0].bytes.end);
        }
    }
}

#[test]
fn truncated_code_keeps_valid_children() {
    let text = "fn main() { let value = 42;";
    let tree =
        ParsedSyntax::parse_text(LanguageId::Rust, text, Duration::from_millis(250)).unwrap();
    assert!(tree.has_errors());
    let nodes = tree.ancestors_at(text.find("42").unwrap());
    assert_eq!(&text[nodes[0].bytes.clone()], "42");
    assert!(nodes.iter().all(|node| node.kind != "ERROR"));
}

#[test]
fn empty_budget_and_oversize_input_are_rejected() {
    assert!(ParsedSyntax::parse_text(LanguageId::Rust, "fn main() {}", Duration::ZERO).is_err());
    assert!(
        ParsedSyntax::parse_text(
            LanguageId::Rust,
            &" ".repeat(256 * 1024 + 1),
            Duration::from_millis(250)
        )
        .is_err()
    );
}
