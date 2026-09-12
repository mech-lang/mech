use alloc::vec::Vec;

use crate::document::red::{
    AstNode, DocumentSyntax, IdentifierSyntax, ParagraphSyntax, SectionSyntax, SyntaxNode,
    SyntaxToken,
};
use crate::document::{
    BodySyntax, CodeBlockSyntax, ExpressionSyntax, MechCodeAltSyntax, MechCodeSyntax,
    OpAssignOperatorSyntax, OpAssignSyntax, ParagraphElementSyntax, SectionElementSyntax,
    SliceRefSyntax, SliceStemSyntax, SubscriptListSyntax, SyntaxKind, TitleFrontMatterSyntax,
    TitleSyntax, UlSubtitleSyntax, VariableAssignSyntax,
};

impl DocumentSyntax {
    pub fn title(&self) -> Option<TitleSyntax> {
        self.syntax().children().find_map(TitleSyntax::cast)
    }

    pub fn body(&self) -> Option<BodySyntax> {
        self.syntax().children().find_map(BodySyntax::cast)
    }

    pub fn sections(&self) -> Vec<SectionSyntax> {
        let mut sections = Vec::new();
        collect_sections(self.syntax(), &mut sections);
        sections
    }
}

impl BodySyntax {
    pub fn sections(&self) -> Vec<SectionSyntax> {
        self.syntax()
            .children()
            .filter_map(SectionSyntax::cast)
            .collect()
    }
}

impl TitleSyntax {
    pub fn front_matter(&self) -> Option<TitleFrontMatterSyntax> {
        self.syntax()
            .children()
            .find_map(TitleFrontMatterSyntax::cast)
    }
}

impl TitleFrontMatterSyntax {
    pub fn keys(&self) -> Vec<IdentifierSyntax> {
        self.syntax()
            .children()
            .filter_map(IdentifierSyntax::cast)
            .collect()
    }
}

impl SectionSyntax {
    pub fn subtitle(&self) -> Option<UlSubtitleSyntax> {
        self.syntax().children().find_map(UlSubtitleSyntax::cast)
    }

    pub fn elements(&self) -> Vec<SectionElementSyntax> {
        self.syntax()
            .children()
            .filter_map(SectionElementSyntax::cast)
            .collect()
    }

    pub fn mech_blocks(&self) -> Vec<MechCodeSyntax> {
        self.syntax()
            .children()
            .filter_map(MechCodeSyntax::cast)
            .collect()
    }
}

impl SectionElementSyntax {
    pub fn value(&self) -> Option<SyntaxNode> {
        self.syntax().children().next()
    }
}

impl ParagraphSyntax {
    pub fn elements(&self) -> Vec<ParagraphElementSyntax> {
        self.syntax()
            .children()
            .filter_map(ParagraphElementSyntax::cast)
            .collect()
    }
}

impl CodeBlockSyntax {
    pub fn delimiters(&self) -> Vec<SyntaxToken> {
        self.syntax()
            .tokens()
            .into_iter()
            .filter(|token| {
                matches!(
                    token.kind(),
                    SyntaxKind::GraveCodeBlockSigil | SyntaxKind::TildeCodeBlockSigil
                )
            })
            .collect()
    }
}

impl MechCodeSyntax {
    pub fn items(&self) -> Vec<MechCodeAltSyntax> {
        self.syntax()
            .children()
            .filter_map(MechCodeAltSyntax::cast)
            .collect()
    }
}

impl MechCodeAltSyntax {
    pub fn value(&self) -> Option<SyntaxNode> {
        self.syntax().children().next()
    }
}

impl SliceRefSyntax {
    pub fn stem(&self) -> Option<SliceStemSyntax> {
        self.syntax().children().find_map(SliceStemSyntax::cast)
    }

    pub fn subscripts(&self) -> Option<SubscriptListSyntax> {
        self.syntax().children().find_map(SubscriptListSyntax::cast)
    }
}

impl OpAssignSyntax {
    pub fn target(&self) -> Option<SliceRefSyntax> {
        self.syntax().children().find_map(SliceRefSyntax::cast)
    }

    pub fn operator(&self) -> Option<OpAssignOperatorSyntax> {
        self.syntax()
            .children()
            .find_map(OpAssignOperatorSyntax::cast)
    }

    pub fn value(&self) -> Option<ExpressionSyntax> {
        self.syntax().children().find_map(ExpressionSyntax::cast)
    }
}

impl VariableAssignSyntax {
    pub fn target(&self) -> Option<SliceRefSyntax> {
        self.syntax().children().find_map(SliceRefSyntax::cast)
    }

    pub fn value(&self) -> Option<ExpressionSyntax> {
        self.syntax().children().find_map(ExpressionSyntax::cast)
    }
}

fn collect_sections(node: &SyntaxNode, output: &mut Vec<SectionSyntax>) {
    for child in node.children() {
        if let Some(section) = SectionSyntax::cast(child.clone()) {
            output.push(section);
        } else if child.kind() == SyntaxKind::Body {
            collect_sections(&child, output);
        }
    }
}
