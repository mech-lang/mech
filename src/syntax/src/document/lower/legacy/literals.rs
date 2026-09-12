//! Compatibility lowering for the closed Phase 2C literal and number island.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use mech_core::nodes::{
    Atom, C64Node, ImaginaryNumber, Kind, KindAnnotation, MechString, Number, RealNumber,
};
use mech_core::{Token as LegacyToken, TokenKind};

use crate::document::ast::literals::{
    AtomLiteralSyntax, BinaryLiteralSyntax, DecimalLiteralSyntax, EmptyLiteralSyntax,
    FloatDecimalStartSyntax, FloatFullSyntax, FloatLiteralSyntax, HexadecimalLiteralSyntax,
    IntegerLiteralSyntax, NumberSyntax, OctalLiteralSyntax, RationalLiteralSyntax, RawStringSyntax,
    RealNumberSyntax, ScientificLiteralSyntax, StringLiteralSyntax, TypedIntegerSyntax,
    UntypedIntegerSyntax, UntypedRealNumberSyntax, Utf8StringSyntax,
};
use crate::document::{
    AstNode, DiagnosticStore, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken,
};

use super::base::{
    lower_legacy_digit_sequence, lower_legacy_escaped_character, lower_legacy_identifier,
};
use super::common;

type LowerResult<T> = Result<T, String>;

/// Lower an underscore-run empty literal to the legacy `Empty` token.
pub fn lower_legacy_empty(syntax: &EmptyLiteralSyntax) -> Result<LegacyToken, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::EmptyLiteral,
        "empty",
        "lowering/invalid-empty-syntax",
        lower_empty_node,
    )
}

/// Lower a closed atom literal.
pub fn lower_legacy_atom(syntax: &AtomLiteralSyntax) -> Result<Atom, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::AtomLiteral,
        "atom",
        "lowering/invalid-atom-syntax",
        lower_atom_node,
    )
}

/// Lower a `string` selection node.
pub fn lower_legacy_string(syntax: &StringLiteralSyntax) -> Result<MechString, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::StringLiteral,
        "string",
        "lowering/invalid-string-syntax",
        lower_string_node,
    )
}

/// Lower an exact UTF-8 string node.
pub fn lower_legacy_utf8_string(syntax: &Utf8StringSyntax) -> Result<MechString, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::Utf8String,
        "utf8-string",
        "lowering/invalid-utf8-string-syntax",
        lower_utf8_string_node,
    )
}

/// Lower an exact raw-string node.
pub fn lower_legacy_raw_string(syntax: &RawStringSyntax) -> Result<MechString, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::RawString,
        "raw-string",
        "lowering/invalid-raw-string-syntax",
        lower_raw_string_node,
    )
}

/// Lower a `number` selection node.
pub fn lower_legacy_number(syntax: &NumberSyntax) -> Result<Number, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::Number,
        "number",
        "lowering/invalid-number-syntax",
        lower_number_node,
    )
}

/// Lower an exact complex-number node.
pub fn lower_legacy_complex_number(
    syntax: &crate::document::ast::literals::ComplexNumberSyntax,
) -> Result<C64Node, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::ComplexNumber,
        "complex-number",
        "lowering/invalid-complex-number-syntax",
        lower_complex_number_node,
    )
}

/// Lower an exact real-number node.
pub fn lower_legacy_real_number(syntax: &RealNumberSyntax) -> Result<RealNumber, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::RealNumber,
        "real-number",
        "lowering/invalid-real-number-syntax",
        |node| lower_real_container(node, false),
    )
}

/// Lower an exact untyped-real-number node.
pub fn lower_legacy_untyped_real_number(
    syntax: &UntypedRealNumberSyntax,
) -> Result<RealNumber, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::UntypedRealNumber,
        "untyped-real-number",
        "lowering/invalid-untyped-real-number-syntax",
        |node| lower_real_container(node, true),
    )
}

/// Lower a rational literal, discarding integer type annotations as legacy
/// compatibility requires.
pub fn lower_legacy_rational_literal(
    syntax: &RationalLiteralSyntax,
) -> Result<RealNumber, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::RationalLiteral,
        "rational-literal",
        "lowering/invalid-rational-literal-syntax",
        lower_rational_literal_node,
    )
}

/// Lower a scientific literal. Typed integer annotations in either component
/// are intentionally discarded, including the canonical typed-exponent case.
pub fn lower_legacy_scientific_literal(
    syntax: &ScientificLiteralSyntax,
) -> Result<RealNumber, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::ScientificLiteral,
        "scientific-literal",
        "lowering/invalid-scientific-literal-syntax",
        lower_scientific_literal_node,
    )
}

/// Lower `.digits` to the existing floating-number representation.
pub fn lower_legacy_float_decimal_start(
    syntax: &FloatDecimalStartSyntax,
) -> Result<RealNumber, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::FloatDecimalStart,
        "float-decimal-start",
        "lowering/invalid-float-decimal-start-syntax",
        lower_float_decimal_start_node,
    )
}

/// Lower `digits.digits` to the existing floating-number representation.
pub fn lower_legacy_float_full(syntax: &FloatFullSyntax) -> Result<RealNumber, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::FloatFull,
        "float-full",
        "lowering/invalid-float-full-syntax",
        lower_float_full_node,
    )
}

/// Lower a `float-literal` selection node.
pub fn lower_legacy_float_literal(
    syntax: &FloatLiteralSyntax,
) -> Result<RealNumber, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::FloatLiteral,
        "float-literal",
        "lowering/invalid-float-literal-syntax",
        lower_float_literal_node,
    )
}

/// Lower an integer-literal selection node.
pub fn lower_legacy_integer_literal(
    syntax: &IntegerLiteralSyntax,
) -> Result<RealNumber, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::IntegerLiteral,
        "integer-literal",
        "lowering/invalid-integer-literal-syntax",
        lower_integer_literal_node,
    )
}

/// Lower a typed integer without validating its suffix against enabled kinds.
pub fn lower_legacy_typed_integer(
    syntax: &TypedIntegerSyntax,
) -> Result<RealNumber, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::TypedInteger,
        "typed-integer",
        "lowering/invalid-typed-integer-syntax",
        lower_typed_integer_node,
    )
}

/// Lower an untyped integer.
pub fn lower_legacy_untyped_integer(
    syntax: &UntypedIntegerSyntax,
) -> Result<RealNumber, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::UntypedInteger,
        "untyped-integer",
        "lowering/invalid-untyped-integer-syntax",
        lower_untyped_integer_node,
    )
}

/// Lower a decimal based literal.
pub fn lower_legacy_decimal_literal(
    syntax: &DecimalLiteralSyntax,
) -> Result<RealNumber, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::DecimalLiteral,
        "decimal-literal",
        "lowering/invalid-decimal-literal-syntax",
        |node| {
            lower_digit_based_literal(node, SyntaxKind::DecimalLiteral, "0d", RealNumber::Decimal)
        },
    )
}

/// Lower a permissive hexadecimal literal.
pub fn lower_legacy_hexadecimal_literal(
    syntax: &HexadecimalLiteralSyntax,
) -> Result<RealNumber, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::HexadecimalLiteral,
        "hexadecimal-literal",
        "lowering/invalid-hexadecimal-literal-syntax",
        lower_hexadecimal_literal_node,
    )
}

/// Lower an octal based literal.
pub fn lower_legacy_octal_literal(
    syntax: &OctalLiteralSyntax,
) -> Result<RealNumber, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::OctalLiteral,
        "octal-literal",
        "lowering/invalid-octal-literal-syntax",
        |node| lower_digit_based_literal(node, SyntaxKind::OctalLiteral, "0o", RealNumber::Octal),
    )
}

/// Lower a binary based literal.
pub fn lower_legacy_binary_literal(
    syntax: &BinaryLiteralSyntax,
) -> Result<RealNumber, DiagnosticStore> {
    lower_value(
        syntax.syntax(),
        SyntaxKind::BinaryLiteral,
        "binary-literal",
        "lowering/invalid-binary-literal-syntax",
        |node| lower_digit_based_literal(node, SyntaxKind::BinaryLiteral, "0b", RealNumber::Binary),
    )
}

fn lower_value<T>(
    syntax: &SyntaxNode,
    expected_kind: SyntaxKind,
    name: &str,
    code: &str,
    lower: impl FnOnce(&SyntaxNode) -> LowerResult<T>,
) -> Result<T, DiagnosticStore> {
    let lowered = (|| {
        common::validate_node(syntax, expected_kind, name)?;
        lower(syntax)
    })();
    lowered.map_err(|message| common::failure_store(syntax, code, message))
}

fn lower_empty_node(syntax: &SyntaxNode) -> LowerResult<LegacyToken> {
    let tokens = common::direct_tokens(syntax, "empty")?;
    if tokens.is_empty() {
        return Err(String::from(
            "empty syntax requires at least one underscore",
        ));
    }
    let mut lowered = Vec::with_capacity(tokens.len());
    for token in &tokens {
        require_token(token, SyntaxKind::Underscore, "_")?;
        lowered.push(common::lower_syntax_token(syntax, token, TokenKind::Empty)?);
    }
    let mut merged = common::merge_legacy_tokens(&mut lowered, "empty literal")?;
    merged.kind = TokenKind::Empty;
    Ok(merged)
}

fn lower_atom_node(syntax: &SyntaxNode) -> LowerResult<Atom> {
    let elements = syntax.children_with_tokens();
    if elements.len() != 2 {
        return Err(String::from("atom syntax requires a colon and identifier"));
    }
    let SyntaxElement::Token(colon) = &elements[0] else {
        return Err(String::from("atom syntax requires a direct colon token"));
    };
    require_token(colon, SyntaxKind::Colon, ":")?;
    let SyntaxElement::Node(identifier) = &elements[1] else {
        return Err(String::from("atom syntax requires an identifier node"));
    };
    if identifier.kind() != SyntaxKind::Identifier {
        return Err(String::from("atom syntax requires an identifier node"));
    }
    let name = lower_legacy_identifier(identifier)
        .map_err(|_| String::from("atom syntax contains an invalid identifier"))?;
    Ok(Atom { name })
}

fn lower_string_node(syntax: &SyntaxNode) -> LowerResult<MechString> {
    let child = only_child(syntax, "string")?;
    match child.kind() {
        SyntaxKind::Utf8String => lower_utf8_string_node(&child),
        SyntaxKind::RawString => lower_raw_string_node(&child),
        _ => Err(String::from(
            "string syntax requires a UTF-8 or raw string child",
        )),
    }
}

fn lower_utf8_string_node(syntax: &SyntaxNode) -> LowerResult<MechString> {
    common::validate_node(syntax, SyntaxKind::Utf8String, "utf8-string")?;
    let elements = syntax.children_with_tokens();
    if elements.len() < 2 {
        return Err(String::from(
            "UTF-8 string syntax requires opening and closing quotes",
        ));
    }
    require_element_token(&elements[0], SyntaxKind::Quote, "\"")?;
    require_element_token(
        elements
            .last()
            .expect("a two-element list has a last element"),
        SyntaxKind::Quote,
        "\"",
    )?;

    let mut content = Vec::new();
    for element in &elements[1..elements.len() - 1] {
        match element {
            SyntaxElement::Token(token) => {
                content.push(common::lower_syntax_token(syntax, token, TokenKind::Text)?);
            }
            SyntaxElement::Node(node) if node.kind() == SyntaxKind::EscapedCharacter => {
                content.push(lower_legacy_escaped_character(node).map_err(|_| {
                    String::from("UTF-8 string syntax contains an invalid escaped character")
                })?);
            }
            SyntaxElement::Node(_) => {
                return Err(String::from(
                    "UTF-8 string syntax contains an unexpected child node",
                ));
            }
        }
    }
    Ok(MechString {
        text: string_content_token(&mut content)?,
    })
}

fn lower_raw_string_node(syntax: &SyntaxNode) -> LowerResult<MechString> {
    common::validate_node(syntax, SyntaxKind::RawString, "raw-string")?;
    let elements = syntax.children_with_tokens();
    if elements.len() < 6 {
        return Err(String::from(
            "raw string syntax requires two triple-quote delimiters",
        ));
    }
    for element in &elements[..3] {
        require_element_token(element, SyntaxKind::Quote, "\"")?;
    }
    for element in &elements[elements.len() - 3..] {
        require_element_token(element, SyntaxKind::Quote, "\"")?;
    }

    let mut content = Vec::new();
    for element in &elements[3..elements.len() - 3] {
        let SyntaxElement::Token(token) = element else {
            return Err(String::from(
                "raw string syntax cannot contain child nodes in its content",
            ));
        };
        content.push(common::lower_syntax_token(syntax, token, TokenKind::Text)?);
    }
    Ok(MechString {
        text: string_content_token(&mut content)?,
    })
}

fn string_content_token(content: &mut Vec<LegacyToken>) -> LowerResult<LegacyToken> {
    if content.is_empty() {
        let mut empty = LegacyToken::default();
        empty.kind = TokenKind::String;
        return Ok(empty);
    }
    let mut token = common::merge_legacy_tokens(content, "string content")?;
    token.kind = TokenKind::String;
    Ok(token)
}

fn lower_number_node(syntax: &SyntaxNode) -> LowerResult<Number> {
    let child = only_child(syntax, "number")?;
    match child.kind() {
        SyntaxKind::ComplexNumber => Ok(Number::Complex(lower_complex_number_node(&child)?)),
        SyntaxKind::RealNumber => Ok(Number::Real(lower_real_container(&child, false)?)),
        _ => Err(String::from(
            "number syntax requires a complex-number or real-number child",
        )),
    }
}

fn lower_complex_number_node(syntax: &SyntaxNode) -> LowerResult<C64Node> {
    common::validate_node(syntax, SyntaxKind::ComplexNumber, "complex-number")?;
    let elements = syntax.children_with_tokens();
    let mut values = Vec::new();
    let mut sign = None;
    let mut unit = false;

    for element in &elements {
        match element {
            SyntaxElement::Node(node) if node.kind() == SyntaxKind::UntypedRealNumber => {
                values.push(lower_real_container(node, true)?);
            }
            SyntaxElement::Token(token) if token.kind() == SyntaxKind::Text => {
                common::validate_token(token)?;
                let text = token
                    .text()
                    .map_err(|_| String::from("cannot read imaginary-unit source"))?;
                if !matches!(text.as_str(), "i" | "j") || unit {
                    return Err(String::from(
                        "complex-number syntax contains an invalid imaginary unit",
                    ));
                }
                unit = true;
            }
            SyntaxElement::Token(token)
                if matches!(token.kind(), SyntaxKind::Plus | SyntaxKind::Dash) =>
            {
                common::validate_token(token)?;
                if sign.replace(token.kind()).is_some() {
                    return Err(String::from(
                        "complex-number syntax contains more than one direct sign",
                    ));
                }
            }
            _ => {
                return Err(String::from(
                    "complex-number syntax contains an unexpected direct element",
                ));
            }
        }
    }

    if !unit {
        return Err(String::from(
            "complex-number syntax requires an imaginary unit",
        ));
    }
    match (values.len(), sign) {
        (1, None) => Ok(C64Node {
            real: None,
            imaginary: ImaginaryNumber {
                number: values.remove(0),
            },
        }),
        (2, Some(kind)) => {
            let real = values.remove(0);
            let imaginary = values.remove(0);
            let imaginary = if kind == SyntaxKind::Dash {
                RealNumber::Negated(Box::new(imaginary))
            } else {
                imaginary
            };
            Ok(C64Node {
                real: Some(real),
                imaginary: ImaginaryNumber { number: imaginary },
            })
        }
        _ => Err(String::from(
            "complex-number syntax has an invalid component/sign arrangement",
        )),
    }
}

fn lower_real_container(syntax: &SyntaxNode, untyped: bool) -> LowerResult<RealNumber> {
    common::validate_node(
        syntax,
        if untyped {
            SyntaxKind::UntypedRealNumber
        } else {
            SyntaxKind::RealNumber
        },
        if untyped {
            "untyped-real-number"
        } else {
            "real-number"
        },
    )?;
    let elements = syntax.children_with_tokens();
    let mut negative = false;
    let mut value = None;

    for element in &elements {
        match element {
            SyntaxElement::Token(token) if token.kind() == SyntaxKind::Dash => {
                require_token(token, SyntaxKind::Dash, "-")?;
                if negative {
                    return Err(String::from(
                        "real-number syntax contains more than one direct dash",
                    ));
                }
                negative = true;
            }
            SyntaxElement::Node(node) => {
                if value
                    .replace(lower_real_value_node(node, untyped)?)
                    .is_some()
                {
                    return Err(String::from(
                        "real-number syntax contains more than one value child",
                    ));
                }
            }
            _ => {
                return Err(String::from(
                    "real-number syntax contains an unexpected direct token",
                ));
            }
        }
    }

    let value = value.ok_or_else(|| String::from("real-number syntax requires a value child"))?;
    Ok(if negative {
        RealNumber::Negated(Box::new(value))
    } else {
        value
    })
}

fn lower_real_value_node(syntax: &SyntaxNode, untyped: bool) -> LowerResult<RealNumber> {
    match syntax.kind() {
        SyntaxKind::HexadecimalLiteral => lower_hexadecimal_literal_node(syntax),
        SyntaxKind::DecimalLiteral => lower_digit_based_literal(
            syntax,
            SyntaxKind::DecimalLiteral,
            "0d",
            RealNumber::Decimal,
        ),
        SyntaxKind::OctalLiteral => {
            lower_digit_based_literal(syntax, SyntaxKind::OctalLiteral, "0o", RealNumber::Octal)
        }
        SyntaxKind::BinaryLiteral => {
            lower_digit_based_literal(syntax, SyntaxKind::BinaryLiteral, "0b", RealNumber::Binary)
        }
        SyntaxKind::ScientificLiteral => lower_scientific_literal_node(syntax),
        SyntaxKind::RationalLiteral => lower_rational_literal_node(syntax),
        SyntaxKind::FloatLiteral => lower_float_literal_node(syntax),
        SyntaxKind::IntegerLiteral if !untyped => lower_integer_literal_node(syntax),
        SyntaxKind::UntypedInteger if untyped => lower_untyped_integer_node(syntax),
        _ => Err(String::from(
            "real-number syntax contains a value outside its selected grammar branch",
        )),
    }
}

fn lower_rational_literal_node(syntax: &SyntaxNode) -> LowerResult<RealNumber> {
    common::validate_node(syntax, SyntaxKind::RationalLiteral, "rational-literal")?;
    let elements = syntax.children_with_tokens();
    if elements.len() != 3 {
        return Err(String::from(
            "rational-literal syntax requires numerator, slash, denominator",
        ));
    }
    let numerator = require_integer_element(&elements[0])?;
    require_element_token(&elements[1], SyntaxKind::Slash, "/")?;
    let denominator = require_integer_element(&elements[2])?;
    Ok(RealNumber::Rational((
        integer_numeric_token(numerator)?,
        integer_numeric_token(denominator)?,
    )))
}

fn lower_scientific_literal_node(syntax: &SyntaxNode) -> LowerResult<RealNumber> {
    common::validate_node(syntax, SyntaxKind::ScientificLiteral, "scientific-literal")?;
    let elements = syntax.children_with_tokens();
    if elements.len() < 3 {
        return Err(String::from(
            "scientific-literal syntax requires base, marker, and exponent",
        ));
    }

    let base = require_scientific_component(&elements[0])?;
    require_scientific_marker(&elements[1])?;
    let mut index = 2;
    if elements
        .get(index)
        .is_some_and(|element| is_element_token(element, SyntaxKind::Plus))
    {
        require_element_token(&elements[index], SyntaxKind::Plus, "+")?;
        index += 1;
    }
    let mut negative = false;
    if elements
        .get(index)
        .is_some_and(|element| is_element_token(element, SyntaxKind::Dash))
    {
        require_element_token(&elements[index], SyntaxKind::Dash, "-")?;
        negative = true;
        index += 1;
    }
    let exponent = elements
        .get(index)
        .ok_or_else(|| String::from("scientific-literal syntax is missing its exponent"))?;
    let exponent = require_scientific_component(exponent)?;
    if index + 1 != elements.len() {
        return Err(String::from(
            "scientific-literal syntax contains trailing direct elements",
        ));
    }

    Ok(RealNumber::Scientific((
        base,
        (negative, exponent.0, exponent.1),
    )))
}

fn require_scientific_marker(element: &SyntaxElement) -> LowerResult<()> {
    let SyntaxElement::Token(token) = element else {
        return Err(String::from(
            "scientific-literal syntax requires a direct exponent marker",
        ));
    };
    if token.kind() != SyntaxKind::Text {
        return Err(String::from(
            "scientific-literal exponent marker must be an anonymous text token",
        ));
    }
    common::validate_token(token)?;
    let text = token
        .text()
        .map_err(|_| String::from("cannot read scientific exponent marker"))?;
    matches!(text.as_str(), "e" | "E")
        .then_some(())
        .ok_or_else(|| String::from("scientific-literal syntax has an invalid exponent marker"))
}

fn require_scientific_component(
    element: &SyntaxElement,
) -> LowerResult<(LegacyToken, LegacyToken)> {
    let SyntaxElement::Node(node) = element else {
        return Err(String::from(
            "scientific-literal syntax requires a floating or integer component",
        ));
    };
    match node.kind() {
        SyntaxKind::FloatLiteral => match lower_float_literal_node(node)? {
            RealNumber::Float(value) => Ok(value),
            _ => unreachable!("float-literal lowerer always returns the float variant"),
        },
        SyntaxKind::IntegerLiteral => match lower_integer_literal_node(node)? {
            RealNumber::Integer(value) => Ok((value, LegacyToken::default())),
            RealNumber::TypedInteger((value, _)) => Ok((value, LegacyToken::default())),
            _ => Err(String::from(
                "integer-literal syntax lowered to an invalid scientific component",
            )),
        },
        _ => Err(String::from(
            "scientific-literal syntax requires a floating or integer component",
        )),
    }
}

fn lower_float_literal_node(syntax: &SyntaxNode) -> LowerResult<RealNumber> {
    common::validate_node(syntax, SyntaxKind::FloatLiteral, "float-literal")?;
    let child = only_child(syntax, "float-literal")?;
    match child.kind() {
        SyntaxKind::FloatDecimalStart => lower_float_decimal_start_node(&child),
        SyntaxKind::FloatFull => lower_float_full_node(&child),
        _ => Err(String::from(
            "float-literal syntax requires a supported float child",
        )),
    }
}

fn lower_float_decimal_start_node(syntax: &SyntaxNode) -> LowerResult<RealNumber> {
    common::validate_node(syntax, SyntaxKind::FloatDecimalStart, "float-decimal-start")?;
    let elements = syntax.children_with_tokens();
    if elements.len() != 2 {
        return Err(String::from(
            "float-decimal-start syntax requires a period and digit sequence",
        ));
    }
    require_element_token(&elements[0], SyntaxKind::Period, ".")?;
    let part = require_digit_sequence_element(&elements[1])?;
    Ok(RealNumber::Float((LegacyToken::default(), part)))
}

fn lower_float_full_node(syntax: &SyntaxNode) -> LowerResult<RealNumber> {
    common::validate_node(syntax, SyntaxKind::FloatFull, "float-full")?;
    let elements = syntax.children_with_tokens();
    if elements.len() != 3 {
        return Err(String::from(
            "float-full syntax requires digits, period, and digits",
        ));
    }
    let whole = require_digit_sequence_element(&elements[0])?;
    require_element_token(&elements[1], SyntaxKind::Period, ".")?;
    let part = require_digit_sequence_element(&elements[2])?;
    Ok(RealNumber::Float((whole, part)))
}

fn lower_integer_literal_node(syntax: &SyntaxNode) -> LowerResult<RealNumber> {
    common::validate_node(syntax, SyntaxKind::IntegerLiteral, "integer-literal")?;
    let child = only_child(syntax, "integer-literal")?;
    match child.kind() {
        SyntaxKind::TypedInteger => lower_typed_integer_node(&child),
        SyntaxKind::UntypedInteger => lower_untyped_integer_node(&child),
        _ => Err(String::from(
            "integer-literal syntax requires a typed or untyped integer child",
        )),
    }
}

fn lower_typed_integer_node(syntax: &SyntaxNode) -> LowerResult<RealNumber> {
    common::validate_node(syntax, SyntaxKind::TypedInteger, "typed-integer")?;
    let elements = syntax.children_with_tokens();
    if elements.len() != 2 {
        return Err(String::from(
            "typed-integer syntax requires digits and an identifier suffix",
        ));
    }
    let digits = require_digit_sequence_element(&elements[0])?;
    let SyntaxElement::Node(identifier) = &elements[1] else {
        return Err(String::from(
            "typed-integer syntax requires an identifier suffix",
        ));
    };
    if identifier.kind() != SyntaxKind::Identifier {
        return Err(String::from(
            "typed-integer syntax requires an identifier suffix",
        ));
    }
    let suffix = lower_legacy_identifier(identifier)
        .map_err(|_| String::from("typed-integer syntax contains an invalid suffix"))?;
    Ok(RealNumber::TypedInteger((
        digits,
        KindAnnotation {
            kind: Kind::Scalar(suffix),
        },
    )))
}

fn lower_untyped_integer_node(syntax: &SyntaxNode) -> LowerResult<RealNumber> {
    common::validate_node(syntax, SyntaxKind::UntypedInteger, "untyped-integer")?;
    let digit = only_child(syntax, "untyped-integer")?;
    if digit.kind() != SyntaxKind::DigitSequence {
        return Err(String::from(
            "untyped-integer syntax requires one digit-sequence child",
        ));
    }
    Ok(RealNumber::Integer(lower_digit_sequence_token(&digit)?))
}

fn lower_digit_based_literal(
    syntax: &SyntaxNode,
    expected_kind: SyntaxKind,
    prefix: &str,
    construct: impl FnOnce(LegacyToken) -> RealNumber,
) -> LowerResult<RealNumber> {
    common::validate_node(syntax, expected_kind, "based-number")?;
    let elements = syntax.children_with_tokens();
    if elements.len() != 2 {
        return Err(String::from(
            "based-number syntax requires a prefix and digit sequence",
        ));
    }
    require_element_token(&elements[0], SyntaxKind::Text, prefix)?;
    let value = require_digit_sequence_element(&elements[1])?;
    Ok(construct(value))
}

fn lower_hexadecimal_literal_node(syntax: &SyntaxNode) -> LowerResult<RealNumber> {
    common::validate_node(
        syntax,
        SyntaxKind::HexadecimalLiteral,
        "hexadecimal-literal",
    )?;
    let elements = syntax.children_with_tokens();
    let Some((first, payload)) = elements.split_first() else {
        return Err(String::from(
            "hexadecimal-literal syntax requires a prefix and payload",
        ));
    };
    require_element_token(first, SyntaxKind::Text, "0x")?;
    if payload.is_empty() {
        return Err(String::from(
            "hexadecimal-literal syntax requires at least one payload token",
        ));
    }

    let mut tokens = Vec::with_capacity(payload.len());
    for element in payload {
        let SyntaxElement::Token(token) = element else {
            return Err(String::from(
                "hexadecimal-literal syntax payload cannot contain child nodes",
            ));
        };
        if !matches!(
            token.kind(),
            SyntaxKind::Digit | SyntaxKind::Underscore | SyntaxKind::Alpha
        ) {
            return Err(String::from(
                "hexadecimal-literal syntax contains an invalid payload token",
            ));
        }
        tokens.push(common::lower_syntax_token(syntax, token, TokenKind::Text)?);
    }
    let mut value = common::merge_legacy_tokens(&mut tokens, "hexadecimal payload")?;
    value.kind = TokenKind::Number;
    Ok(RealNumber::Hexadecimal(value))
}

fn require_integer_element(element: &SyntaxElement) -> LowerResult<RealNumber> {
    let SyntaxElement::Node(node) = element else {
        return Err(String::from(
            "rational-literal syntax requires integer-literal children",
        ));
    };
    if node.kind() != SyntaxKind::IntegerLiteral {
        return Err(String::from(
            "rational-literal syntax requires integer-literal children",
        ));
    }
    lower_integer_literal_node(node)
}

fn integer_numeric_token(value: RealNumber) -> LowerResult<LegacyToken> {
    match value {
        RealNumber::Integer(token) => Ok(token),
        RealNumber::TypedInteger((token, _)) => Ok(token),
        _ => Err(String::from(
            "integer-literal lowered to a non-integer compatibility value",
        )),
    }
}

fn require_digit_sequence_element(element: &SyntaxElement) -> LowerResult<LegacyToken> {
    let SyntaxElement::Node(node) = element else {
        return Err(String::from("expected a digit-sequence syntax node"));
    };
    if node.kind() != SyntaxKind::DigitSequence {
        return Err(String::from("expected a digit-sequence syntax node"));
    }
    lower_digit_sequence_token(node)
}

fn lower_digit_sequence_token(syntax: &SyntaxNode) -> LowerResult<LegacyToken> {
    let mut digits = lower_legacy_digit_sequence(syntax)
        .map_err(|_| String::from("digit-sequence syntax cannot be compatibility-lowered"))?;
    let mut token = common::merge_legacy_tokens(&mut digits, "digit sequence")?;
    token.kind = TokenKind::Number;
    Ok(token)
}

fn only_child(syntax: &SyntaxNode, name: &str) -> LowerResult<SyntaxNode> {
    let elements = syntax.children_with_tokens();
    if elements.len() != 1 {
        return Err(alloc::format!(
            "{name} syntax requires exactly one child node"
        ));
    }
    let SyntaxElement::Node(child) = &elements[0] else {
        return Err(alloc::format!(
            "{name} syntax requires exactly one child node"
        ));
    };
    Ok(child.clone())
}

fn require_element_token(element: &SyntaxElement, kind: SyntaxKind, text: &str) -> LowerResult<()> {
    let SyntaxElement::Token(token) = element else {
        return Err(String::from("expected a direct syntax token"));
    };
    require_token(token, kind, text)
}

fn require_token(token: &SyntaxToken, kind: SyntaxKind, text: &str) -> LowerResult<()> {
    common::validate_token(token)?;
    let actual = token
        .text()
        .map_err(|_| String::from("cannot read canonical token source"))?;
    if token.kind() != kind || actual != text {
        return Err(alloc::format!("expected {kind:?} token {text:?}"));
    }
    Ok(())
}

fn is_element_token(element: &SyntaxElement, kind: SyntaxKind) -> bool {
    matches!(element, SyntaxElement::Token(token) if token.kind() == kind)
}
