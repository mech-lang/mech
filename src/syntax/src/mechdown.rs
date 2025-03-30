#[macro_use]
use crate::*;
use crate::label;
use crate::labelr;
use nom::sequence::tuple as nom_tuple;
use nom::bytes::complete::{tag, take_until};
use crate::nom::error::ParseError;

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

/*

| Syntax      | Description |
| ----------- | ----------- |
| Header      | Title       |
| Paragraph   | Text        |

Also for alignment

| Syntax      | Description | Test Text     |
| :---        |    :----:   |          ---: |
| Header      | Title       | Here's this   |
| Paragraph   | Text        | And more      |

*/

// markdown-table := ?markdown_table_header, markdown_table_row* ;
// markdown_table_header := +("|", paragraph), "|", new-line, +("|", whitespace0, alignment-separator, whitespace0), "|", new-line ;
// markdown_table_row := new-line, whitespace0, +("|", paragraph), "|" ;


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
  let (input, (header,alignment)) = markdown_table_header(input)?;
  let (input, rows) = many0(markdown_table_row)(input)?;
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

pub fn strong(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, _) = tuple((asterisk,asterisk))(input)?;
  let (input, text) = paragraph_element(input)?;
  let (input, _) = tuple((asterisk,asterisk))(input)?;
  Ok((input, ParagraphElement::Strong(Box::new(text))))
}

pub fn emphasis(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, _) = asterisk(input)?;
  let (input, text) = paragraph_element(input)?;
  let (input, _) = asterisk(input)?;
  Ok((input, ParagraphElement::Emphasis(Box::new(text))))
}

pub fn strikethrough(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, _) = tilde(input)?;
  let (input, text) = paragraph_element(input)?;
  let (input, _) = tilde(input)?;
  Ok((input, ParagraphElement::Strikethrough(Box::new(text))))
}

pub fn underline(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, _) = underscore(input)?;
  let (input, text) = paragraph_element(input)?;
  let (input, _) = underscore(input)?;
  Ok((input, ParagraphElement::Underline(Box::new(text))))
}

pub fn inline_code(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, _) = grave(input)?;
  let (input, text) = many0(tuple((is_not(grave),text)))(input)?;
  let (input, _) = grave(input)?;
  let mut text = text.into_iter().map(|(_,tkn)| tkn).collect();
  let mut text = Token::merge_tokens(&mut text).unwrap();
  text.kind = TokenKind::Text;
  Ok((input, ParagraphElement::InlineCode(text)))
}

pub fn paragraph_text(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, elements) = match many1(nom_tuple((is_not(alt((tilde,asterisk,underscore,grave,define_operator,bar))),text)))(input) {
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

pub fn paragraph_element(input: ParseString) -> ParseResult<ParagraphElement> {
  alt((hyperlink, paragraph_text, strong, emphasis, inline_code, strikethrough, underline))(input)
}

// paragraph := paragraph_starter, paragraph_element* ;
pub fn paragraph(input: ParseString) -> ParseResult<Paragraph> {
  let (input, elements) = many1(alt((paragraph_text, strong, emphasis, inline_code, strikethrough, underline)))(input)?;
  Ok((input, Paragraph{elements}))
}

// unordered_list := list_item+, new_line?, whitespace* ;
pub fn unordered_list(input: ParseString) -> ParseResult<UnorderedList> {
  let (input, items) = many1(list_item)(input)?;
  let (input, _) = opt(new_line)(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input,  UnorderedList{items}))
}

// list_item := dash, <space+>, <paragraph>, new_line* ;
pub fn list_item(input: ParseString) -> ParseResult<Paragraph> {
  let msg1 = "Expects space after dash";
  let msg2 = "Expects paragraph as list item";
  let (input, _) = dash(input)?;
  let (input, _) = labelr!(null(many1(space)), skip_nil, msg1)(input)?;
  let (input, list_item) = label!(paragraph, msg2)(input)?;
  let (input, _) = many0(new_line)(input)?;
  Ok((input,  list_item))
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
  let filtered_text: Vec<char> = text.into_iter().flat_map(|(_, s)| s.chars().collect::<Vec<char>>()).collect();
 
  match code_id {
    Some(id) => { 
      if id.to_string() == "ebnf" {
        let ebnf_text = filtered_text.iter().collect::<String>();
        match parse_grammar(&ebnf_text) {
          Ok(grammar_tree) => {return Ok((input, SectionElement::Grammar(grammar_tree)));},
          Err(err) => todo!(),
        }
      }
    },
    None => (),
  }
  let code_token = Token::new(TokenKind::CodeBlock, src_range, filtered_text);
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

// hyperlink := "[", paragraph, "]", "(", +text, ")" ;
pub fn hyperlink(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, _) = left_bracket(input)?;
  let (input, url_paragraph) = paragraph(input)?;
  let (input, _) = right_bracket(input)?;
  let (input, _) = left_parenthesis(input)?;
  let (input, url) = many1(tuple((is_not(right_parenthesis),text)))(input)?;
  let (input, _) = right_parenthesis(input)?;
  let mut tokens = url.into_iter().map(|(_,tkn)| tkn).collect::<Vec<Token>>();
  let merged = Token::merge_tokens(&mut tokens).unwrap();
  Ok((input, ParagraphElement::Hyperlink((Box::new(url_paragraph), merged))))
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
  let (input, sections) = many1(alt((section,section_elements)))(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, Body{sections}))
}