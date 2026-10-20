pub mod document;
pub mod registry;
pub use registry::LanguageId;

use crate::Result;
use std::{
    ops::{ControlFlow, Range},
    time::{Duration, Instant},
};
use tree_sitter::{ParseOptions, Parser, Tree};

pub const MAX_SOURCE_BYTES: usize = 256 * 1024;
pub const PARSE_BUDGET: Duration = Duration::from_millis(250);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyntaxNode {
    pub bytes: Range<usize>,
    pub kind: String,
}

pub struct ParsedSyntax {
    pub language: LanguageId,
    tree: Tree,
}

impl ParsedSyntax {
    pub fn parse(
        language: LanguageId,
        source: &document::SourceDocument,
        budget: Duration,
    ) -> Result<Self> {
        Self::parse_text(language, source.text(), budget)
    }

    pub fn parse_text(language: LanguageId, text: &str, budget: Duration) -> Result<Self> {
        if text.len() > MAX_SOURCE_BYTES {
            return Err("syntax source exceeds 256 KiB".into());
        }
        let deadline = Instant::now() + budget.min(PARSE_BUDGET);
        if budget.is_zero() {
            return Err("syntax parsing deadline expired".into());
        }
        let mut parser = Parser::new();
        parser.set_language(&language.grammar())?;
        let mut progress = |_: &tree_sitter::ParseState| {
            if Instant::now() >= deadline {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        };
        let bytes = text.as_bytes();
        let tree = parser
            .parse_with_options(
                &mut |offset, _| &bytes[offset..],
                None,
                Some(ParseOptions::new().progress_callback(&mut progress)),
            )
            .ok_or("syntax parsing deadline expired")?;
        if Instant::now() >= deadline {
            return Err("syntax parsing deadline expired".into());
        }
        Ok(Self { language, tree })
    }

    pub fn has_errors(&self) -> bool {
        self.tree.root_node().has_error()
    }

    pub fn ancestors_at(&self, byte: usize) -> Vec<SyntaxNode> {
        let root = self.tree.root_node();
        if byte >= root.end_byte() {
            return vec![];
        }
        let mut current = root.named_descendant_for_byte_range(byte, byte);
        let mut nodes: Vec<SyntaxNode> = vec![];
        while let Some(node) = current {
            let bytes = node.byte_range();
            if node.is_named()
                && !node.is_error()
                && !node.is_missing()
                && bytes.start <= byte
                && byte < bytes.end
                && !nodes.iter().any(|n| n.bytes == bytes)
            {
                nodes.push(SyntaxNode {
                    bytes,
                    kind: node.kind().into(),
                });
            }
            current = node.parent();
        }
        nodes
    }
}
