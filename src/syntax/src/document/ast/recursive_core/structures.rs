use alloc::vec::Vec;

use crate::document::{
    AstNode, EmptyMapSyntax, EmptySetSyntax, ExpressionSyntax, IdentifierSyntax, SyntaxKind,
    SyntaxNode, SyntaxToken,
};

use super::{KindAnnotationSyntax, child, children, direct_token, direct_tokens, nth_child};

recursive_ast_node!(StructureSyntax, Structure);
recursive_ast_node!(MatrixSyntax, Matrix);
recursive_ast_node!(MatrixRowSyntax, MatrixRow);
recursive_ast_node!(MatrixColumnSyntax, MatrixColumn);
recursive_ast_node!(TableSyntax, Table);
recursive_ast_node!(FancyTableSyntax, FancyTable);
recursive_ast_node!(FancyTableHeaderSyntax, FancyTableHeader);
recursive_ast_node!(FancyTableRowSyntax, FancyTableRow);
recursive_ast_node!(InlineTableSyntax, InlineTable);
recursive_ast_node!(InlineTableHeaderSyntax, InlineTableHeader);
recursive_ast_node!(InlineTableRowSyntax, InlineTableRow);
recursive_ast_node!(RegularTableSyntax, RegularTable);
recursive_ast_node!(TableHeaderSyntax, TableHeader);
recursive_ast_node!(TableRowSyntax, TableRow);
recursive_ast_node!(HeaderFieldSyntax, HeaderField);
recursive_ast_node!(TableFieldSyntax, TableField);
recursive_ast_node!(MapSyntax, Map);
recursive_ast_node!(MapEntrySyntax, MapEntry);
recursive_ast_node!(RecordSyntax, Record);
recursive_ast_node!(RecordBindingSyntax, RecordBinding);
recursive_ast_node!(SetSyntax, Set);
recursive_ast_node!(TupleSyntax, Tuple);
recursive_ast_node!(TupleStructSyntax, TupleStruct);

#[derive(Clone, Debug)]
pub enum StructureValueSyntax {
    Matrix(MatrixSyntax),
    Table(TableSyntax),
    Map(MapSyntax),
    Record(RecordSyntax),
    Set(SetSyntax),
    Tuple(TupleSyntax),
    TupleStruct(TupleStructSyntax),
    EmptyMap(EmptyMapSyntax),
    EmptySet(EmptySetSyntax),
}

impl AstNode for StructureValueSyntax {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::Matrix
                | SyntaxKind::Table
                | SyntaxKind::Map
                | SyntaxKind::Record
                | SyntaxKind::Set
                | SyntaxKind::Tuple
                | SyntaxKind::TupleStruct
                | SyntaxKind::EmptyMap
                | SyntaxKind::EmptySet
        )
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        match syntax.kind() {
            SyntaxKind::Matrix => MatrixSyntax::cast(syntax).map(Self::Matrix),
            SyntaxKind::Table => TableSyntax::cast(syntax).map(Self::Table),
            SyntaxKind::Map => MapSyntax::cast(syntax).map(Self::Map),
            SyntaxKind::Record => RecordSyntax::cast(syntax).map(Self::Record),
            SyntaxKind::Set => SetSyntax::cast(syntax).map(Self::Set),
            SyntaxKind::Tuple => TupleSyntax::cast(syntax).map(Self::Tuple),
            SyntaxKind::TupleStruct => TupleStructSyntax::cast(syntax).map(Self::TupleStruct),
            SyntaxKind::EmptyMap => EmptyMapSyntax::cast(syntax).map(Self::EmptyMap),
            SyntaxKind::EmptySet => EmptySetSyntax::cast(syntax).map(Self::EmptySet),
            _ => None,
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::Matrix(value) => value.syntax(),
            Self::Table(value) => value.syntax(),
            Self::Map(value) => value.syntax(),
            Self::Record(value) => value.syntax(),
            Self::Set(value) => value.syntax(),
            Self::Tuple(value) => value.syntax(),
            Self::TupleStruct(value) => value.syntax(),
            Self::EmptyMap(value) => value.syntax(),
            Self::EmptySet(value) => value.syntax(),
        }
    }
}

impl StructureSyntax {
    pub fn value(&self) -> Option<StructureValueSyntax> {
        child(&self.0)
    }
}

impl MatrixSyntax {
    pub fn rows(&self) -> Vec<MatrixRowSyntax> {
        children(&self.0)
    }
    pub fn opening_delimiter(&self) -> Option<SyntaxToken> {
        direct_tokens(&self.0).into_iter().find(|token| {
            matches!(
                token.kind(),
                SyntaxKind::LeftBracket | SyntaxKind::BoxDrawing
            )
        })
    }
    pub fn closing_delimiter(&self) -> Option<SyntaxToken> {
        direct_tokens(&self.0)
            .into_iter()
            .filter(|token| {
                matches!(
                    token.kind(),
                    SyntaxKind::RightBracket | SyntaxKind::BoxDrawing
                )
            })
            .last()
    }
}

impl MatrixRowSyntax {
    pub fn columns(&self) -> Vec<MatrixColumnSyntax> {
        children(&self.0)
    }
}

impl MatrixColumnSyntax {
    pub fn value(&self) -> Option<ExpressionSyntax> {
        child(&self.0)
    }
}

#[derive(Clone, Debug)]
pub enum TableValueSyntax {
    Fancy(FancyTableSyntax),
    Inline(InlineTableSyntax),
    Regular(RegularTableSyntax),
}

impl AstNode for TableValueSyntax {
    fn can_cast(kind: SyntaxKind) -> bool {
        matches!(
            kind,
            SyntaxKind::FancyTable | SyntaxKind::InlineTable | SyntaxKind::RegularTable
        )
    }
    fn cast(syntax: SyntaxNode) -> Option<Self> {
        match syntax.kind() {
            SyntaxKind::FancyTable => FancyTableSyntax::cast(syntax).map(Self::Fancy),
            SyntaxKind::InlineTable => InlineTableSyntax::cast(syntax).map(Self::Inline),
            SyntaxKind::RegularTable => RegularTableSyntax::cast(syntax).map(Self::Regular),
            _ => None,
        }
    }
    fn syntax(&self) -> &SyntaxNode {
        match self {
            Self::Fancy(value) => value.syntax(),
            Self::Inline(value) => value.syntax(),
            Self::Regular(value) => value.syntax(),
        }
    }
}

impl TableSyntax {
    pub fn value(&self) -> Option<TableValueSyntax> {
        child(&self.0)
    }
}

impl FancyTableSyntax {
    pub fn header(&self) -> Option<FancyTableHeaderSyntax> {
        child(&self.0)
    }
    pub fn rows(&self) -> Vec<FancyTableRowSyntax> {
        children(&self.0)
    }
}

impl InlineTableSyntax {
    pub fn header(&self) -> Option<InlineTableHeaderSyntax> {
        child(&self.0)
    }
    pub fn rows(&self) -> Vec<InlineTableRowSyntax> {
        children(&self.0)
    }
}

impl RegularTableSyntax {
    pub fn header(&self) -> Option<TableHeaderSyntax> {
        child(&self.0)
    }
    pub fn rows(&self) -> Vec<TableRowSyntax> {
        children(&self.0)
    }
}

macro_rules! field_list {
    ($name:ident, $field:ty) => {
        impl $name {
            pub fn fields(&self) -> Vec<$field> {
                children(&self.0)
            }
        }
    };
}

field_list!(FancyTableHeaderSyntax, TableFieldSyntax);
field_list!(InlineTableHeaderSyntax, HeaderFieldSyntax);
field_list!(TableHeaderSyntax, HeaderFieldSyntax);

macro_rules! expression_row {
    ($name:ident) => {
        impl $name {
            pub fn cells(&self) -> Vec<ExpressionSyntax> {
                children(&self.0)
            }
        }
    };
}

expression_row!(FancyTableRowSyntax);
expression_row!(InlineTableRowSyntax);
expression_row!(TableRowSyntax);

macro_rules! named_field {
    ($name:ident) => {
        impl $name {
            pub fn name(&self) -> Option<IdentifierSyntax> {
                child(&self.0)
            }
            pub fn annotation(&self) -> Option<KindAnnotationSyntax> {
                child(&self.0)
            }
        }
    };
}

named_field!(HeaderFieldSyntax);
named_field!(TableFieldSyntax);

impl MapSyntax {
    pub fn opening_brace(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::LeftBrace, 0)
    }
    pub fn entries(&self) -> Vec<MapEntrySyntax> {
        children(&self.0)
    }
    pub fn closing_brace(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::RightBrace, 0)
    }
}

impl MapEntrySyntax {
    pub fn key(&self) -> Option<ExpressionSyntax> {
        nth_child(&self.0, 0)
    }
    pub fn colon(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::Colon, 0)
    }
    pub fn value(&self) -> Option<ExpressionSyntax> {
        nth_child(&self.0, 1)
    }
}

impl RecordSyntax {
    pub fn bindings(&self) -> Vec<RecordBindingSyntax> {
        children(&self.0)
    }
}

impl RecordBindingSyntax {
    pub fn name(&self) -> Option<IdentifierSyntax> {
        child(&self.0)
    }
    pub fn annotation(&self) -> Option<KindAnnotationSyntax> {
        child(&self.0)
    }
    pub fn colon(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::Colon, 0)
    }
    pub fn value(&self) -> Option<ExpressionSyntax> {
        child(&self.0)
    }
}

impl SetSyntax {
    pub fn opening_brace(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::LeftBrace, 0)
    }
    pub fn items(&self) -> Vec<ExpressionSyntax> {
        children(&self.0)
    }
    pub fn closing_brace(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::RightBrace, 0)
    }
}

impl TupleSyntax {
    pub fn opening_parenthesis(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::LeftParen, 0)
    }
    pub fn items(&self) -> Vec<ExpressionSyntax> {
        children(&self.0)
    }
    pub fn closing_parenthesis(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::RightParen, 0)
    }
}

impl TupleStructSyntax {
    pub fn prefix(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::Colon, 0)
    }
    pub fn name(&self) -> Option<IdentifierSyntax> {
        child(&self.0)
    }
    pub fn opening_parenthesis(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::LeftParen, 0)
    }
    pub fn value(&self) -> Option<ExpressionSyntax> {
        child(&self.0)
    }
    pub fn closing_parenthesis(&self) -> Option<SyntaxToken> {
        direct_token(&self.0, SyntaxKind::RightParen, 0)
    }
}
