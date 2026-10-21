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
        LanguageId::Go => matches!(
            kind,
            "package_clause" | "function_declaration" | "method_declaration"
        ),
        LanguageId::Lua => kind == "function_declaration",
        LanguageId::Ruby => matches!(kind, "method" | "singleton_method"),
        LanguageId::Zig => kind == "function_declaration",
        LanguageId::Php => kind == "php_tag",
        LanguageId::Java => kind == "class_declaration",
    }
}

fn score(parsed: &ParsedSyntax, text: &str) -> Option<usize> {
    let mut errors = vec![false; text.len()];
    let mut evidence = false;
    let mut stack = vec![parsed.tree.root_node()];
    while let Some(node) = stack.pop() {
        if node.is_error() || node.is_missing() {
            errors[node.byte_range()].fill(true);
        }
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
        if errors[offset] {
            bad += ch.len_utf8();
        }
    }
    if total == 0 {
        return None;
    }
    let quality = (total - bad) * 100 / total;
    (quality >= 80).then_some(quality)
}

fn local_score(parsed: &ParsedSyntax, text: &str, byte: usize) -> bool {
    let mut current = parsed
        .tree
        .root_node()
        .named_descendant_for_byte_range(byte, byte);
    while let Some(node) = current {
        if node.start_byte() <= byte
            && byte < node.end_byte()
            && !node.has_error()
            && !node.is_missing()
            && text[..node.start_byte()].trim().is_empty()
        {
            let mut stack = vec![node];
            while let Some(child) = stack.pop() {
                if child.kind().contains("comment") || child.kind().contains("string") {
                    continue;
                }
                if indicator(parsed.language, child.kind()) {
                    return true;
                }
                let mut cursor = child.walk();
                stack.extend(child.children(&mut cursor));
            }
        }
        current = node.parent();
    }
    false
}

fn ambiguous() -> crate::Error {
    format!(
        "ambiguous syntax language; set @flash-treesitter-language to {}",
        LanguageId::ALL.map(LanguageId::name).join(", ")
    )
    .into()
}

pub fn resolve(
    document: &SourceDocument,
    choice: LanguageChoice,
    budget: Duration,
) -> Result<ParsedSyntax> {
    resolve_inner(document, choice, None, budget, true)
}

pub fn resolve_at(
    document: &SourceDocument,
    choice: LanguageChoice,
    origin: crate::model::ScreenPoint,
    budget: Duration,
) -> Result<ParsedSyntax> {
    resolve_inner(
        document,
        choice,
        document.cursor_byte_at(origin),
        budget,
        true,
    )
}

pub fn resolve_selection_at(
    document: &SourceDocument,
    choice: LanguageChoice,
    origin: crate::model::ScreenPoint,
    budget: Duration,
) -> Result<ParsedSyntax> {
    resolve_inner(
        document,
        choice,
        document.cursor_byte_at(origin),
        budget,
        false,
    )
}

fn resolve_inner(
    document: &SourceDocument,
    choice: LanguageChoice,
    focus: Option<usize>,
    budget: Duration,
    include_fallback: bool,
) -> Result<ParsedSyntax> {
    let deadline = Instant::now() + budget.min(PARSE_BUDGET);
    if let LanguageChoice::Explicit(language) = choice {
        return ParsedSyntax::parse(
            language,
            document,
            deadline.saturating_duration_since(Instant::now()),
        );
    }
    if let Some(byte) = focus {
        // Parse bounded local fragments, not a full-screen prefix padded with
        // blanks for every candidate. Tree byte offsets are translated back.
        let text = document.text();
        let starts: Vec<_> = std::iter::once(0)
            .chain(text[..byte].match_indices('\n').map(|(i, _)| i + 1))
            .collect();
        let end = text[byte..]
            .match_indices('\n')
            .nth(16)
            .map_or(text.len(), |(i, _)| byte + i);
        for start in starts.into_iter().rev().take(32) {
            if end - start > 8192 {
                break;
            }
            let line = text[start..].lines().next().unwrap_or("").trim();
            if !line.contains(['(', '{'])
                && ![
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
            {
                continue;
            }
            let mut candidates = Vec::new();
            for language in LanguageId::ALL {
                let parsed = ParsedSyntax::parse_text(
                    language,
                    &text[start..end],
                    deadline.saturating_duration_since(Instant::now()),
                )
                .map_err(|error| -> crate::Error {
                    format!(
                        "{error} during {} local language detection",
                        language.name()
                    )
                    .into()
                })?;
                if local_score(&parsed, &text[start..end], byte - start) {
                    let strength = if matches!(language, LanguageId::C | LanguageId::Cpp) {
                        100
                    } else {
                        120
                    };
                    candidates.push((strength, parsed));
                }
            }
            // Reserved declaration syntax is stronger evidence than C/C++
            // treating an unknown keyword such as `fn` as a typedef name.
            candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.0));
            if candidates.len() > 1 && candidates[0].0 == candidates[1].0 {
                return Err(format!(
                    "ambiguous syntax language ({}); set @flash-treesitter-language explicitly",
                    candidates
                        .iter()
                        .map(|(_, parsed)| parsed.language.name())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
                .into());
            }
            if !candidates.is_empty() {
                let (_, mut parsed) = candidates.remove(0);
                // S only selects ancestors at this cursor; parsing unrelated
                // terminal history again is necessary for R's remote hits only.
                if include_fallback {
                    let full = ParsedSyntax::parse(
                        parsed.language,
                        document,
                        deadline.saturating_duration_since(Instant::now()),
                    )
                    .map_err(|error| -> crate::Error {
                        format!(
                            "{error} while preparing {} full-viewport search",
                            parsed.language.name()
                        )
                        .into()
                    })?;
                    parsed.fallback = Some(full.tree);
                }
                parsed.offset = start;
                return Ok(parsed);
            }
        }
        return Err(ambiguous());
    }
    let mut candidates = Vec::new();
    for language in LanguageId::ALL {
        let parsed = ParsedSyntax::parse(
            language,
            document,
            deadline.saturating_duration_since(Instant::now()),
        )?;
        if let Some(quality) = score(&parsed, document.text()) {
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
    Err(ambiguous())
}

pub fn detect(document: &SourceDocument, budget: Duration) -> Result<LanguageId> {
    resolve(document, LanguageChoice::Auto, budget).map(|parsed| parsed.language)
}
