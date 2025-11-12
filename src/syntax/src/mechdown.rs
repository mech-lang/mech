#[macro_use]
use crate::*;

#[cfg(not(feature = "no-std"))] use core::fmt;
#[cfg(feature = "no-std")] use alloc::fmt;
#[cfg(feature = "no-std")] use alloc::string::String;
#[cfg(feature = "no-std")] use alloc::vec::Vec;
use nom::{
  IResult,
  branch::alt,
  sequence::tuple as nom_tuple,
  combinator::{opt, eof, peek},
  multi::{many1, many_till, many0, separated_list1,separated_list0},
  bytes::complete::{take_until, take_while},
  Err,
  Err::Failure
};

use std::collections::HashMap;
use colored::*;

use crate::*;

// Mechdown
// ============================================================================

// title := text+, new_line, equal+, (space|tab)*, whitespace* ;
pub fn title(input: ParseString) -> ParseResult<Title> {
  let (input, mut text) = many1(text)(input)?;
  let (input, _) = new_line(input)?;
  let (input, _) = many1(equal)(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = new_line(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = whitespace0(input)?;
  let mut title = Token::merge_tokens(&mut text).unwrap();
  title.kind = TokenKind::Title;
  Ok((input, Title{text: title}))
}

pub struct MarkdownTableHeader {
  pub header: Vec<(Token, Token)>,
}

pub fn no_alignment(input: ParseString) -> ParseResult<ColumnAlignment> {
  let (input, _) = many1(dash)(input)?;
  Ok((input, ColumnAlignment::Left))
}

pub fn left_alignment(input: ParseString) -> ParseResult<ColumnAlignment> {
  let (input, _) = colon(input)?;
  let (input, _) = many1(dash)(input)?;
  Ok((input, ColumnAlignment::Left))
}

pub fn right_alignment(input: ParseString) -> ParseResult<ColumnAlignment> {
  let (input, _) = many1(dash)(input)?;
  let (input, _) = colon(input)?;
  Ok((input, ColumnAlignment::Right))
}

pub fn center_alignment(input: ParseString) -> ParseResult<ColumnAlignment> {
  let (input, _) = colon(input)?;
  let (input, _) = many1(dash)(input)?;
  let (input, _) = colon(input)?;
  Ok((input, ColumnAlignment::Center))
}

pub fn alignment_separator(input: ParseString) -> ParseResult<ColumnAlignment> {
  let (input, _) = many0(space_tab)(input)?;
  let (input, separator) = alt((center_alignment, left_alignment, right_alignment, no_alignment))(input)?;
  let (input, _) = many0(space_tab)(input)?;
  Ok((input, separator))
}

pub fn mechdown_table(input: ParseString) -> ParseResult<MarkdownTable> {
  let (input, _) = whitespace0(input)?;
  let (input, table) = alt((mechdown_table_with_header, mechdown_table_no_header))(input)?;
  Ok((input, table))
}

pub fn mechdown_table_with_header(input: ParseString) -> ParseResult<MarkdownTable> {
  let (input, (header,alignment)) = mechdown_table_header(input)?;
  let (input, rows) = many1(mechdown_table_row)(input)?;
  Ok((input, MarkdownTable{header, rows, alignment}))
}

pub fn mechdown_table_no_header(input: ParseString) -> ParseResult<MarkdownTable> {
  let (input, rows) = many1(mechdown_table_row)(input)?;
  let header = vec![];
  let alignment = vec![];
  Ok((input, MarkdownTable{header, rows, alignment}))
}

pub fn mechdown_table_header(input: ParseString) -> ParseResult<(Vec<Paragraph>,Vec<ColumnAlignment>)> {
  let (input, _) = whitespace0(input)?;
  let (input, header) = many1(tuple((bar, tuple((many0(space_tab), paragraph)))))(input)?;
  let (input, _) = bar(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, alignment) = many1(tuple((bar, tuple((many0(space_tab), alignment_separator)))))(input)?;
  let (input, _) = bar(input)?;
  let (input, _) = whitespace0(input)?;
  let column_names: Vec<Paragraph> = header.into_iter().map(|(_,(_,tkn))| tkn).collect();
  let column_alignments = alignment.into_iter().map(|(_,(_,tkn))| tkn).collect();
  Ok((input, (column_names,column_alignments)))
}

pub fn empty_paragraph(input: ParseString) -> ParseResult<Paragraph> {
  Ok((input, Paragraph{elements: vec![]}))
}

// mechdown_table_row := +(bar, paragraph), bar, *whitespace ;
pub fn mechdown_table_row(input: ParseString) -> ParseResult<Vec<Paragraph>> {
  let (input, _) = whitespace0(input)?;
  let (input, _) = bar(input)?;
  let (input, row) = many1(tuple((alt((tuple((many0(space_tab), paragraph)),tuple((many1(space_tab), empty_paragraph)))),bar)))(input)?;
  let (input, _) = whitespace0(input)?;
  let row = row.into_iter().map(|((_,tkn),_)| tkn).collect();
  Ok((input, row))
}

// subtitle := digit_token+, period, space*, text+, new_line, dash+, (space|tab)*, new_line, (space|tab)*, whitespace* ;
pub fn ul_subtitle(input: ParseString) -> ParseResult<Subtitle> {
  let (input, _) = many1(digit_token)(input)?;
  let (input, _) = period(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, text) = paragraph(input)?;
  let (input, _) = new_line(input)?;
  let (input, _) = many1(dash)(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = new_line(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, Subtitle{text, level: 2}))
}

// alpha_subtitle := (space|tab)*, "(", alpha, ")", (space|tab)+, text+, (space|tab)*, whitespace* ;
pub fn subtitle(input: ParseString) -> ParseResult<Subtitle> {
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = left_parenthesis(input)?;
  let (input, num) = separated_list1(period,alt((many1(alpha),many1(digit))))(input)?;
  let (input, _) = right_parenthesis(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, text) = paragraph(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = whitespace0(input)?;
  let level: u8 = if num.len() < 3 { 3 } else { num.len() as u8 + 1 };
  Ok((input, Subtitle{text, level}))
}

// strong := (asterisk, asterisk), +paragraph-element, (asterisk, asterisk) ;
pub fn strong(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, _) = tuple((asterisk,asterisk))(input)?;
  let (input, text) = paragraph_element(input)?;
  let (input, _) = tuple((asterisk,asterisk))(input)?;
  Ok((input, ParagraphElement::Strong(Box::new(text))))
}

/// emphasis := asterisk, +paragraph-element, asterisk ;
pub fn emphasis(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, _) = asterisk(input)?;
  let (input, text) = paragraph_element(input)?;
  let (input, _) = asterisk(input)?;
  Ok((input, ParagraphElement::Emphasis(Box::new(text))))
}

// strikethrough := tilde, +paragraph-element, tilde ;
pub fn strikethrough(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, _) = tilde(input)?;
  let (input, text) = paragraph_element(input)?;
  let (input, _) = tilde(input)?;
  Ok((input, ParagraphElement::Strikethrough(Box::new(text))))
}

/// underline := underscore, +paragraph-element, underscore ;
pub fn underline(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, _) = underscore(input)?;
  let (input, text) = paragraph_element(input)?;
  let (input, _) = underscore(input)?;
  Ok((input, ParagraphElement::Underline(Box::new(text))))
}

/// highlight := "!!", +paragraph-element, "!!" ;
pub fn highlight(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, _) = highlight_sigil(input)?;
  let (input, text) = paragraph_element(input)?;
  let (input, _) = highlight_sigil(input)?;
  Ok((input, ParagraphElement::Highlight(Box::new(text))))
}

// inline-code := grave, +text, grave ; 
pub fn inline_code(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, _) = grave(input)?;
  let (input, text) = many0(tuple((is_not(grave),text)))(input)?;
  let (input, _) = grave(input)?;
  let mut text = text.into_iter().map(|(_,tkn)| tkn).collect();
  // return empty token if there's nothing between the graves
  let mut text = match Token::merge_tokens(&mut text) {
    Some(t) => t,
    None => {
      return Ok((input, ParagraphElement::InlineCode(Token::default())));
    }
  };
  text.kind = TokenKind::Text;
  Ok((input, ParagraphElement::InlineCode(text)))
}

// inline-equation := equation-sigil, +text, equation-sigil ;
pub fn inline_equation(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, _) = equation_sigil(input)?;
  let (input, txt) = many0(tuple((is_not(equation_sigil),alt((backslash,text)))))(input)?;
  let (input, _) = equation_sigil(input)?;
  let mut txt = txt.into_iter().map(|(_,tkn)| tkn).collect();
  let mut eqn = Token::merge_tokens(&mut txt).unwrap();
  eqn.kind = TokenKind::Text;
  Ok((input, ParagraphElement::InlineEquation(eqn)))
}

// hyperlink := "[", +text, "]", "(", +text, ")" ;
pub fn hyperlink(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, _) = left_bracket(input)?;
  let (input, link_text) = many1(tuple((is_not(right_bracket),text)))(input)?;
  let (input, _) = right_bracket(input)?;
  let (input, _) = left_parenthesis(input)?;
  let (input, link) = many1(tuple((is_not(right_parenthesis),text)))(input)?;
  let (input, _) = right_parenthesis(input)?;
  let mut tokens = link.into_iter().map(|(_,tkn)| tkn).collect::<Vec<Token>>();
  let link_merged = Token::merge_tokens(&mut tokens).unwrap();
  let mut tokens = link_text.into_iter().map(|(_,tkn)| tkn).collect::<Vec<Token>>();
  let text_merged = Token::merge_tokens(&mut tokens).unwrap();
  Ok((input, ParagraphElement::Hyperlink((text_merged, link_merged))))
}

// raw-hyperlink := http-prefix, +text ;
pub fn raw_hyperlink(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, _) = peek(http_prefix)(input)?;
  let (input, address) = many1(tuple((is_not(space), text)))(input)?;
  let mut tokens = address.into_iter().map(|(_,tkn)| tkn).collect::<Vec<Token>>();
  let url = Token::merge_tokens(&mut tokens).unwrap();
  Ok((input, ParagraphElement::Hyperlink((url.clone(), url))))
}

// option-map := "{", whitespace*, mapping*, whitespace*, "}" ;
pub fn option_map(input: ParseString) -> ParseResult<OptionMap> {
  let msg = "Expects right bracket '}' to terminate inline table";
  let (input, (_, r)) = range(left_brace)(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, elements) = many1(option_mapping)(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = label!(right_brace, msg, r)(input)?;
  Ok((input, OptionMap{elements}))
}

// option-mapping :=  whitespace*, expression, whitespace*, ":", whitespace*, expression, comma?, whitespace* ;
pub fn option_mapping(input: ParseString) -> ParseResult<(Identifier, MechString)> {
  let msg1 = "Unexpected space before colon ':'";
  let msg2 = "Expects a value";
  let msg3 = "Expects whitespace or comma followed by whitespace";
  let msg4 = "Expects whitespace";
  let (input, _) = whitespace0(input)?;
  let (input, key) = identifier(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = colon(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, value) = string(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = opt(comma)(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, (key, value)))
}

// img := "![", paragraph, "]", "(", +text, ")" , ?option-map ;
pub fn img(input: ParseString) -> ParseResult<Image> {
  let (input, _) = img_prefix(input)?;
  let (input, caption_text) = paragraph(input)?;
  let (input, _) = right_bracket(input)?;
  let (input, _) = left_parenthesis(input)?;
  let (input, src) = many1(tuple((is_not(right_parenthesis),text)))(input)?;
  let (input, _) = right_parenthesis(input)?;
  let (input, style) = opt(option_map)(input)?;
  let merged_src = Token::merge_tokens(&mut src.into_iter().map(|(_,tkn)| tkn).collect::<Vec<Token>>()).unwrap();
  Ok((input, Image{src: merged_src, caption: Some(caption_text), style}))
}

// paragraph-text := ¬(img-prefix | http-prefix | left-bracket | tilde | asterisk | underscore | grave | define-operator | bar), +text ;
pub fn paragraph_text(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, elements) = match many1(nom_tuple((is_not(alt((section_sigil, footnote_prefix, highlight_sigil, equation_sigil, img_prefix, http_prefix, left_brace, left_bracket, left_angle, right_bracket, tilde, asterisk, underscore, grave, define_operator, bar))),text)))(input) {
    Ok((input, mut text)) => {
      let mut text = text.into_iter().map(|(_,tkn)| tkn).collect();
      let mut text = Token::merge_tokens(&mut text).unwrap();
      text.kind = TokenKind::Text;
      (input, ParagraphElement::Text(text))
    }, 
    Err(err) => {return Err(err);},
  };
  Ok((input, elements))
}

// eval-inline-mech-cdoe := "{", ws0, expression, ws0, "}" ;`
pub fn eval_inline_mech_code(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, _) = left_brace(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, expr) = expression(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = right_brace(input)?;
  Ok((input, ParagraphElement::EvalInlineMechCode(expr)))
}

// inline-mech-cdoe := "{{", ws0, expression, ws0, "}}" ;`
pub fn inline_mech_code(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, _) = left_brace(input)?;
  let (input, _) = left_brace(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, expr) = mech_code_alt(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = right_brace(input)?;
  let (input, _) = right_brace(input)?;
  Ok((input, ParagraphElement::InlineMechCode(expr)))
}

// footnote-reference := "[^", +text, "]" ;
pub fn footnote_reference(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, _) = footnote_prefix(input)?;
  let (input, text) = many1(tuple((is_not(right_bracket),text)))(input)?;
  let (input, _) = right_bracket(input)?;
  let mut tokens = text.into_iter().map(|(_,tkn)| tkn).collect::<Vec<Token>>();
  let footnote_text = Token::merge_tokens(&mut tokens).unwrap();
  Ok((input, ParagraphElement::FootnoteReference(footnote_text)))
}

// reference := "[", +alphanumeric, "]" ;
pub fn reference(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, _) = left_bracket(input)?;
  let (input, mut txt) = many1(alphanumeric)(input)?;
  let (input, _) = right_bracket(input)?;
  let ref_text = Token::merge_tokens(&mut txt).unwrap();
  Ok((input, ParagraphElement::Reference(ref_text)))
}

// section_ref := "§" , +(alphanumeric | period) ;
pub fn section_reference(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, _) = section_sigil(input)?;
  let (input, mut txt) = many1(alt((alphanumeric, period)))(input)?;
  let section_text = Token::merge_tokens(&mut txt).unwrap();
  Ok((input, ParagraphElement::SectionReference(section_text)))
}

// paragraph-element := hyperlink | reference | section-ref | raw-hyperlink | highlight | footnote-reference | inline-mech-code | eval-inline-mech-code | inline-equation | paragraph-text | strong | highlight | emphasis | inline-code | strikethrough | underline ;
pub fn paragraph_element(input: ParseString) -> ParseResult<ParagraphElement> {
  alt((hyperlink, reference, section_reference, raw_hyperlink, highlight, footnote_reference, inline_mech_code, eval_inline_mech_code, inline_equation, paragraph_text, strong, highlight, emphasis, inline_code, strikethrough, underline))(input)
}

// paragraph := +paragraph_element ;
pub fn paragraph(input: ParseString) -> ParseResult<Paragraph> {
  let (input, elements) = many1(paragraph_element)(input)?;
  Ok((input, Paragraph{elements}))
}

// indented-ordered-list-item := ws, number, ".", +text, new_line*; 
pub fn ordered_list_item(input: ParseString) -> ParseResult<(Number,Paragraph)> {
  let (input, number) = number(input)?;
  let (input, _) = period(input)?;
  let (input, list_item) = paragraph(input)?;
  let (input, _) = new_line(input)?;
  Ok((input, (number,list_item)))
}

// checked-item := "-", ("[", "x", "]"), paragraph ;
pub fn checked_item(input: ParseString) -> ParseResult<(bool,Paragraph)> {
  let (input, _) = dash(input)?;
  let (input, _) = left_bracket(input)?;
  let (input, _) = alt((tag("x"),tag("✓"),tag("✗")))(input)?;
  let (input, _) = right_bracket(input)?;
  let (input, list_item) = paragraph(input)?;
  let (input, _) = new_line(input)?;
  Ok((input, (true,list_item)))
}

// unchecked-item := "-", ("[", whitespace0, "]"), paragraph ;
pub fn unchecked_item(input: ParseString) -> ParseResult<(bool,Paragraph)> {
  let (input, _) = dash(input)?;
  let (input, _) = left_bracket(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, _) = right_bracket(input)?;
  let (input, list_item) = paragraph(input)?;
  let (input, _) = new_line(input)?;
  Ok((input, (false,list_item)))
}

// check-list-item := checked-item | unchecked-item ;
pub fn check_list_item(input: ParseString) -> ParseResult<(bool,Paragraph)> {
  let (input, item) = alt((checked_item, unchecked_item))(input)?;
  Ok((input, item))
}

pub fn check_list(mut input: ParseString, level: usize) -> ParseResult<MDList> {
  let mut items = vec![];
  loop {
    // Calculate current line indent
    let mut indent = 0;
    let mut current = input.peek(indent);
    while current == Some(" ") || current == Some("\t") {
      indent += 1;
      current = input.peek(indent);
    }
    // If indent is less than current level, we are done parsing this list level
    if indent < level {
      break;
    }
    // Consume whitespace
    let (next_input, _) = many0(space_tab)(input.clone())?;
    // Try parsing a checklist item
    let (next_input, list_item) = match check_list_item(next_input.clone()) {
      Ok((next_input, list_item)) => (next_input, list_item),
      Err(err) => {
        if !items.is_empty() {
          break;
        } else {
          return Err(err);
        }
      }
    };
    // Look ahead to next line's indent
    let mut lookahead_indent = 0;
    let mut current = next_input.peek(lookahead_indent);
    while current == Some(" ") || current == Some("\t") {
      lookahead_indent += 1;
      current = next_input.peek(lookahead_indent);
    }
    input = next_input;
    if lookahead_indent < level {
      // End of this list level
      items.push((list_item, None));
      break;
    } else if lookahead_indent == level {
      // Same level, continue
      items.push((list_item, None));
      continue;
    } else {
      // Nested sublist: parse recursively
      let (next_input, sublist_md) = sublist(input.clone(), lookahead_indent)?;
      items.push((list_item, Some(sublist_md)));
      input = next_input;
    }
  }
  Ok((input, MDList::Check(items)))
}


// unordered_list := +list_item, ?new_line, *whitespace ;
pub fn unordered_list(mut input: ParseString, level: usize) -> ParseResult<MDList> {
  let mut items = vec![];
  loop {
    let mut indent = 0;
    let mut current = input.peek(indent);
    while current == Some(" ") || current == Some("\t") {
      indent += 1;
      current = input.peek(indent);
    }
    // If indentation is less than the current level, return to parent list
    if indent < level {
      return Ok((input, MDList::Unordered(items)));
    }
    let (next_input, _) = many0(space_tab)(input.clone())?;
    // Try to parse a list item
    let (next_input, list_item) = match unordered_list_item(next_input.clone()) {
      Ok((next_input, list_item)) => (next_input, list_item),
      Err(err) => {
        if !items.is_empty() {
          return Ok((input, MDList::Unordered(items)));
        } else {
          return Err(err);
        }
      }
    };
    // Look ahead at the next line to determine indent
    let mut lookahead_indent = 0;
    let mut current = next_input.peek(lookahead_indent);
    while current == Some(" ") || current == Some("\t") {
      lookahead_indent += 1;
      current = next_input.peek(lookahead_indent);
    }
    input = next_input;
    if lookahead_indent < level {
      // This is the last item at the current list level
      items.push((list_item, None));
      return Ok((input, MDList::Unordered(items)));
    } else if lookahead_indent == level {
      // Continue at the same level
      items.push((list_item, None));
      continue;
    } else {
      // Nested list detected
      let (next_input, sub) = sublist(input.clone(), lookahead_indent)?;
      items.push((list_item, Some(sub)));
      input = next_input;
    }
  }
}

// ordered-list := +ordered-list-item, ?new-line, *whitespace ;
pub fn ordered_list(mut input: ParseString, level: usize) -> ParseResult<MDList> {
  let mut items = vec![];
  loop {
    let mut indent = 0;
    let mut current = input.peek(indent);
    while current == Some(" ") || current == Some("\t") {
      indent += 1;
      current = input.peek(indent);
    }
    // If indent drops below current level, return to parent
    if indent < level {
      let start = items.first()
        .map(|item: &((Number, Paragraph), Option<MDList>)| item.0.0.clone())
        .unwrap_or(Number::from_integer(1));
      return Ok((input, MDList::Ordered(OrderedList { start, items })));
    }
    // Consume whitespace
    let (next_input, _) = many0(space_tab)(input.clone())?;
    // Try to parse an ordered list item
    let (next_input, (list_item, _)) = match tuple((ordered_list_item, is_not(tuple((dash, dash)))))(next_input.clone()) {
      Ok((next_input, res)) => (next_input, res),
      Err(err) => {
        if !items.is_empty() {
          let start = items.first()
            .map(|((number, _), _)| number.clone())
            .unwrap_or(Number::from_integer(1));
          return Ok((input, MDList::Ordered(OrderedList { start, items })));
        } else {
          return Err(err);
        }
      }
    };

    // Determine indentation of the next line
    let mut lookahead_indent = 0;
    let mut current = next_input.peek(lookahead_indent);
    while current == Some(" ") || current == Some("\t") {
      lookahead_indent += 1;
      current = next_input.peek(lookahead_indent);
    }

    input = next_input;

    if lookahead_indent < level {
      items.push((list_item, None));
      let start = items.first()
        .map(|((number, _), _)| number.clone())
        .unwrap_or(Number::from_integer(1));
      return Ok((input, MDList::Ordered(OrderedList { start, items })));
    } else if lookahead_indent == level {
      items.push((list_item, None));
      continue;
    } else {
      // Nested sublist
      let (next_input, sub) = sublist(input.clone(), lookahead_indent)?;
      items.push((list_item, Some(sub)));
      input = next_input;
    }
  }
}



pub fn sublist(input: ParseString, level: usize) -> ParseResult<MDList> {
  let (input, list) = match ordered_list(input.clone(), level) {
    Ok((input, list)) => (input, list),
    _ => match check_list(input.clone(), level) {
      Ok((input, list)) => (input, list),
      _ => match unordered_list(input.clone(), level) {
        Ok((input, list)) => (input, list),
        Err(err) => { return Err(err); }
      }
    }
  };
  Ok((input, list))
}

// mechdown-list := ordered-list | unordered-list ;
pub fn mechdown_list(input: ParseString) -> ParseResult<MDList> {
  let (input, list) = match ordered_list(input.clone(), 0) {
    Ok((input, list)) => (input, list),
    _ => match check_list(input.clone(), 0) {
      Ok((input, list)) => (input, list),
      _ => match unordered_list(input.clone(), 0) {
        Ok((input, list)) => (input, list),
        Err(err) => { return Err(err); }
      }
    }
  };
  Ok((input, list))
}

// list_item := dash, <space+>, <paragraph>, new_line* ;
pub fn unordered_list_item(input: ParseString) -> ParseResult<(Option<Token>,Paragraph)> {
  let msg1 = "Expects space after dash";
  let msg2 = "Expects paragraph as list item";
  let (input, _) = dash(input)?;
  let (input, bullet) = opt(tuple((left_parenthesis, emoji, right_parenthesis)))(input)?;
  let (input, _) = labelr!(null(many1(space)), skip_nil, msg1)(input)?;
  let (input, list_item) = label!(paragraph, msg2)(input)?;
  let (input, _) = many0(new_line)(input)?;
  let bullet = match bullet {
    Some((_,b,_)) => Some(b),
    None => None,
  };
  Ok((input,  (bullet, list_item)))
}


pub fn skip_till_eol(input: ParseString) -> ParseResult<()> {

  Ok((input, ()))
}

// codeblock-sigil := "```" | "~~~" ;
pub fn codeblock_sigil(input: ParseString) -> ParseResult<fn(ParseString) -> ParseResult<Token>> {
  let (input, sgl_tkn) = alt((grave_codeblock_sigil, tilde_codeblock_sigil))(input)?;
  let sgl_cmb = match sgl_tkn.kind {
    TokenKind::GraveCodeBlockSigil => grave_codeblock_sigil,
    TokenKind::TildeCodeBlockSigil => tilde_codeblock_sigil,
    _ => unreachable!(),
  };
  Ok((input, sgl_cmb))
}

//
pub fn code_block(input: ParseString) -> ParseResult<SectionElement> {
  let msg1 = "Expects 3 graves to start a code block";
  let msg2 = "Expects new_line";
  let msg3 = "Expects 3 graves followed by new_line to terminate a code block";
  let (input, (end_sgl,r)) = range(codeblock_sigil)(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, code_id) = many0(tuple((is_not(left_brace),text)))(input)?;
  let code_id = code_id.into_iter().map(|(_,tkn)| tkn).collect::<Vec<Token>>();
  let (input, options) = opt(option_map)(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = label!(new_line, msg2)(input)?;
  let (input, (text,src_range)) = range(many0(nom_tuple((
    is_not(end_sgl),
    any,
  ))))(input)?;
  let (input, _) = end_sgl(input)?;
  let (input, _) = whitespace0(input)?;
  let block_src: Vec<char> = text.into_iter().flat_map(|(_, s)| s.chars().collect::<Vec<char>>()).collect();
  let code_token = Token::new(TokenKind::CodeBlock, src_range, block_src.clone());

  let code_id = code_id.iter().flat_map(|tkn| tkn.chars.clone().into_iter().collect::<Vec<char>>()).collect::<String>();
  match code_id.as_str() {
    "ebnf" => {
      let ebnf_text = block_src.iter().collect::<String>();
      match parse_grammar(&ebnf_text) {
        Ok(grammar_tree) => {return Ok((input, SectionElement::Grammar(grammar_tree)));},
        Err(err) => {
          println!("Error parsing EBNF grammar: {:?}", err);
          todo!();
        }
      }
    }
    tag => {
      // if x begins with mec, mech, or 🤖
      if tag.starts_with("mech") || tag.starts_with("mec") || tag.starts_with("🤖") {

        // get rid of the prefix and then treat the rest of the string after : as an identifier
        let rest = tag.trim_start_matches("mech").trim_start_matches("mec").trim_start_matches("🤖").trim_start_matches(":");
        
        let config = if rest == "" {BlockConfig { namespace: 0, disabled: false, hidden: false}}
        else if rest == "disabled" { BlockConfig { namespace: hash_str(rest), disabled: true, hidden: false} }
        else if rest == "hidden" { BlockConfig { namespace: hash_str(rest), disabled: false, hidden: true} }
        else { BlockConfig { namespace: hash_str(rest), disabled: false, hidden: false} };

        let mech_src = block_src.iter().collect::<String>();
        let graphemes = graphemes::init_source(&mech_src);
        let parse_string = ParseString::new(&graphemes);

        match many1(mech_code)(parse_string) {
          Ok((_, mech_tree)) => {
            // TODO what if not all the input is parsed? Is that handled?
            return Ok((input, SectionElement::FencedMechCode(FencedMechCode{code: mech_tree, config, options})));
          },
          Err(err) => {
            println!("Error parsing Mech code: {:?}", err);
            todo!();
          }
        };
      } else if tag.starts_with("equation") || tag.starts_with("eq") || tag.starts_with("math") || tag.starts_with("latex") || tag.starts_with("tex") {
          return Ok((input, SectionElement::Equation(code_token)));
      } else if tag.starts_with("diagram") || tag.starts_with("chart") || tag.starts_with("mermaid") {
          return Ok((input, SectionElement::Diagram(code_token)));          
      } else {
        // Some other code block, just keep moving although we might want to do something with it later
      }
    }
  } 
  Ok((input, SectionElement::CodeBlock(code_token)))
}

pub fn thematic_break(input: ParseString) -> ParseResult<SectionElement> {
  let (input, _) = many1(asterisk)(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = new_line(input)?;
  Ok((input, SectionElement::ThematicBreak))
}

// footnote := "[^", +text, "]", ":", ws0, paragraph ;
pub fn footnote(input: ParseString) -> ParseResult<Footnote> {
  let (input, _) = footnote_prefix(input)?;
  let (input, text) = many1(tuple((is_not(right_bracket),text)))(input)?;
  let (input, _) = right_bracket(input)?;
  let (input, _) = colon(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, paragraph) = paragraph(input)?;
  let mut tokens = text.into_iter().map(|(_,tkn)| tkn).collect::<Vec<Token>>();
  let footnote_text = Token::merge_tokens(&mut tokens).unwrap();
  let footnote = (footnote_text, paragraph);
  Ok((input, footnote))
}

pub fn blank_line(input: ParseString) -> ParseResult<Vec<Token>> {
  let (input, mut st) = many0(space_tab)(input)?;
  let (input, n) = new_line(input)?;
  st.push(n);
  Ok((input, st))
}

pub fn question_block(input: ParseString) -> ParseResult<SectionElement> {
    let (input, _) = question_sigil(input)?;
    let (input, _) = many0(space_tab)(input)?;
    let (input, first_para) = paragraph(input)?;

    // Parse *(newline, *space, paragraph)
    let (input, mut rest_paras) = many0(|input| {
        let (input, _) = new_line(input)?;
        let (input, _) = many0(space_tab)(input)?;
        let (input, para) = paragraph(input)?;
        Ok((input, para))
    })(input)?;

    let mut all_paragraphs = vec![first_para];
    all_paragraphs.append(&mut rest_paras);

    Ok((input, SectionElement::QuestionBlock(all_paragraphs)))
}

pub fn info_block(input: ParseString) -> ParseResult<SectionElement> {
    let (input, _) = info_sigil(input)?;
    let (input, _) = many0(space_tab)(input)?;
    let (input, first_para) = paragraph(input)?;

    // Parse *(newline, *space, paragraph)
    let (input, mut rest_paras) = many0(|input| {
        let (input, _) = new_line(input)?;
        let (input, _) = many0(space_tab)(input)?;
        let (input, para) = paragraph(input)?;
        Ok((input, para))
    })(input)?;

    let mut all_paragraphs = vec![first_para];
    all_paragraphs.append(&mut rest_paras);

    Ok((input, SectionElement::InfoBlock(all_paragraphs)))
}

pub fn quote_block(input: ParseString) -> ParseResult<SectionElement> {
    let (input, _) = quote_sigil(input)?;
    let (input, _) = many0(space_tab)(input)?;
    let (input, first_para) = paragraph(input)?;

    // Parse *(newline, *space, paragraph)
    let (input, mut rest_paras) = many0(|input| {
        let (input, _) = new_line(input)?;
        let (input, _) = many0(space_tab)(input)?;
        let (input, para) = paragraph(input)?;
        Ok((input, para))
    })(input)?;

    let mut all_paragraphs = vec![first_para];
    all_paragraphs.append(&mut rest_paras);

    Ok((input, SectionElement::QuoteBlock(all_paragraphs)))
}

// abstract-element := abstract-sigil, *space, paragraph, *(new_line, *space, paragraph)));
pub fn abstract_el(input: ParseString) -> ParseResult<SectionElement> {
    let (input, _) = abstract_sigil(input)?;
    let (input, _) = many0(space_tab)(input)?;
    let (input, first_para) = paragraph(input)?;

    // Parse *(newline, *space, paragraph)
    let (input, mut rest_paras) = many0(|input| {
        let (input, _) = new_line(input)?;
        let (input, _) = many0(space_tab)(input)?;
        let (input, para) = paragraph(input)?;
        Ok((input, para))
    })(input)?;

    let mut all_paragraphs = vec![first_para];
    all_paragraphs.append(&mut rest_paras);

    Ok((input, SectionElement::Abstract(all_paragraphs)))
}

// equation := "$$" , +text ;
pub fn equation(input: ParseString) -> ParseResult<Token> {
  let (input, _) = equation_sigil(input)?;
  let (input, mut txt) = many1(alt((backslash,text)))(input)?;
  let mut eqn = Token::merge_tokens(&mut txt).unwrap();
  Ok((input, eqn))
}

// citation := "[", (identifier | number), "]", ":", ws0, paragraph, ws0, ?("(", +text, ")") ;
pub fn citation(input: ParseString) -> ParseResult<Citation> {
  let (input, _) = left_bracket(input)?;
  let (input, mut id) = many1(alphanumeric)(input)?;
  let (input, _) = right_bracket(input)?;
  let (input, _) = colon(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, txt) = paragraph(input)?;
  let (input, _) = whitespace0(input)?;
  let id = Token::merge_tokens(&mut id).unwrap();
  Ok((input, Citation{id, text: txt}))
}

// float-sigil := ">>" | "<<" ;
pub fn float_sigil(input: ParseString) -> ParseResult<FloatDirection> {
  let (input, d) = alt((float_left, float_right))(input)?;
  let d = match d.kind {
    TokenKind::FloatLeft => FloatDirection::Left,
    TokenKind::FloatRight => FloatDirection::Right,
    _ => unreachable!(),
  };
  Ok((input, d))
}

// float := float-sigil, section-element ;
pub fn float(input: ParseString) -> ParseResult<(Box<SectionElement>,FloatDirection)> {
  let (input, direction) = float_sigil(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, el) = section_element(input)?;
  Ok((input, (Box::new(el), direction)))
}

// section-element := +mech-code | question-block | info-block | list | footnote | citation | abstract-element | img | equation | markdown-table | float | quote-block | code-block | thematic-break | subtitle | paragraph ;
pub fn section_element(input: ParseString) -> ParseResult<SectionElement> {
  let (input, section_element) = match many1(mech_code)(input.clone()) {
    Ok((input, code)) => (input, SectionElement::MechCode(code)),
    _=> match question_block(input.clone()) {
      Ok((input, qblock)) => (input, qblock),
      _ => match info_block(input.clone()) {
        Ok((input, iblock)) => (input, iblock),
        _ => match mechdown_list(input.clone()) {
          Ok((input, lst)) => (input, SectionElement::List(lst)),
          _ => match footnote(input.clone()) {
            Ok((input, ftnote)) => (input, SectionElement::Footnote(ftnote)),
            _ => match citation(input.clone()) {
              Ok((input, citation)) => (input, SectionElement::Citation(citation)),
              _ => match abstract_el(input.clone()) {
                Ok((input, abstrct)) => (input, abstrct),
                _ => match img(input.clone()) {
                  Ok((input, img)) => (input, SectionElement::Image(img)),
                  _ => match equation(input.clone()) {
                    Ok((input, eqn)) => (input, SectionElement::Equation(eqn)),
                    _ => match mechdown_table(input.clone()) {
                      Ok((input, table)) => (input, SectionElement::Table(table)),
                      _ => match float(input.clone()) {
                        Ok((input, flt)) => (input, SectionElement::Float(flt)),
                        _ => match quote_block(input.clone()) {   
                          Ok((input, quote)) => (input, quote),
                          _ => match code_block(input.clone()) {
                            Ok((input, m)) => (input,m),
                            _ => match thematic_break(input.clone()) {
                              Ok((input, _)) => (input, SectionElement::ThematicBreak),
                              _ => match subtitle(input.clone()) {
                                Ok((input, subtitle)) => (input, SectionElement::Subtitle(subtitle)),
                                _ => match paragraph(input) {
                                  Ok((input, p)) => (input, SectionElement::Paragraph(p)),
                                  Err(err) => { return Err(err); }
                                }
                              }
                            }
                          }
                        }
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }
    }
  };
  let (input, _) = many0(blank_line)(input)?;
  Ok((input, section_element))
}

// section := ul_subtitle, *section-element ;
pub fn section(input: ParseString) -> ParseResult<Section> {
  let msg = "Expects user function, block, mech code block, code block, statement, paragraph, or unordered list";
  let (input, subtitle) = ul_subtitle(input)?;
  let (input, elements) = many0(tuple((is_not(ul_subtitle),section_element)))(input)?;
  let elements = elements.into_iter().map(|(_,e)| e).collect();
  Ok((input, Section{subtitle: Some(subtitle), elements}))
}

// section-elements := +section-element ;
pub fn section_elements(input: ParseString) -> ParseResult<Section> {
  let msg = "Expects user function, block, mech code block, code block, statement, paragraph, or unordered list";
  let (input, elements) = many1(tuple((is_not(ul_subtitle),section_element)))(input)?;
  let elements = elements.into_iter().map(|(_,e)| e).collect();
  Ok((input, Section{subtitle: None, elements}))
}

// body := whitespace0, (section | section_elements)+, whitespace0 ;
pub fn body(input: ParseString) -> ParseResult<Body> {
  let (input, _) = whitespace0(input)?;
  let (input, sections) = many0(alt((section,section_elements)))(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, Body{sections}))
}