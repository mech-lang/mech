#[macro_use]
use crate::*;
use crate::label;
use crate::labelr;
use nom::sequence::tuple as nom_tuple;

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

// subtitle := text+, new_line, dash+, (space|tab)*, whitespace* ;
pub fn ul_subtitle(input: ParseString) -> ParseResult<Subtitle> {
  let (input, _) = many1(digit_token)(input)?;
  let (input, _) = period(input)?;
  let (input, _) = many0(space)(input)?;
  let (input, mut text) = many1(text)(input)?;
  let (input, _) = new_line(input)?;
  let (input, _) = many1(dash)(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = new_line(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = whitespace0(input)?;
  let mut title = Token::merge_tokens(&mut text).unwrap();
  title.kind = TokenKind::Title;
  Ok((input, Subtitle{text: title}))
}

// number_subtitle := space*, number, period, space+, text, space*, new_line* ;
pub fn number_subtitle(input: ParseString) -> ParseResult<Subtitle> {
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = left_parenthesis(input)?;
  let (input, _) = integer_literal(input)?;
  let (input, _) = right_parenthesis(input)?;
  let (input, _) = many1(space_tab)(input)?;
  let (input, mut text) = many1(text)(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = whitespace0(input)?;
  let mut title = Token::merge_tokens(&mut text).unwrap();
  title.kind = TokenKind::Title;
  Ok((input, Subtitle{text: title}))
}

// alpha_subtitle := space*, alpha, right_parenthesis, space+, text, space*, new_line* ;
pub fn alpha_subtitle(input: ParseString) -> ParseResult<Subtitle> {
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = left_parenthesis(input)?;
  let (input, _) = alpha(input)?;
  let (input, _) = right_parenthesis(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, mut text) = many1(text)(input)?;
  let (input, _) = many0(space_tab)(input)?;
  let (input, _) = whitespace0(input)?;
  let mut title = Token::merge_tokens(&mut text).unwrap();
  title.kind = TokenKind::Title;
  Ok((input, Subtitle{text: title}))
}

// paragraph_symbol := ampersand | at | slash | backslash | asterisk | caret | hashtag | underscore ;
pub fn paragraph_symbol(input: ParseString) -> ParseResult<Token> {
  let (input, symbol) = alt((ampersand, at, slash, backslash, asterisk, caret, hashtag, underscore, equal, tilde, plus, percent))(input)?;
  Ok((input, symbol))
}

// paragraph_starter := (word | number | quote | left_angle | right_angle | left_bracket | right_bracket | period | exclamation | question | comma | colon | semicolon | left_parenthesis | right_parenthesis | emoji)+ ;
pub fn paragraph_starter(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, text) = alt((alpha_token, quote))(input)?;
  Ok((input, ParagraphElement::Start(text)))
}

// paragraph_element := text+ ;
pub fn paragraph_element(input: ParseString) -> ParseResult<ParagraphElement> {
  let (input, elements) = match many1(nom_tuple((is_not(define_operator),text)))(input) {
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

// paragraph := (inline_code | paragraph_text)+, whitespace*, new_line* ;
pub fn paragraph(input: ParseString) -> ParseResult<Paragraph> {
  let (input, first) = paragraph_starter(input)?;
  let (input, mut rest) = many0(paragraph_element)(input)?;
  let mut elements = vec![first];
  elements.append(&mut rest);
  Ok((input, Paragraph{elements}))
}

// unordered_list := list_item+, new_line?, whitespace* ;
pub fn unordered_list(input: ParseString) -> ParseResult<UnorderedList> {
  let (input, items) = many1(list_item)(input)?;
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


// code_block := grave, <grave>, <grave>, <new_line>, formatted_text, <grave{3}, new_line, whitespace*> ;
pub fn code_block(input: ParseString) -> ParseResult<SectionElement> {
  let msg1 = "Expects 3 graves to start a code block";
  let msg2 = "Expects new_line";
  let msg3 = "Expects 3 graves followed by new_line to terminate a code block";
  let (input, (_, r)) = range(nom_tuple((
    grave,
    label!(grave, msg1),
    label!(grave, msg1),
  )))(input)?;
  let (input, _) = label!(new_line, msg2)(input)?;
  //let (input, text) = formatted_text(input)?;
  let (input, _) = label!(nom_tuple((grave, grave, grave, new_line, whitespace0)), msg3, r)(input)?;
  Ok((input, SectionElement::CodeBlock))
}

// section_element := mech_code | unordered_list | comment | paragraph | code_block | sub_section;
pub fn section_element(input: ParseString) -> ParseResult<SectionElement> {
  let (input, section_element) = match mech_code(input.clone()) {
    Ok((input, code)) => (input, SectionElement::MechCode(code)),
    //Err(Failure(err)) => {return Err(Failure(err));}
    _ => match unordered_list(input.clone()) {
      Ok((input, list)) => (input, SectionElement::UnorderedList(list)),
      //Err(Failure(err)) => {return Err(Failure(err));}
      _ => match comment(input.clone()) {
        Ok((input, comment)) => (input, SectionElement::Comment(comment)),
        //Err(Failure(err)) => {return Err(Failure(err));}
        _ => match paragraph(input.clone()) {
          Ok((input, p)) => (input, SectionElement::Paragraph(p)),
          //Err(Failure(err)) => {return Err(Failure(err));}
          _ => match code_block(input.clone()) {
            Ok((input, m)) => (input,SectionElement::CodeBlock),
            //Err(Failure(err)) => {return Err(Failure(err));}
            _ => match sub_section(input) {
              Ok((input, s)) => (input, SectionElement::Section(Box::new(s))),
              Err(err) => { return Err(err); }
            }
          }
        }
      }
    }
  };
  let (input, _) = whitespace0(input)?;
  Ok((input, section_element))
}

// section_element := comment | unordered_list | mech_code | paragraph | code_block;
pub fn sub_section_element(input: ParseString) -> ParseResult<SectionElement> {
  let (input, section_element) = match comment(input.clone()) {
    Ok((input, comment)) => (input, SectionElement::Comment(comment)),
    _ => match unordered_list(input.clone()) {
      Ok((input, list)) => (input, SectionElement::UnorderedList(list)),
      _ => match mech_code(input.clone()) {
        Ok((input, m)) => (input, SectionElement::MechCode(m)),
        _ => match paragraph(input.clone()) {
          Ok((input, p)) => (input, SectionElement::Paragraph(p)),
          _ => match code_block(input.clone()) {
            Ok((input, m)) => (input,SectionElement::CodeBlock),
            Err(err) => { return Err(err); }
          }
        }
      }
    }
  };
  let (input, _) = whitespace0(input)?;
  Ok((input, section_element))
}

// section := ul_subtitle?, section_element+ ;
pub fn section(input: ParseString) -> ParseResult<Section> {
  let msg = "Expects user function, block, mech code block, code block, statement, paragraph, or unordered list";
  let (input, subtitle) = opt(ul_subtitle)(input)?;
  let (input, elements) = many1(section_element)(input)?;
  Ok((input, Section{subtitle, elements}))
}

// sub_section := alpha_subtitle, sub_section_element* ;
pub fn sub_section(input: ParseString) -> ParseResult<Section> {
  let msg = "Expects user function, block, mech code block, code block, statement, paragraph, or unordered list";
  let (input, subtitle) = alpha_subtitle(input)?;
  let (input, elements) = many0(sub_section_element)(input)?;
  Ok((input, Section{subtitle: Some(subtitle), elements}))
}


// body := section+ ;
pub fn body(input: ParseString) -> ParseResult<Body> {
  let (input, _) = whitespace0(input)?;
  let (input, sections) = many1(section)(input)?;
  let (input, _) = whitespace0(input)?;
  Ok((input, Body{sections}))
}