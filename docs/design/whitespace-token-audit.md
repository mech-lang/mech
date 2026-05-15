# Whitespace & Dropped-Token Audit (syntax/nodes)

## Executive summary
The parser currently **recognizes** whitespace/newlines/comments in many places, but often returns `()` or constructs AST nodes whose `tokens()` methods omit delimiters and formatting. This makes faithful source reconstruction impossible.

## Primary drop points

### 1) Core whitespace parsers discard token payloads
- `whitespace0`, `whitespace1`, `space_tab0`, `space_tab1`, `list_separator`, `enum_separator`, `ws0e`, `ws1e`, and `newline_indent` all parse whitespace/separators and return `ParseResult<()>`, so parsed whitespace is immediately dropped. (`src/syntax/src/base.rs`)
- This is the foundational loss point used all over grammar combinators.

### 2) Operator leaf macros consume surrounding whitespace but only keep operator glyph
- `ws0_leaf!` and `ws1_leaf!` call `whitespace0/whitespace1` before/after the operator and then emit one operator token with `chars` equal only to the operator text.
- Any spaces/newlines around `=`, `:=`, `->`, `=>`, `...`, etc. are consumed and unrecoverable.

### 3) Statement/code framing consumes terminal whitespace/comments separately from code item
- `mech_code_alt` consumes leading `whitespace0` before code parse.
- `code_terminal` consumes trailing `space_tab*`, optional semicolon+comment, line/eof terminator, and trailing `whitespace0`.
- `mech_code` returns `Vec<(MechCode, Option<Comment>)>`, so comments are detached side-channel values and structural whitespace is not attached to AST tokens.

### 4) Many grammar sites intentionally parse-and-ignore whitespace
- Across syntax modules, patterns like `let (input, _) = whitespace0(input)?;`, `many0(space_tab)`, and tuple wrappers with ignored separator pieces are pervasive.
- Examples include state machines and structures parsers, where indentation/row spacing and separator formatting are accepted but not preserved in node token storage.

### 5) Node token reconstitution is semantic, not lexical
- Most `tokens()` implementations in `src/core/src/nodes.rs` are compositional (append child tokens) and generally do not include punctuation/separators/whitespace unless those were explicitly stored as child tokens.
- Because separators/whitespace were often dropped during parse, `tokens()` cannot reproduce original formatting.

### 6) Some token synthesis uses default/no source range
- In number/complex token reconstruction logic, certain operator tokens are synthesized with `SourceRange::default()` (e.g., plus/minus glue), which further reduces exact source fidelity.

## Concrete hotspot list (quick index)

- **Whitespace parser sink functions**: `whitespace0`, `whitespace1`, `space_tab0`, `space_tab1`, `ws0e`, `ws1e`, `newline_indent`, `list_separator`, `enum_separator`.
- **Whitespace-eating operator constructors**: `ws0_leaf!`, `ws1_leaf!` and all operators built from them.
- **Program-level framing sink**: `mech_code_alt`, `code_terminal`, `mech_code`.
- **Recovery paths merge skipped text into coarse `Error` tokens**: `skip_till_eol`, `skip_till_end_of_statement`, `skip_till_section_element`, `skip_till_paragraph_element`.
- **AST token flattening without trivia channel**: large swaths of `impl tokens()` in `src/core/src/nodes.rs`.

## Why this blocks round-tripping
Round-tripping requires at least one preserved channel for lexical trivia (spaces/tabs/newlines/comments/separators) with source ranges and stable association to neighboring syntax nodes. Current parser design accepts trivia for parsing correctness but frequently discards it (`()`), or keeps only semantic tokens, so pretty-printer/source-reconstructor cannot recover the original text.

## Suggested next step (implementation direction)
Introduce a `Trivia` channel (leading/trailing/interstitial) in parse outputs and/or token stream retention mode, then update `tokens()`/formatter paths to include trivia when reconstructing source.
