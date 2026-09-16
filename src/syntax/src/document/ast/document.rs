use alloc::vec::Vec;

use crate::document::red::{
    AstNode, DocumentSyntax, IdentifierSyntax, ParagraphSyntax, SectionSyntax, SyntaxNode,
    SyntaxToken,
};
use crate::document::{
    BodySyntax, CodeBlockSyntax, CodeFenceInfo, ContextSendSyntax, EnumDefineSyntax,
    EnumVariantInlineKindSyntax, EnumVariantKindSyntax, EnumVariantSyntax,
    EvalInlineMechCodeSyntax, ExpressionSyntax, KindAnnotationSyntax, KindDefineSyntax,
    MechCodeAltSyntax, MechCodeSyntax, OpAssignOperatorSyntax, OpAssignSyntax, OptionMapSyntax,
    ParagraphElementSyntax, SectionElementSyntax, SliceRefSyntax, SliceStemSyntax,
    SubscriptListSyntax, SyntaxKind, TextRange, TitleFrontMatterSyntax, TitleSyntax,
    TupleDestructureSyntax, UlSubtitleSyntax, VariableAssignSyntax, VariableSyntax,
};

impl KindDefineSyntax {
    pub fn name(&self) -> Option<IdentifierSyntax> {
        self.syntax().children().find_map(IdentifierSyntax::cast)
    }

    pub fn annotation(&self) -> Option<KindAnnotationSyntax> {
        self.syntax()
            .children()
            .find_map(KindAnnotationSyntax::cast)
    }
}

impl EnumDefineSyntax {
    pub fn name(&self) -> Option<IdentifierSyntax> {
        self.syntax().children().find_map(IdentifierSyntax::cast)
    }

    pub fn variants(&self) -> Vec<EnumVariantSyntax> {
        self.syntax()
            .children()
            .filter_map(EnumVariantSyntax::cast)
            .collect()
    }
}

impl EnumVariantSyntax {
    pub fn name(&self) -> Option<IdentifierSyntax> {
        self.syntax().children().find_map(IdentifierSyntax::cast)
    }

    pub fn payload(&self) -> Option<KindAnnotationSyntax> {
        self.syntax().children().find_map(|child| {
            if let Some(inline) = EnumVariantInlineKindSyntax::cast(child.clone()) {
                inline
                    .syntax()
                    .children()
                    .find_map(KindAnnotationSyntax::cast)
            } else {
                EnumVariantKindSyntax::cast(child).and_then(|payload| {
                    payload
                        .syntax()
                        .children()
                        .find_map(KindAnnotationSyntax::cast)
                })
            }
        })
    }
}

impl DocumentSyntax {
    pub fn title(&self) -> Option<TitleSyntax> {
        self.syntax().children().find_map(TitleSyntax::cast)
    }

    pub fn body(&self) -> Option<BodySyntax> {
        self.syntax().children().find_map(BodySyntax::cast)
    }

    /// Whether this clean document contains executable source syntax.
    ///
    /// This is a source-classification query, not target-capability validation.
    /// It excludes comments, displayed inline code, Mika-local source, and
    /// disabled/inert fences. Evaluated inline expressions and named Mech
    /// scopes count as source. Consumers still perform semantic validation.
    pub fn contains_executable_source(&self) -> bool {
        use crate::document::{CodeFenceScope, NodeFlags};
        if self.syntax().flags().intersects(
            NodeFlags::ERROR
                | NodeFlags::MISSING
                | NodeFlags::CONTAINS_ERROR
                | NodeFlags::CONTAINS_MISSING,
        ) {
            return false;
        }
        let mut pending = alloc::vec![self.syntax().clone()];
        while let Some(node) = pending.pop() {
            if matches!(
                node.kind(),
                SyntaxKind::InlineMechCode | SyntaxKind::MikaSection
            ) {
                continue;
            }
            if let Some(fence) = CodeBlockSyntax::cast(node.clone()) {
                if matches!(
                    fence.info().map(|info| info.scope),
                    Some(CodeFenceScope::Root | CodeFenceScope::Named(_))
                ) && let Some(body) = fence.mech_code()
                {
                    pending.push(body.syntax().clone());
                }
                continue;
            }
            if EvalInlineMechCodeSyntax::cast(node.clone()).is_some() {
                return true;
            }
            if let Some(code) = MechCodeSyntax::cast(node.clone()) {
                if code
                    .items()
                    .iter()
                    .filter_map(MechCodeAltSyntax::value)
                    .any(|item| mech_code_item_is_executable(&item))
                {
                    return true;
                }
                continue;
            }
            pending.extend(node.children());
        }
        false
    }

    pub fn sections(&self) -> Vec<SectionSyntax> {
        let mut sections = Vec::new();
        collect_sections(self.syntax(), &mut sections);
        sections
    }
}

fn mech_code_item_is_executable(item: &SyntaxNode) -> bool {
    fn is_metadata(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::Comment
                | SyntaxKind::ContextDeclaration
                | SyntaxKind::ExportDeclaration
                | SyntaxKind::ImportDeclaration
                | SyntaxKind::ModuleImport
        )
    }

    if is_metadata(item.kind()) {
        return false;
    }
    if item.kind() == SyntaxKind::Statement {
        return item.children().any(|child| !is_metadata(child.kind()));
    }
    true
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
    pub fn info_range(&self) -> Option<TextRange> {
        let start = self.delimiters().first()?.range().end;
        let end = self
            .syntax()
            .tokens()
            .into_iter()
            .find(|token| {
                matches!(
                    token.kind(),
                    SyntaxKind::Newline | SyntaxKind::CarriageReturn
                )
            })?
            .range()
            .start;
        Some(TextRange::new(start, end))
    }

    pub fn info(&self) -> Option<CodeFenceInfo> {
        let text = self.syntax().source().text(self.info_range()?).ok()?;
        let info = text.split_once('{').map_or(text.as_str(), |(info, _)| info);
        Some(CodeFenceInfo::from_info_string(info))
    }

    pub fn presentation(&self) -> Option<crate::document::CodeFencePresentation> {
        use crate::document::NodeFlags;
        if self.syntax().flags().intersects(
            NodeFlags::ERROR
                | NodeFlags::MISSING
                | NodeFlags::CONTAINS_ERROR
                | NodeFlags::CONTAINS_MISSING,
        ) {
            return None;
        }
        let mut presentation = crate::document::CodeFencePresentation::default();
        if let Some(options) = self.options() {
            for mapping in options.mappings() {
                let key = mapping.key()?.syntax().text().ok()?;
                let value = mapping.value()?.decoded_text()?;
                if key == "output" {
                    presentation.show_output = !matches!(
                        value.trim().to_ascii_lowercase().as_str(),
                        "false" | "no" | "off" | "0"
                    );
                } else {
                    presentation.styles.push((key, value));
                }
            }
        }
        if self.info()?.hidden {
            presentation.show_output = false;
        }
        Some(presentation)
    }

    pub fn options(&self) -> Option<OptionMapSyntax> {
        self.syntax().children().find_map(OptionMapSyntax::cast)
    }

    pub fn mech_code(&self) -> Option<MechCodeSyntax> {
        self.syntax().children().find_map(MechCodeSyntax::cast)
    }

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

impl EvalInlineMechCodeSyntax {
    pub fn expression(&self) -> Option<ExpressionSyntax> {
        self.syntax().children().find_map(ExpressionSyntax::cast)
    }
}

impl ContextSendSyntax {
    pub fn target(&self) -> Option<VariableSyntax> {
        self.syntax().children().find_map(VariableSyntax::cast)
    }

    pub fn expression(&self) -> Option<ExpressionSyntax> {
        self.syntax().children().find_map(ExpressionSyntax::cast)
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

impl OptionMapSyntax {
    pub fn mappings(&self) -> Vec<crate::document::OptionMappingSyntax> {
        self.syntax()
            .children()
            .filter_map(crate::document::OptionMappingSyntax::cast)
            .collect()
    }
}

impl crate::document::OptionMappingSyntax {
    pub fn key(&self) -> Option<IdentifierSyntax> {
        self.syntax().children().find_map(IdentifierSyntax::cast)
    }
    pub fn value(&self) -> Option<crate::document::OptionValueSyntax> {
        self.syntax()
            .children()
            .find_map(crate::document::OptionValueSyntax::cast)
    }
}

impl crate::document::OptionValueSyntax {
    pub fn decoded_text(&self) -> Option<alloc::string::String> {
        if let Some(string) = self
            .syntax()
            .children()
            .find_map(crate::document::StringLiteralSyntax::cast)
        {
            return string.decoded_text();
        }
        self.syntax()
            .children()
            .find_map(IdentifierSyntax::cast)?
            .syntax()
            .text()
            .ok()
    }
}

impl TupleDestructureSyntax {
    pub fn names(&self) -> Vec<IdentifierSyntax> {
        self.syntax()
            .children()
            .filter_map(IdentifierSyntax::cast)
            .collect()
    }

    pub fn value(&self) -> Option<ExpressionSyntax> {
        self.syntax().children().find_map(ExpressionSyntax::cast)
    }
}
