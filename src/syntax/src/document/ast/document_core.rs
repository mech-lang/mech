// Generated from the canonical S7 document rule closure.
// Do not edit by hand.

use crate::document::{AstNode, SyntaxKind, SyntaxNode};

macro_rules! document_ast_node {
    ($name:ident, $kind:ident) => {
        #[derive(Clone, Debug)]
        pub struct $name(pub(crate) SyntaxNode);

        impl AstNode for $name {
            fn can_cast(kind: SyntaxKind) -> bool {
                kind == SyntaxKind::$kind
            }

            fn cast(syntax: SyntaxNode) -> Option<Self> {
                Self::can_cast(syntax.kind()).then_some(Self(syntax))
            }

            fn syntax(&self) -> &SyntaxNode {
                &self.0
            }
        }
    };
}

document_ast_node!(AbstractElSyntax, AbstractEl);
document_ast_node!(ActivationArmSyntax, ActivationArm);
document_ast_node!(ActivationScopeSyntax, ActivationScope);
document_ast_node!(AlignmentSeparatorSyntax, AlignmentSeparator);
document_ast_node!(BodySyntax, Body);
document_ast_node!(CenterAlignmentSyntax, CenterAlignment);
document_ast_node!(CheckListSyntax, CheckList);
document_ast_node!(CheckListItemSyntax, CheckListItem);
document_ast_node!(CheckedItemSyntax, CheckedItem);
document_ast_node!(CitationSyntax, Citation);
document_ast_node!(CodeBlockSyntax, CodeBlock);
document_ast_node!(CodeTerminalSyntax, CodeTerminal);
document_ast_node!(ContextSendSyntax, ContextSend);
document_ast_node!(EmphasisSyntax, Emphasis);
document_ast_node!(EmptyParagraphSyntax, EmptyParagraph);
document_ast_node!(EnumDefineSyntax, EnumDefine);
document_ast_node!(EnumVariantSyntax, EnumVariant);
document_ast_node!(EnumVariantInlineKindSyntax, EnumVariantInlineKind);
document_ast_node!(EnumVariantKindSyntax, EnumVariantKind);
document_ast_node!(ErrorBlockSyntax, ErrorBlock);
document_ast_node!(EvalInlineMechCodeSyntax, EvalInlineMechCode);
document_ast_node!(FigureItemSyntax, FigureItem);
document_ast_node!(FiguresSyntax, Figures);
document_ast_node!(FiguresRowSyntax, FiguresRow);
document_ast_node!(FloatSyntax, Float);
document_ast_node!(FloatSigilSyntax, FloatSigil);
document_ast_node!(FootnoteSyntax, Footnote);
document_ast_node!(FsmSyntax, Fsm);
document_ast_node!(FsmArmSyntax, FsmArm);
document_ast_node!(FsmBlockTransitionSyntax, FsmBlockTransition);
document_ast_node!(FsmCommentArmSyntax, FsmCommentArm);
document_ast_node!(FsmDeclareSyntax, FsmDeclare);
document_ast_node!(FsmGuardSyntax, FsmGuard);
document_ast_node!(FsmGuardArmSyntax, FsmGuardArm);
document_ast_node!(FsmImplementationSyntax, FsmImplementation);
document_ast_node!(FsmSpecificationSyntax, FsmSpecification);
document_ast_node!(FsmStateDefinitionSyntax, FsmStateDefinition);
document_ast_node!(
    FsmStateDefinitionVariablesSyntax,
    FsmStateDefinitionVariables
);
document_ast_node!(FsmStatementTransitionSyntax, FsmStatementTransition);
document_ast_node!(FsmTransitionSyntax, FsmTransition);
document_ast_node!(FunctionArgSyntax, FunctionArg);
document_ast_node!(FunctionDefineSyntax, FunctionDefine);
document_ast_node!(FunctionDefineMatchArmsSyntax, FunctionDefineMatchArms);
document_ast_node!(FunctionDefineStatementsSyntax, FunctionDefineStatements);
document_ast_node!(FunctionMatchArmSyntax, FunctionMatchArm);
document_ast_node!(FunctionOutArgSyntax, FunctionOutArg);
document_ast_node!(FunctionOutArgsSyntax, FunctionOutArgs);
document_ast_node!(HighlightSyntax, Highlight);
document_ast_node!(HyperlinkSyntax, Hyperlink);
document_ast_node!(IdeaBlockSyntax, IdeaBlock);
document_ast_node!(ImgSyntax, Img);
document_ast_node!(InfoBlockSyntax, InfoBlock);
document_ast_node!(InlineMechCodeSyntax, InlineMechCode);
document_ast_node!(InlineParagraphSyntax, InlineParagraph);
document_ast_node!(InvariantDefineSyntax, InvariantDefine);
document_ast_node!(KindDefineSyntax, KindDefine);
document_ast_node!(LeftAlignmentSyntax, LeftAlignment);
document_ast_node!(MechCodeSyntax, MechCode);
document_ast_node!(MechCodeAltSyntax, MechCodeAlt);
document_ast_node!(MechdownListSyntax, MechdownList);
document_ast_node!(MechdownTableSyntax, MechdownTable);
document_ast_node!(MechdownTableHeaderSyntax, MechdownTableHeader);
document_ast_node!(MechdownTableNoHeaderSyntax, MechdownTableNoHeader);
document_ast_node!(MechdownTableRowSyntax, MechdownTableRow);
document_ast_node!(MechdownTableWithHeaderSyntax, MechdownTableWithHeader);
document_ast_node!(MicroMikaSyntax, MicroMika);
document_ast_node!(MikaSyntax, Mika);
document_ast_node!(MikaArmLeftSyntax, MikaArmLeft);
document_ast_node!(MikaArmRightSyntax, MikaArmRight);
document_ast_node!(MikaExpressionInnerSyntax, MikaExpressionInner);
document_ast_node!(MikaEyeLeftSyntax, MikaEyeLeft);
document_ast_node!(MikaEyeRightSyntax, MikaEyeRight);
document_ast_node!(MikaNoseSyntax, MikaNose);
document_ast_node!(MikaSectionSyntax, MikaSection);
document_ast_node!(MiniMikaSyntax, MiniMika);
document_ast_node!(NoAlignmentSyntax, NoAlignment);
document_ast_node!(NotMechCodeSyntax, NotMechCode);
document_ast_node!(OpAssignSyntax, OpAssign);
document_ast_node!(OptionMapSyntax, OptionMap);
document_ast_node!(OptionMappingSyntax, OptionMapping);
document_ast_node!(OptionValueSyntax, OptionValue);
document_ast_node!(OrderedListSyntax, OrderedList);
document_ast_node!(OrderedListItemSyntax, OrderedListItem);
document_ast_node!(ParagraphElementSyntax, ParagraphElement);
document_ast_node!(ParagraphNewlineSyntax, ParagraphNewline);
document_ast_node!(PromptSyntax, Prompt);
document_ast_node!(QuestionBlockSyntax, QuestionBlock);
document_ast_node!(QuoteBlockSyntax, QuoteBlock);
document_ast_node!(RightAlignmentSyntax, RightAlignment);
document_ast_node!(SectionElementSyntax, SectionElement);
document_ast_node!(SliceRefSyntax, SliceRef);
document_ast_node!(StatementSyntax, Statement);
document_ast_node!(StrikethroughSyntax, Strikethrough);
document_ast_node!(StrongSyntax, Strong);
document_ast_node!(SublistSyntax, Sublist);
document_ast_node!(SubtitleSyntax, Subtitle);
document_ast_node!(SuccessBlockSyntax, SuccessBlock);
document_ast_node!(TitleSyntax, Title);
document_ast_node!(TitleFrontMatterSyntax, TitleFrontMatter);
document_ast_node!(TupleDestructureSyntax, TupleDestructure);
document_ast_node!(UlSubtitleSyntax, UlSubtitle);
document_ast_node!(UncheckedItemSyntax, UncheckedItem);
document_ast_node!(UnderlineSyntax, Underline);
document_ast_node!(UnorderedListSyntax, UnorderedList);
document_ast_node!(UnorderedListItemSyntax, UnorderedListItem);
document_ast_node!(VariableAssignSyntax, VariableAssign);
document_ast_node!(WarningBlockSyntax, WarningBlock);

#[derive(Clone, Debug)]
pub enum CanonicalDocumentNode {
    AbstractEl(AbstractElSyntax),
    ActivationArm(ActivationArmSyntax),
    ActivationScope(ActivationScopeSyntax),
    AlignmentSeparator(AlignmentSeparatorSyntax),
    Body(BodySyntax),
    CenterAlignment(CenterAlignmentSyntax),
    CheckList(CheckListSyntax),
    CheckListItem(CheckListItemSyntax),
    CheckedItem(CheckedItemSyntax),
    Citation(CitationSyntax),
    CodeBlock(CodeBlockSyntax),
    CodeTerminal(CodeTerminalSyntax),
    ContextSend(ContextSendSyntax),
    Emphasis(EmphasisSyntax),
    EmptyParagraph(EmptyParagraphSyntax),
    EnumDefine(EnumDefineSyntax),
    EnumVariant(EnumVariantSyntax),
    EnumVariantInlineKind(EnumVariantInlineKindSyntax),
    EnumVariantKind(EnumVariantKindSyntax),
    ErrorBlock(ErrorBlockSyntax),
    EvalInlineMechCode(EvalInlineMechCodeSyntax),
    FigureItem(FigureItemSyntax),
    Figures(FiguresSyntax),
    FiguresRow(FiguresRowSyntax),
    Float(FloatSyntax),
    FloatSigil(FloatSigilSyntax),
    Footnote(FootnoteSyntax),
    Fsm(FsmSyntax),
    FsmArm(FsmArmSyntax),
    FsmBlockTransition(FsmBlockTransitionSyntax),
    FsmCommentArm(FsmCommentArmSyntax),
    FsmDeclare(FsmDeclareSyntax),
    FsmGuard(FsmGuardSyntax),
    FsmGuardArm(FsmGuardArmSyntax),
    FsmImplementation(FsmImplementationSyntax),
    FsmSpecification(FsmSpecificationSyntax),
    FsmStateDefinition(FsmStateDefinitionSyntax),
    FsmStateDefinitionVariables(FsmStateDefinitionVariablesSyntax),
    FsmStatementTransition(FsmStatementTransitionSyntax),
    FsmTransition(FsmTransitionSyntax),
    FunctionArg(FunctionArgSyntax),
    FunctionDefine(FunctionDefineSyntax),
    FunctionDefineMatchArms(FunctionDefineMatchArmsSyntax),
    FunctionDefineStatements(FunctionDefineStatementsSyntax),
    FunctionMatchArm(FunctionMatchArmSyntax),
    FunctionOutArg(FunctionOutArgSyntax),
    FunctionOutArgs(FunctionOutArgsSyntax),
    Highlight(HighlightSyntax),
    Hyperlink(HyperlinkSyntax),
    IdeaBlock(IdeaBlockSyntax),
    Img(ImgSyntax),
    InfoBlock(InfoBlockSyntax),
    InlineMechCode(InlineMechCodeSyntax),
    InlineParagraph(InlineParagraphSyntax),
    InvariantDefine(InvariantDefineSyntax),
    KindDefine(KindDefineSyntax),
    LeftAlignment(LeftAlignmentSyntax),
    MechCode(MechCodeSyntax),
    MechCodeAlt(MechCodeAltSyntax),
    MechdownList(MechdownListSyntax),
    MechdownTable(MechdownTableSyntax),
    MechdownTableHeader(MechdownTableHeaderSyntax),
    MechdownTableNoHeader(MechdownTableNoHeaderSyntax),
    MechdownTableRow(MechdownTableRowSyntax),
    MechdownTableWithHeader(MechdownTableWithHeaderSyntax),
    MicroMika(MicroMikaSyntax),
    Mika(MikaSyntax),
    MikaArmLeft(MikaArmLeftSyntax),
    MikaArmRight(MikaArmRightSyntax),
    MikaExpressionInner(MikaExpressionInnerSyntax),
    MikaEyeLeft(MikaEyeLeftSyntax),
    MikaEyeRight(MikaEyeRightSyntax),
    MikaNose(MikaNoseSyntax),
    MikaSection(MikaSectionSyntax),
    MiniMika(MiniMikaSyntax),
    NoAlignment(NoAlignmentSyntax),
    NotMechCode(NotMechCodeSyntax),
    OpAssign(OpAssignSyntax),
    OptionMap(OptionMapSyntax),
    OptionMapping(OptionMappingSyntax),
    OptionValue(OptionValueSyntax),
    OrderedList(OrderedListSyntax),
    OrderedListItem(OrderedListItemSyntax),
    ParagraphElement(ParagraphElementSyntax),
    ParagraphNewline(ParagraphNewlineSyntax),
    Prompt(PromptSyntax),
    QuestionBlock(QuestionBlockSyntax),
    QuoteBlock(QuoteBlockSyntax),
    RightAlignment(RightAlignmentSyntax),
    SectionElement(SectionElementSyntax),
    SliceRef(SliceRefSyntax),
    Statement(StatementSyntax),
    Strikethrough(StrikethroughSyntax),
    Strong(StrongSyntax),
    Sublist(SublistSyntax),
    Subtitle(SubtitleSyntax),
    SuccessBlock(SuccessBlockSyntax),
    Title(TitleSyntax),
    TitleFrontMatter(TitleFrontMatterSyntax),
    TupleDestructure(TupleDestructureSyntax),
    UlSubtitle(UlSubtitleSyntax),
    UncheckedItem(UncheckedItemSyntax),
    Underline(UnderlineSyntax),
    UnorderedList(UnorderedListSyntax),
    UnorderedListItem(UnorderedListItemSyntax),
    VariableAssign(VariableAssignSyntax),
    WarningBlock(WarningBlockSyntax),
}

impl CanonicalDocumentNode {
    pub fn cast(node: SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::AbstractEl => AbstractElSyntax::cast(node).map(Self::AbstractEl),
            SyntaxKind::ActivationArm => ActivationArmSyntax::cast(node).map(Self::ActivationArm),
            SyntaxKind::ActivationScope => {
                ActivationScopeSyntax::cast(node).map(Self::ActivationScope)
            }
            SyntaxKind::AlignmentSeparator => {
                AlignmentSeparatorSyntax::cast(node).map(Self::AlignmentSeparator)
            }
            SyntaxKind::Body => BodySyntax::cast(node).map(Self::Body),
            SyntaxKind::CenterAlignment => {
                CenterAlignmentSyntax::cast(node).map(Self::CenterAlignment)
            }
            SyntaxKind::CheckList => CheckListSyntax::cast(node).map(Self::CheckList),
            SyntaxKind::CheckListItem => CheckListItemSyntax::cast(node).map(Self::CheckListItem),
            SyntaxKind::CheckedItem => CheckedItemSyntax::cast(node).map(Self::CheckedItem),
            SyntaxKind::Citation => CitationSyntax::cast(node).map(Self::Citation),
            SyntaxKind::CodeBlock => CodeBlockSyntax::cast(node).map(Self::CodeBlock),
            SyntaxKind::CodeTerminal => CodeTerminalSyntax::cast(node).map(Self::CodeTerminal),
            SyntaxKind::ContextSend => ContextSendSyntax::cast(node).map(Self::ContextSend),
            SyntaxKind::Emphasis => EmphasisSyntax::cast(node).map(Self::Emphasis),
            SyntaxKind::EmptyParagraph => {
                EmptyParagraphSyntax::cast(node).map(Self::EmptyParagraph)
            }
            SyntaxKind::EnumDefine => EnumDefineSyntax::cast(node).map(Self::EnumDefine),
            SyntaxKind::EnumVariant => EnumVariantSyntax::cast(node).map(Self::EnumVariant),
            SyntaxKind::EnumVariantInlineKind => {
                EnumVariantInlineKindSyntax::cast(node).map(Self::EnumVariantInlineKind)
            }
            SyntaxKind::EnumVariantKind => {
                EnumVariantKindSyntax::cast(node).map(Self::EnumVariantKind)
            }
            SyntaxKind::ErrorBlock => ErrorBlockSyntax::cast(node).map(Self::ErrorBlock),
            SyntaxKind::EvalInlineMechCode => {
                EvalInlineMechCodeSyntax::cast(node).map(Self::EvalInlineMechCode)
            }
            SyntaxKind::FigureItem => FigureItemSyntax::cast(node).map(Self::FigureItem),
            SyntaxKind::Figures => FiguresSyntax::cast(node).map(Self::Figures),
            SyntaxKind::FiguresRow => FiguresRowSyntax::cast(node).map(Self::FiguresRow),
            SyntaxKind::Float => FloatSyntax::cast(node).map(Self::Float),
            SyntaxKind::FloatSigil => FloatSigilSyntax::cast(node).map(Self::FloatSigil),
            SyntaxKind::Footnote => FootnoteSyntax::cast(node).map(Self::Footnote),
            SyntaxKind::Fsm => FsmSyntax::cast(node).map(Self::Fsm),
            SyntaxKind::FsmArm => FsmArmSyntax::cast(node).map(Self::FsmArm),
            SyntaxKind::FsmBlockTransition => {
                FsmBlockTransitionSyntax::cast(node).map(Self::FsmBlockTransition)
            }
            SyntaxKind::FsmCommentArm => FsmCommentArmSyntax::cast(node).map(Self::FsmCommentArm),
            SyntaxKind::FsmDeclare => FsmDeclareSyntax::cast(node).map(Self::FsmDeclare),
            SyntaxKind::FsmGuard => FsmGuardSyntax::cast(node).map(Self::FsmGuard),
            SyntaxKind::FsmGuardArm => FsmGuardArmSyntax::cast(node).map(Self::FsmGuardArm),
            SyntaxKind::FsmImplementation => {
                FsmImplementationSyntax::cast(node).map(Self::FsmImplementation)
            }
            SyntaxKind::FsmSpecification => {
                FsmSpecificationSyntax::cast(node).map(Self::FsmSpecification)
            }
            SyntaxKind::FsmStateDefinition => {
                FsmStateDefinitionSyntax::cast(node).map(Self::FsmStateDefinition)
            }
            SyntaxKind::FsmStateDefinitionVariables => {
                FsmStateDefinitionVariablesSyntax::cast(node).map(Self::FsmStateDefinitionVariables)
            }
            SyntaxKind::FsmStatementTransition => {
                FsmStatementTransitionSyntax::cast(node).map(Self::FsmStatementTransition)
            }
            SyntaxKind::FsmTransition => FsmTransitionSyntax::cast(node).map(Self::FsmTransition),
            SyntaxKind::FunctionArg => FunctionArgSyntax::cast(node).map(Self::FunctionArg),
            SyntaxKind::FunctionDefine => {
                FunctionDefineSyntax::cast(node).map(Self::FunctionDefine)
            }
            SyntaxKind::FunctionDefineMatchArms => {
                FunctionDefineMatchArmsSyntax::cast(node).map(Self::FunctionDefineMatchArms)
            }
            SyntaxKind::FunctionDefineStatements => {
                FunctionDefineStatementsSyntax::cast(node).map(Self::FunctionDefineStatements)
            }
            SyntaxKind::FunctionMatchArm => {
                FunctionMatchArmSyntax::cast(node).map(Self::FunctionMatchArm)
            }
            SyntaxKind::FunctionOutArg => {
                FunctionOutArgSyntax::cast(node).map(Self::FunctionOutArg)
            }
            SyntaxKind::FunctionOutArgs => {
                FunctionOutArgsSyntax::cast(node).map(Self::FunctionOutArgs)
            }
            SyntaxKind::Highlight => HighlightSyntax::cast(node).map(Self::Highlight),
            SyntaxKind::Hyperlink => HyperlinkSyntax::cast(node).map(Self::Hyperlink),
            SyntaxKind::IdeaBlock => IdeaBlockSyntax::cast(node).map(Self::IdeaBlock),
            SyntaxKind::Img => ImgSyntax::cast(node).map(Self::Img),
            SyntaxKind::InfoBlock => InfoBlockSyntax::cast(node).map(Self::InfoBlock),
            SyntaxKind::InlineMechCode => {
                InlineMechCodeSyntax::cast(node).map(Self::InlineMechCode)
            }
            SyntaxKind::InlineParagraph => {
                InlineParagraphSyntax::cast(node).map(Self::InlineParagraph)
            }
            SyntaxKind::InvariantDefine => {
                InvariantDefineSyntax::cast(node).map(Self::InvariantDefine)
            }
            SyntaxKind::KindDefine => KindDefineSyntax::cast(node).map(Self::KindDefine),
            SyntaxKind::LeftAlignment => LeftAlignmentSyntax::cast(node).map(Self::LeftAlignment),
            SyntaxKind::MechCode => MechCodeSyntax::cast(node).map(Self::MechCode),
            SyntaxKind::MechCodeAlt => MechCodeAltSyntax::cast(node).map(Self::MechCodeAlt),
            SyntaxKind::MechdownList => MechdownListSyntax::cast(node).map(Self::MechdownList),
            SyntaxKind::MechdownTable => MechdownTableSyntax::cast(node).map(Self::MechdownTable),
            SyntaxKind::MechdownTableHeader => {
                MechdownTableHeaderSyntax::cast(node).map(Self::MechdownTableHeader)
            }
            SyntaxKind::MechdownTableNoHeader => {
                MechdownTableNoHeaderSyntax::cast(node).map(Self::MechdownTableNoHeader)
            }
            SyntaxKind::MechdownTableRow => {
                MechdownTableRowSyntax::cast(node).map(Self::MechdownTableRow)
            }
            SyntaxKind::MechdownTableWithHeader => {
                MechdownTableWithHeaderSyntax::cast(node).map(Self::MechdownTableWithHeader)
            }
            SyntaxKind::MicroMika => MicroMikaSyntax::cast(node).map(Self::MicroMika),
            SyntaxKind::Mika => MikaSyntax::cast(node).map(Self::Mika),
            SyntaxKind::MikaArmLeft => MikaArmLeftSyntax::cast(node).map(Self::MikaArmLeft),
            SyntaxKind::MikaArmRight => MikaArmRightSyntax::cast(node).map(Self::MikaArmRight),
            SyntaxKind::MikaExpressionInner => {
                MikaExpressionInnerSyntax::cast(node).map(Self::MikaExpressionInner)
            }
            SyntaxKind::MikaEyeLeft => MikaEyeLeftSyntax::cast(node).map(Self::MikaEyeLeft),
            SyntaxKind::MikaEyeRight => MikaEyeRightSyntax::cast(node).map(Self::MikaEyeRight),
            SyntaxKind::MikaNose => MikaNoseSyntax::cast(node).map(Self::MikaNose),
            SyntaxKind::MikaSection => MikaSectionSyntax::cast(node).map(Self::MikaSection),
            SyntaxKind::MiniMika => MiniMikaSyntax::cast(node).map(Self::MiniMika),
            SyntaxKind::NoAlignment => NoAlignmentSyntax::cast(node).map(Self::NoAlignment),
            SyntaxKind::NotMechCode => NotMechCodeSyntax::cast(node).map(Self::NotMechCode),
            SyntaxKind::OpAssign => OpAssignSyntax::cast(node).map(Self::OpAssign),
            SyntaxKind::OptionMap => OptionMapSyntax::cast(node).map(Self::OptionMap),
            SyntaxKind::OptionMapping => OptionMappingSyntax::cast(node).map(Self::OptionMapping),
            SyntaxKind::OptionValue => OptionValueSyntax::cast(node).map(Self::OptionValue),
            SyntaxKind::OrderedList => OrderedListSyntax::cast(node).map(Self::OrderedList),
            SyntaxKind::OrderedListItem => {
                OrderedListItemSyntax::cast(node).map(Self::OrderedListItem)
            }
            SyntaxKind::ParagraphElement => {
                ParagraphElementSyntax::cast(node).map(Self::ParagraphElement)
            }
            SyntaxKind::ParagraphNewline => {
                ParagraphNewlineSyntax::cast(node).map(Self::ParagraphNewline)
            }
            SyntaxKind::Prompt => PromptSyntax::cast(node).map(Self::Prompt),
            SyntaxKind::QuestionBlock => QuestionBlockSyntax::cast(node).map(Self::QuestionBlock),
            SyntaxKind::QuoteBlock => QuoteBlockSyntax::cast(node).map(Self::QuoteBlock),
            SyntaxKind::RightAlignment => {
                RightAlignmentSyntax::cast(node).map(Self::RightAlignment)
            }
            SyntaxKind::SectionElement => {
                SectionElementSyntax::cast(node).map(Self::SectionElement)
            }
            SyntaxKind::SliceRef => SliceRefSyntax::cast(node).map(Self::SliceRef),
            SyntaxKind::Statement => StatementSyntax::cast(node).map(Self::Statement),
            SyntaxKind::Strikethrough => StrikethroughSyntax::cast(node).map(Self::Strikethrough),
            SyntaxKind::Strong => StrongSyntax::cast(node).map(Self::Strong),
            SyntaxKind::Sublist => SublistSyntax::cast(node).map(Self::Sublist),
            SyntaxKind::Subtitle => SubtitleSyntax::cast(node).map(Self::Subtitle),
            SyntaxKind::SuccessBlock => SuccessBlockSyntax::cast(node).map(Self::SuccessBlock),
            SyntaxKind::Title => TitleSyntax::cast(node).map(Self::Title),
            SyntaxKind::TitleFrontMatter => {
                TitleFrontMatterSyntax::cast(node).map(Self::TitleFrontMatter)
            }
            SyntaxKind::TupleDestructure => {
                TupleDestructureSyntax::cast(node).map(Self::TupleDestructure)
            }
            SyntaxKind::UlSubtitle => UlSubtitleSyntax::cast(node).map(Self::UlSubtitle),
            SyntaxKind::UncheckedItem => UncheckedItemSyntax::cast(node).map(Self::UncheckedItem),
            SyntaxKind::Underline => UnderlineSyntax::cast(node).map(Self::Underline),
            SyntaxKind::UnorderedList => UnorderedListSyntax::cast(node).map(Self::UnorderedList),
            SyntaxKind::UnorderedListItem => {
                UnorderedListItemSyntax::cast(node).map(Self::UnorderedListItem)
            }
            SyntaxKind::VariableAssign => {
                VariableAssignSyntax::cast(node).map(Self::VariableAssign)
            }
            SyntaxKind::WarningBlock => WarningBlockSyntax::cast(node).map(Self::WarningBlock),
            _ => None,
        }
    }

    pub fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::AbstractEl(node) => node.syntax(),
            Self::ActivationArm(node) => node.syntax(),
            Self::ActivationScope(node) => node.syntax(),
            Self::AlignmentSeparator(node) => node.syntax(),
            Self::Body(node) => node.syntax(),
            Self::CenterAlignment(node) => node.syntax(),
            Self::CheckList(node) => node.syntax(),
            Self::CheckListItem(node) => node.syntax(),
            Self::CheckedItem(node) => node.syntax(),
            Self::Citation(node) => node.syntax(),
            Self::CodeBlock(node) => node.syntax(),
            Self::CodeTerminal(node) => node.syntax(),
            Self::ContextSend(node) => node.syntax(),
            Self::Emphasis(node) => node.syntax(),
            Self::EmptyParagraph(node) => node.syntax(),
            Self::EnumDefine(node) => node.syntax(),
            Self::EnumVariant(node) => node.syntax(),
            Self::EnumVariantInlineKind(node) => node.syntax(),
            Self::EnumVariantKind(node) => node.syntax(),
            Self::ErrorBlock(node) => node.syntax(),
            Self::EvalInlineMechCode(node) => node.syntax(),
            Self::FigureItem(node) => node.syntax(),
            Self::Figures(node) => node.syntax(),
            Self::FiguresRow(node) => node.syntax(),
            Self::Float(node) => node.syntax(),
            Self::FloatSigil(node) => node.syntax(),
            Self::Footnote(node) => node.syntax(),
            Self::Fsm(node) => node.syntax(),
            Self::FsmArm(node) => node.syntax(),
            Self::FsmBlockTransition(node) => node.syntax(),
            Self::FsmCommentArm(node) => node.syntax(),
            Self::FsmDeclare(node) => node.syntax(),
            Self::FsmGuard(node) => node.syntax(),
            Self::FsmGuardArm(node) => node.syntax(),
            Self::FsmImplementation(node) => node.syntax(),
            Self::FsmSpecification(node) => node.syntax(),
            Self::FsmStateDefinition(node) => node.syntax(),
            Self::FsmStateDefinitionVariables(node) => node.syntax(),
            Self::FsmStatementTransition(node) => node.syntax(),
            Self::FsmTransition(node) => node.syntax(),
            Self::FunctionArg(node) => node.syntax(),
            Self::FunctionDefine(node) => node.syntax(),
            Self::FunctionDefineMatchArms(node) => node.syntax(),
            Self::FunctionDefineStatements(node) => node.syntax(),
            Self::FunctionMatchArm(node) => node.syntax(),
            Self::FunctionOutArg(node) => node.syntax(),
            Self::FunctionOutArgs(node) => node.syntax(),
            Self::Highlight(node) => node.syntax(),
            Self::Hyperlink(node) => node.syntax(),
            Self::IdeaBlock(node) => node.syntax(),
            Self::Img(node) => node.syntax(),
            Self::InfoBlock(node) => node.syntax(),
            Self::InlineMechCode(node) => node.syntax(),
            Self::InlineParagraph(node) => node.syntax(),
            Self::InvariantDefine(node) => node.syntax(),
            Self::KindDefine(node) => node.syntax(),
            Self::LeftAlignment(node) => node.syntax(),
            Self::MechCode(node) => node.syntax(),
            Self::MechCodeAlt(node) => node.syntax(),
            Self::MechdownList(node) => node.syntax(),
            Self::MechdownTable(node) => node.syntax(),
            Self::MechdownTableHeader(node) => node.syntax(),
            Self::MechdownTableNoHeader(node) => node.syntax(),
            Self::MechdownTableRow(node) => node.syntax(),
            Self::MechdownTableWithHeader(node) => node.syntax(),
            Self::MicroMika(node) => node.syntax(),
            Self::Mika(node) => node.syntax(),
            Self::MikaArmLeft(node) => node.syntax(),
            Self::MikaArmRight(node) => node.syntax(),
            Self::MikaExpressionInner(node) => node.syntax(),
            Self::MikaEyeLeft(node) => node.syntax(),
            Self::MikaEyeRight(node) => node.syntax(),
            Self::MikaNose(node) => node.syntax(),
            Self::MikaSection(node) => node.syntax(),
            Self::MiniMika(node) => node.syntax(),
            Self::NoAlignment(node) => node.syntax(),
            Self::NotMechCode(node) => node.syntax(),
            Self::OpAssign(node) => node.syntax(),
            Self::OptionMap(node) => node.syntax(),
            Self::OptionMapping(node) => node.syntax(),
            Self::OptionValue(node) => node.syntax(),
            Self::OrderedList(node) => node.syntax(),
            Self::OrderedListItem(node) => node.syntax(),
            Self::ParagraphElement(node) => node.syntax(),
            Self::ParagraphNewline(node) => node.syntax(),
            Self::Prompt(node) => node.syntax(),
            Self::QuestionBlock(node) => node.syntax(),
            Self::QuoteBlock(node) => node.syntax(),
            Self::RightAlignment(node) => node.syntax(),
            Self::SectionElement(node) => node.syntax(),
            Self::SliceRef(node) => node.syntax(),
            Self::Statement(node) => node.syntax(),
            Self::Strikethrough(node) => node.syntax(),
            Self::Strong(node) => node.syntax(),
            Self::Sublist(node) => node.syntax(),
            Self::Subtitle(node) => node.syntax(),
            Self::SuccessBlock(node) => node.syntax(),
            Self::Title(node) => node.syntax(),
            Self::TitleFrontMatter(node) => node.syntax(),
            Self::TupleDestructure(node) => node.syntax(),
            Self::UlSubtitle(node) => node.syntax(),
            Self::UncheckedItem(node) => node.syntax(),
            Self::Underline(node) => node.syntax(),
            Self::UnorderedList(node) => node.syntax(),
            Self::UnorderedListItem(node) => node.syntax(),
            Self::VariableAssign(node) => node.syntax(),
            Self::WarningBlock(node) => node.syntax(),
        }
    }
}
