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
  bytes::complete::take_until,
  Err,
  Err::Failure
};

use std::collections::HashMap;
use colored::*;

use crate::*;

// ### Mechdown

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

pub fn markdown_table(input: ParseString) -> ParseResult<MarkdownTable> {
  let (input, _) = whitespace0(input)?;
  let (input, table) = alt((markdown_table_with_header, markdown_table_no_header))(input)?;
  Ok((input, table))
}

pub fn markdown_table_with_header(input: ParseString) -> ParseResult<MarkdownTable> {
  let (input, (header,alignment)) = markdown_table_header(input)?;
  let (input, rows) = many1(markdown_table_row)(input)?;
  Ok((input, MarkdownTable{header, rows, alignment}))
}

pub fn markdown_table_no_header(input: ParseString) -> ParseResult<MarkdownTable> {
  let (input, rows) = many1(markdown_table_row)(input)?;
  let header = vec![];
  let alignment = vec![];
  Ok((input, MarkdownTable{header, rows, alignment}))
}

pub fn markdown_table_header(input: ParseString) -> ParseResult<(Vec<Paragraph>,Vec<ColumnAlignment>)> {
  let (input, _) = whitespace0(input)?;
  let (input, header) = many1(tuple((bar, paragraph)))(input)?;
  let (input, _) = bar(input)?;
  let (input, _) = new_line(input)?;
  let (input, _) = whitespace0(input)?;
  let (input, alignment) = many1(tuple((bar, alignment_separator)))(input)?;
  let (input, _) = bar(input)?;
  let (input, _) = new_line(input)?;
  let column_names: Vec<Paragraph> = header.into_iter().map(|(_,tkn)| tkn).collect();
  let column_alignments = alignment.into_iter().map(|(_,tkn)| tkn).collect();
  Ok((input, (column_names,column_alignments)))
}

// markdown_table_row := +(bar, paragraph), bar, *whitespace ;
pub fn markdown_table_row(input: ParseString) -> ParseResult<Vec<Paragraph>> {
  let (input, _) = whitespace0(input)?;
  let (input, row) = many1(tuple((bar, paragraph)))(input)?;
  let (input, _) = bar(input)?;
  let (input, _) = whitespace0(input)?;
  let row = row.into_iter().map(|(_,tkn)| tkn).collect();
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

// number_subtitle := (space|tab)*, "(", integer_literal, ")", (space|tab)+, text+, (space|tab)*, whitespace* ;
pub fn number_subtitle(input: ParseString) -> ParseResult<Subtitle> {
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = left_parenthesis(input)?;
  let (input, _) = integer_literal(input)?;
  let (input, _) = right_parenthesis(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, text) = paragraph(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, Subtitle{text, level: 3}))
}

// alpha_subtitle := (space|tab)*, "(", alpha, ")", (space|tab)+, text+, (space|tab)*, whitespace* ;
pub fn alpha_subtitle(input: ParseString) -> ParseResult<Subtitle> {
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = left_parenthesis(input)?;
  let (input, _) = alpha(input)?;
  let (input, _) = right_parenthesis(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, text) = paragraph(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, Subtitle{text, level: 4}))
}

// strong := (asterisk, asterisk), +paragraph_element, (asterisk, asterisk) ;
pub fn strong(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, _) = tuple((asterisk,asterisk))(input)?;
  let (input, text) = paragraph_element(input)?;
  let (input, _) = tuple((asterisk,asterisk))(input)?;
  Ok((input, ParagraphElement::Strong(Box::new(text))))
}

/// emphasis := asterisk, +paragraph_element, asterisk ;
pub fn emphasis(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, _) = asterisk(input)?;
  let (input, text) = paragraph_element(input)?;
  let (input, _) = asterisk(input)?;
  Ok((input, ParagraphElement::Emphasis(Box::new(text))))
}

// strikethrough := tilde, +paragraph_element, tilde ;
pub fn strikethrough(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, _) = tilde(input)?;
  let (input, text) = paragraph_element(input)?;
  let (input, _) = tilde(input)?;
  Ok((input, ParagraphElement::Strikethrough(Box::new(text))))
}

/// underline := underscore, +paragraph_element, underscore ;
pub fn underline(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, _) = underscore(input)?;
  let (input, text) = paragraph_element(input)?;
  let (input, _) = underscore(input)?;
  Ok((input, ParagraphElement::Underline(Box::new(text))))
}

// inline_code := grave, +text, grave ;
pub fn inline_code(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, _) = grave(input)?;
  let (input, text) = many0(tuple((is_not(grave),text)))(input)?;
  let (input, _) = grave(input)?;
  let mut text = text.into_iter().map(|(_,tkn)| tkn).collect();
  let mut text = Token::merge_tokens(&mut text).unwrap();
  text.kind = TokenKind::Text;
  Ok((input, ParagraphElement::InlineCode(text)))
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

// raw-hyperlink := ("https" | "http" |)
pub fn raw_hyperlink(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, _) = peek(http_prefix)(input)?;
  let (input, address) = many1(tuple((is_not(space), text)))(input)?;
  let mut tokens = address.into_iter().map(|(_,tkn)| tkn).collect::<Vec<Token>>();
  let url = Token::merge_tokens(&mut tokens).unwrap();
  Ok((input, ParagraphElement::Hyperlink((url.clone(), url))))
}

// img := "![", +text, "]", "(", +text, ")" ;
pub fn img(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, _) = img_prefix(input)?;
  let (input, caption_text) = many1(tuple((is_not(right_bracket),text)))(input)?;
  let (input, _) = right_bracket(input)?;
  let (input, _) = left_parenthesis(input)?;
  let (input, src) = many1(tuple((is_not(right_parenthesis),text)))(input)?;
  let (input, _) = right_parenthesis(input)?;
  let merged_caption = Token::merge_tokens(&mut caption_text.into_iter().map(|(_,tkn)| tkn).collect::<Vec<Token>>()).unwrap();
  let merged_src = Token::merge_tokens(&mut src.into_iter().map(|(_,tkn)| tkn).collect::<Vec<Token>>()).unwrap();
  Ok((input, ParagraphElement::Image( Image{src: merged_src, caption: Some(merged_caption)} )))
}

// paragraph_text := ¬(img_prefix | http_prefix | left_bracket | tilde | asterisk | underscore | grave | define_operator | bar), +text ;
pub fn paragraph_text(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, elements) = match many1(nom_tuple((is_not(alt((img_prefix,http_prefix,left_brace,left_bracket,tilde,asterisk,underscore,grave,define_operator,bar))),text)))(input) {
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

// inline-mech-cdoe := "{", expression, "}" ;`
pub fn inline_mech_code(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, _) = left_brace(input)?;
  let (input, expr) = expression(input)?;
  let (input, _) = right_brace(input)?;
  Ok((input, ParagraphElement::InlineMechCode(expr)))
}

// paragraph_element := hyperlink | raw_hyperlink | img | paragraph_text | strong | emphasis | inline_code | strikethrough | underline ;
pub fn paragraph_element(input: ParseString) -> ParseResult<ParagraphElement> {
  alt((hyperlink, raw_hyperlink, img, inline_mech_code, paragraph_text, strong, emphasis, inline_code, strikethrough, underline))(input)
}

// paragraph := +paragraph_element ;
pub fn paragraph(input: ParseString) -> ParseResult<Paragraph> {
  let (input, elements) = many1(paragraph_element)(input)?;
  Ok((input, Paragraph{elements}))
}

// unordered_list := +list_item, ?new_line, *whitespace ;
pub fn unordered_list(input: ParseString) -> ParseResult<UnorderedList> {
  let (input, items) = many1(list_item)(input)?;
  let (input, _) = opt(new_line)(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input,  UnorderedList{items}))
}

// list_item := dash, <space+>, <paragraph>, new_line* ;
pub fn list_item(input: ParseString) -> ParseResult<(Option<Token>,Paragraph)> {
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

// code_block := grave, <grave>, <grave>, <new_line>, any, <grave{3}, new_line, whitespace*> ;
pub fn code_block(input: ParseString) -> ParseResult<SectionElement> {
  let msg1 = "Expects 3 graves to start a code block";
  let msg2 = "Expects new_line";
  let msg3 = "Expects 3 graves followed by new_line to terminate a code block";
  let (input, (_, r)) = range(nom_tuple((
    grave,
    label!(grave, msg1),
    label!(grave, msg1),
  )))(input)?;
  let (input, code_id) = opt(identifier)(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = label!(new_line, msg2)(input)?;
  let (input, (text,src_range)) = range(many0(nom_tuple((
    is_not(nom_tuple((grave, grave, grave))),
    any,
  ))))(input)?;
  let (input, _) = nom_tuple((grave, grave, grave))(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = new_line(input)?;
  let block_src: Vec<char> = text.into_iter().flat_map(|(_, s)| s.chars().collect::<Vec<char>>()).collect();
 
  match code_id {
    Some(id) => { 
      match id.to_string().as_str() {
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

            // get rid of the prefix and then treat the rest of the string as an identifier
            let rest = tag.trim_start_matches("mech").trim_start_matches("mec").trim_start_matches("🤖");
            let code_id = if rest == "" { 0 } else {
              hash_str(rest)
            };

            let mech_src = block_src.iter().collect::<String>();
            let graphemes = graphemes::init_source(&mech_src);
            let parse_string = ParseString::new(&graphemes);

            match many1(mech_code)(parse_string) {
              Ok((_, mech_tree)) => {
                // TODO what if not all the input is parsed? Is that handled?
                return Ok((input, SectionElement::FencedMechCode((mech_tree,code_id))));
              },
              Err(err) => {
                println!("Error parsing Mech code: {:?}", err);
                todo!();
              }
            };
          } else {
            // Some other code block, just keep moving although we might want to do something with it later
          }
        }
      } 
    },
    None => (),
  }
  let code_token = Token::new(TokenKind::CodeBlock, src_range, block_src);
  Ok((input, SectionElement::CodeBlock(code_token)))
}

pub fn block_quote(input: ParseString) -> ParseResult<Paragraph> {
  let (input, _) = right_angle(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, text) = paragraph(input)?;
  let (input, _) = many0(space_tab)(input)?;
  Ok((input, text))
}

pub fn thematic_break(input: ParseString) -> ParseResult<SectionElement> {
  let (input, _) = many1(asterisk)(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = new_line(input)?;
  Ok((input, SectionElement::ThematicBreak))
}

// section_element := mech_code | unordered_list | comment | paragraph | code_block | sub_section;
pub fn section_element(input: ParseString) -> ParseResult<SectionElement> {
  let (input, section_element) = match many1(mech_code)(input.clone()) {
    Ok((input, code)) => (input, SectionElement::MechCode(code)),
    _ => match unordered_list(input.clone()) {
      Ok((input, list)) => (input, SectionElement::UnorderedList(list)),
      _ => match markdown_table(input.clone()) {
        Ok((input, table)) => (input, SectionElement::Table(table)),
        _ => match block_quote(input.clone()) {   
          Ok((input, quote)) => (input, SectionElement::BlockQuote(quote)),
          _ => match code_block(input.clone()) {
            Ok((input, m)) => (input,m),
            _ => match thematic_break(input.clone()) {
              Ok((input, _)) => (input, SectionElement::ThematicBreak),
              _ => match sub_section(input.clone()) {
                Ok((input, s)) => (input, SectionElement::Section(Box::new(s))),
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
  };
  let (input, _) = whitespace0(input)?;
  Ok((input, section_element))
}

// sub_section_element := comment | unordered_list | mech_code | paragraph | code_block;
pub fn sub_section_element(input: ParseString) -> ParseResult<SectionElement> {
  let (input, section_element) = match many1(mech_code)(input.clone()) {
    Ok((input, code)) => (input, SectionElement::MechCode(code)),
    _ => match unordered_list(input.clone()) {
      Ok((input, list)) => (input, SectionElement::UnorderedList(list)),
      _ => match markdown_table(input.clone()) {
        Ok((input, table)) => (input, SectionElement::Table(table)),
        _ => match block_quote(input.clone()) {   
          Ok((input, quote)) => (input, SectionElement::BlockQuote(quote)),
          _ => match code_block(input.clone()) {
            Ok((input, m)) => (input,m),
            _ => match thematic_break(input.clone()) {
              Ok((input, _)) => (input, SectionElement::ThematicBreak),
              _ => match paragraph(input) {
                Ok((input, p)) => (input, SectionElement::Paragraph(p)),
                Err(err) => { return Err(err); }
              }
            }
          }
        }
      }
    }
  };
  let (input, _) = whitespace0(input)?;
  Ok((input, section_element))
}

// section := ul_subtitle, section_element* ;
pub fn section(input: ParseString) -> ParseResult<Section> {
  let msg = "Expects user function, block, mech code block, code block, statement, paragraph, or unordered list";
  let (input, subtitle) = ul_subtitle(input)?;
  let (input, elements) = many0(tuple((is_not(ul_subtitle),section_element)))(input)?;
  let elements = elements.into_iter().map(|(_,e)| e).collect();
  Ok((input, Section{subtitle: Some(subtitle), elements}))
}

// section_elements := section_element+ ;
pub fn section_elements(input: ParseString) -> ParseResult<Section> {
  let msg = "Expects user function, block, mech code block, code block, statement, paragraph, or unordered list";
  let (input, elements) = many1(tuple((is_not(ul_subtitle),section_element)))(input)?;
  let elements = elements.into_iter().map(|(_,e)| e).collect();
  Ok((input, Section{subtitle: None, elements}))
}

// sub_section := alpha_subtitle, sub_section_element* ;
pub fn sub_section(input: ParseString) -> ParseResult<Section> {
  let msg = "Expects user function, block, mech code block, code block, statement, paragraph, or unordered list";
  let (input, subtitle) = alpha_subtitle(input)?;
  let (input, elements) = many0(tuple((is_not(alt((ul_subtitle,alpha_subtitle))),sub_section_element)))(input)?;
  let elements = elements.into_iter().map(|(_,e)| e).collect();
  Ok((input, Section{subtitle: Some(subtitle), elements}))
}

// body := whitespace0, (section | section_elements)+, whitespace0 ;
pub fn body(input: ParseString) -> ParseResult<Body> {
  let (input, _) = whitespace0(input)?;
  let (input, sections) = many0(alt((section,section_elements)))(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, Body{sections}))
}