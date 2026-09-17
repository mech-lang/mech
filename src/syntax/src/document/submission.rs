//! Canonical source-level metadata for interactive submissions.

use alloc::vec;
use alloc::vec::Vec;

use super::{
    DocumentId, ParseConfig, Revision, SyntaxElement, SyntaxKind, TextSnapshot,
    parse_canonical_document,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubmissionTerminal {
    pub byte_offset: usize,
    pub suppresses_value: bool,
}

/// Return terminal metadata for the final canonical code item in a finite
/// submission. Comments, strings, and resource URI text cannot masquerade as
/// the semicolon owned by a `CodeTerminal` node.
pub fn submission_terminal(source: &str) -> Option<SubmissionTerminal> {
    let source = TextSnapshot::new(DocumentId(0), Revision(0), source).ok()?;
    let parsed = parse_canonical_document(source, ParseConfig::default());
    let mut pending = vec![parsed.syntax()];
    let mut terminal = None;
    while let Some(node) = pending.pop() {
        let children = node.children().collect::<Vec<_>>();
        if node.kind() == SyntaxKind::MechCode {
            let mut executable_owner = false;
            for child in &children {
                if child.kind() == SyntaxKind::MechCodeAlt {
                    executable_owner = child
                        .children()
                        .any(|owner| owner.kind() != SyntaxKind::Comment);
                } else if child.kind() == SyntaxKind::CodeTerminal
                    && executable_owner
                    && terminal
                        .as_ref()
                        .is_none_or(|previous: &super::SyntaxNode| {
                            previous.range().start < child.range().start
                        })
                {
                    terminal = Some(child.clone());
                }
            }
        }
        pending.extend(children);
    }
    let terminal = terminal?;
    let semicolon = terminal
        .children_with_tokens()
        .into_iter()
        .find_map(|element| match element {
            SyntaxElement::Token(token) if token.kind() == SyntaxKind::Semicolon => Some(token),
            _ => None,
        });
    Some(match semicolon {
        Some(token) => SubmissionTerminal {
            byte_offset: token.range().start.0 as usize,
            suppresses_value: true,
        },
        None => SubmissionTerminal {
            byte_offset: terminal.range().start.0 as usize,
            suppresses_value: false,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_metadata_follows_canonical_comments_strings_and_resource_uris() {
        for source in [
            "1 + 1; -- suppressed\n",
            "1 + 1; // suppressed\n",
            "1 + 1;\n-- later comment\n",
        ] {
            assert!(
                submission_terminal(source).unwrap().suppresses_value,
                "{source:?}"
            );
        }
        for source in [
            "1 + 1 -- comment ;\n",
            "1 + 1-- comment ;\n",
            "1 + 1// comment ;\n",
            "\"text; -- still text\"\n",
            "@out := console://repl/output{:write(line)}\n",
            "@out := console://repl//output-part{:write(line)}\n",
        ] {
            assert!(
                !submission_terminal(source).unwrap().suppresses_value,
                "{source:?}"
            );
        }
    }
}
