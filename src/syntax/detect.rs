use super::{LanguageChoice, LanguageId, PARSE_BUDGET, ParsedSyntax, document::SourceDocument};
use crate::Result;
use std::time::{Duration, Instant};

fn indicator(language: LanguageId, kind: &str) -> bool {
    match language {
        LanguageId::Rust => matches!(kind, "function_item" | "impl_item" | "struct_item"),
        LanguageId::Python => matches!(kind, "function_definition" | "class_definition"),
        LanguageId::Javascript => matches!(kind, "function_declaration" | "arrow_function"),
        LanguageId::Typescript => matches!(
            kind,
            "type_annotation" | "interface_declaration" | "type_alias_declaration"
        ),
        LanguageId::Tsx => matches!(kind, "jsx_element" | "jsx_self_closing_element"),
        LanguageId::Json => kind == "pair",
        LanguageId::Bash => matches!(kind, "function_definition" | "if_statement"),
        LanguageId::C | LanguageId::Cpp => kind == "function_definition",
        LanguageId::Go => kind == "package_clause",
        LanguageId::Lua => kind == "function_declaration",
        LanguageId::Ruby => matches!(kind, "method" | "singleton_method"),
        LanguageId::Zig => kind == "function_declaration",
        LanguageId::Php => kind == "php_tag",
        LanguageId::Java => kind == "class_declaration",
    }
}

fn score(parsed: &ParsedSyntax, text: &str) -> Option<usize> {
    let mut error_bytes = vec![false; text.len()];
    let mut evidence = false;
    let mut stack = vec![parsed.tree.root_node()];
    while let Some(node) = stack.pop() {
        if node.is_error() || node.is_missing() {
            error_bytes[node.byte_range()].fill(true);
        }
        // Never mine apparent language declarations out of comments or literals.
        if node.kind().contains("comment") || node.kind().contains("string") {
            continue;
        }
        evidence |= indicator(parsed.language, node.kind()) && !node.has_error();
        let mut cursor = node.walk();
        stack.extend(node.children(&mut cursor));
    }
    if !evidence {
        return None;
    }
    let mut total = 0;
    let mut bad = 0;
    for (offset, ch) in text.char_indices().filter(|(_, ch)| !ch.is_whitespace()) {
        total += ch.len_utf8();
        if error_bytes[offset] {
            bad += ch.len_utf8();
        }
    }
    if total == 0 {
        return None;
    }
    let quality = (total - bad) * 100 / total;
    (quality >= 80).then_some(quality)
}

pub fn resolve(
    document: &SourceDocument,
    choice: LanguageChoice,
    budget: Duration,
) -> Result<ParsedSyntax> {
    resolve_inner(document, choice, None, budget)
}

pub fn resolve_at(
    document: &SourceDocument,
    choice: LanguageChoice,
    origin: crate::model::ScreenPoint,
    budget: Duration,
) -> Result<ParsedSyntax> {
    resolve_inner(document, choice, document.byte_at(origin), budget)
}

fn local_score(parsed: &ParsedSyntax, byte: usize) -> Option<usize> {
    let mut current = parsed
        .tree
        .root_node()
        .named_descendant_for_byte_range(byte, byte);
    while let Some(node) = current {
        if node.start_byte() <= byte
            && byte < node.end_byte()
            && !node.has_error()
            && !node.is_missing()
            && indicator(parsed.language, node.kind())
        {
            return Some(100);
        }
        current = node.parent();
    }
    None
}

fn resolve_inner(
    document: &SourceDocument,
    choice: LanguageChoice,
    focus: Option<usize>,
    budget: Duration,
) -> Result<ParsedSyntax> {
    let deadline = Instant::now() + budget.min(PARSE_BUDGET);
    if let LanguageChoice::Explicit(language) = choice {
        return ParsedSyntax::parse(
            language,
            document,
            deadline.saturating_duration_since(Instant::now()),
        );
    }
    let mut candidates = Vec::new();
    for language in LanguageId::ALL {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("syntax language detection deadline expired; set @flash-treesitter-language explicitly".into());
        }
        let mut parsed = ParsedSyntax::parse(language, document, remaining)?;
        let mut quality = match focus {
            Some(byte) => local_score(&parsed, byte).or_else(|| {
                (!parsed.has_errors())
                    .then(|| score(&parsed, document.text()))
                    .flatten()
            }),
            None => score(&parsed, document.text()),
        };
        if quality.is_none()
            && let Some(byte) = focus
        {
            // A shell command preceding `cat` can make a parser consume the
            // following declaration as an erroneous header. Try logical-line
            // starts, preserving every byte offset with blank prefix padding.
            let starts: Vec<_> = document.text()[..byte]
                .match_indices('\n')
                .map(|(index, _)| index + 1)
                .rev()
                .take(32)
                .filter(|start| {
                    let line = document.text()[*start..]
                        .lines()
                        .next()
                        .unwrap_or("")
                        .trim();
                    // Only a cheap admission filter; the grammar must still
                    // prove a valid enclosing declaration before acceptance.
                    line.contains(['(', '{'])
                        || [
                            "def ",
                            "class ",
                            "struct ",
                            "impl ",
                            "function ",
                            "local ",
                            "package ",
                            "<?php",
                        ]
                        .iter()
                        .any(|prefix| line.starts_with(prefix))
                })
                .collect();
            for start in starts {
                let mut text = String::with_capacity(document.text().len());
                for ch in document.text()[..start].bytes() {
                    text.push(if ch == b'\n' { '\n' } else { ' ' });
                }
                text.push_str(&document.text()[start..]);
                let remaining = deadline.saturating_duration_since(Instant::now());
                let recovered = ParsedSyntax::parse_text(language, &text, remaining)?;
                if let Some(score) = local_score(&recovered, byte) {
                    quality = Some(score);
                    parsed = recovered;
                    break;
                }
            }
        }
        if let Some(quality) = quality {
            candidates.push((quality, parsed));
        }
    }
    candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.0));
    if candidates.first().is_some_and(|best| {
        candidates
            .get(1)
            .is_none_or(|second| best.0 >= second.0 + 10)
    }) {
        return Ok(candidates.remove(0).1);
    }
    Err(format!(
        "ambiguous syntax language; set @flash-treesitter-language to {}",
        LanguageId::ALL.map(LanguageId::name).join(", ")
    )
    .into())
}

pub fn detect(document: &SourceDocument, budget: Duration) -> Result<LanguageId> {
    resolve(document, LanguageChoice::Auto, budget).map(|parsed| parsed.language)
}
