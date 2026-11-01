//! Direct execution of existing lexical productions for sealed ASCII input.
//! A following ASCII byte (or final EOF) proves the grapheme boundary. Unicode,
//! partial input, and resource frontiers retain the general continuation path.
use super::*;
use crate::document::TextSize;

#[derive(Clone, Copy)]
enum Class {
    Whitespace,
    SpaceTab,
    Digits,
    Identifier,
    One,
}
fn token(byte: u8) -> Option<SyntaxKind> {
    Some(match byte {
        b'a'..=b'z' | b'A'..=b'Z' => SyntaxKind::Alpha,
        b'0'..=b'9' => SyntaxKind::Digit,
        b' ' => SyntaxKind::Whitespace,
        b'\t' => SyntaxKind::Tab,
        b'\n' => SyntaxKind::Newline,
        b'\r' => SyntaxKind::CarriageReturn,
        b'&' => SyntaxKind::Ampersand,
        b'\'' => SyntaxKind::Apostrophe,
        b'*' => SyntaxKind::Asterisk,
        b'@' => SyntaxKind::At,
        b'|' => SyntaxKind::Bar,
        b'\\' => SyntaxKind::Backslash,
        b'^' => SyntaxKind::Caret,
        b':' => SyntaxKind::Colon,
        b',' => SyntaxKind::Comma,
        b'-' => SyntaxKind::Dash,
        b'$' => SyntaxKind::Dollar,
        b'=' => SyntaxKind::Equal,
        b'!' => SyntaxKind::Exclamation,
        b'`' => SyntaxKind::Grave,
        b'#' => SyntaxKind::HashTag,
        b'%' => SyntaxKind::Percent,
        b'.' => SyntaxKind::Period,
        b'+' => SyntaxKind::Plus,
        b'?' => SyntaxKind::Question,
        b'"' => SyntaxKind::Quote,
        b';' => SyntaxKind::Semicolon,
        b'/' => SyntaxKind::Slash,
        b'~' => SyntaxKind::Tilde,
        b'_' => SyntaxKind::Underscore,
        b'[' => SyntaxKind::LeftBracket,
        b'(' => SyntaxKind::LeftParen,
        b'{' => SyntaxKind::LeftBrace,
        b'<' => SyntaxKind::LeftAngle,
        b']' => SyntaxKind::RightBracket,
        b')' => SyntaxKind::RightParen,
        b'}' => SyntaxKind::RightBrace,
        b'>' => SyntaxKind::RightAngle,
        _ => return None,
    })
}
fn accepted(class: Class, byte: u8) -> bool {
    match class {
        Class::Whitespace => matches!(byte, b' ' | b'\t' | b'\r' | b'\n'),
        Class::SpaceTab => matches!(byte, b' ' | b'\t'),
        Class::Digits => byte.is_ascii_digit() || byte == b'_',
        Class::Identifier => {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'_' | b'&'
                        | b'$'
                        | b'%'
                        | b'/'
                        | b'#'
                        | b'\\'
                        | b'~'
                        | b'+'
                        | b'-'
                        | b'*'
                        | b'^'
                )
        }
        Class::One => false,
    }
}
pub(super) fn parse(
    parser: &mut Parser<'_>,
    rule: RuleId,
    fixed: Option<&FixedTerminalSpec>,
    allowance: &mut u64,
    work: &mut u64,
) -> bool {
    // Tiny work slices and resource frontiers continue through the resumable VM.
    if *allowance < 128 || parser.is_halted() {
        return false;
    }
    let mut reads = 0;
    let mut read = |offset: u32| {
        reads += 1;
        parser.cursor().context_byte_at(offset)
    };
    let plan = (|| {
        let first = read(0);
        let (class, min, node, single) = match rule {
            rules::WHITESPACE0 => (Class::Whitespace, 0, None, None),
            rules::WHITESPACE1 => (Class::Whitespace, 1, None, None),
            rules::WS0E | rules::SPACE_TAB0 => (Class::SpaceTab, 0, None, None),
            rules::WS1E | rules::SPACE_TAB1 => (Class::SpaceTab, 1, None, None),
            rules::DIGIT_SEQUENCE if first.is_some_and(|b| b.is_ascii_digit()) => {
                (Class::Digits, 1, Some(SyntaxKind::DigitSequence), None)
            }
            rules::IDENTIFIER if first.is_some_and(|b| b.is_ascii_alphabetic()) => {
                (Class::Identifier, 1, Some(SyntaxKind::Identifier), None)
            }
            rules::ALPHA | rules::ALPHA_TOKEN if first.is_some_and(|b| b.is_ascii_alphabetic()) => {
                (Class::One, 1, None, Some((SyntaxKind::Alpha, 1)))
            }
            rules::DIGIT | rules::DIGIT_TOKEN if first.is_some_and(|b| b.is_ascii_digit()) => {
                (Class::One, 1, None, Some((SyntaxKind::Digit, 1)))
            }
            rules::ANY | rules::ANY_TOKEN if first.is_some_and(|b| b.is_ascii() && b != b'\r') => {
                (Class::One, 1, None, Some((SyntaxKind::Any, 1)))
            }
            rules::TEXT | rules::RAW_TEXT
                if first.is_some_and(|b| b.is_ascii() && !matches!(b, b'\\' | b'\r' | b'\n')) =>
            {
                let Some(kind) = first.and_then(token) else {
                    return None;
                };
                (Class::One, 1, None, Some((kind, 1)))
            }
            _ => {
                let Some(spec) = fixed else {
                    return None;
                };
                if spec.spacing != TerminalSpacing::Exact
                    || !spec.literal.is_ascii()
                    || spec.literal.contains('\r')
                {
                    return None;
                }
                if spec
                    .literal
                    .as_bytes()
                    .iter()
                    .enumerate()
                    .any(|(i, b)| read(i as u32) != Some(*b))
                {
                    return None;
                }
                (
                    Class::One,
                    1,
                    None,
                    Some((spec.kind, spec.literal.len() as u32)),
                )
            }
        };
        let (end, count) = if let Some((_, width)) = single {
            let next = read(width);
            if next.is_some_and(|b| !b.is_ascii())
                || parser.offset() + TextSize(width) > parser.cursor().end()
            {
                return None;
            }
            (width, 1)
        } else {
            let mut at = 0_u32;
            let mut count = 0;
            loop {
                let b = if at == 0 { first } else { read(at) };
                let Some(b) = b else {
                    break;
                };
                if !b.is_ascii() {
                    return None;
                }
                if parser.offset() + TextSize(at) >= parser.cursor().end() || !accepted(class, b) {
                    break;
                }
                if at as u64 >= (*allowance / 4).min(parser.state.fuel.saturating_sub(128)) {
                    return None;
                }
                if matches!(class, Class::Digits) && b == b'_' {
                    if !read(at + 1).is_some_and(|b| b.is_ascii_digit()) {
                        return None;
                    }
                }
                at += 1;
                count += 1;
                if b == b'\r' && read(at) == Some(b'\n') {
                    if parser.offset() + TextSize(at) >= parser.cursor().end() {
                        return None;
                    }
                    at += 1;
                }
            }
            if count < min {
                return None;
            }
            (at, count)
        };
        Some((node, single, end, count))
    })();
    drop(read);
    *allowance -= reads;
    *work += reads;
    let Some((node, single, end, count)) = plan else {
        return false;
    };
    let emission_work = count as u64 + u64::from(node.is_some()) * 2;
    if *allowance < emission_work {
        return false;
    }
    if parser.state.fuel <= count as u64
        || parser.state.events.len()
            + count
            + usize::from(node.is_some()) * 3
            + parser.state.open_markers.len()
            + 5
            >= parser.state.config.limits.max_events as usize
    {
        return false;
    }
    *allowance -= emission_work;
    *work += emission_work;
    let marker = node.map(|_| parser.start());
    if let Some((kind, width)) = single {
        parser.bump_bytes_token(width, kind).unwrap();
    } else {
        let end = parser.offset() + TextSize(end);
        while parser.offset() < end {
            let b = parser.cursor().byte().unwrap();
            let width = if b == b'\r' && parser.cursor().byte_at(1) == Some(b'\n') {
                2
            } else {
                1
            };
            parser.bump_bytes_token(width, token(b).unwrap()).unwrap();
        }
    }
    if let (Some(marker), Some(kind)) = (marker, node) {
        marker.complete(parser, kind);
    }
    true
}
