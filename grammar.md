# Mech Grammar (10/16/2022 01:11:35)

| Symbol |   Semantics                                         |
|:------:|:----------------------------------------------------|
|  "abc" | input matches string literal "abc" (terminal)       |
|  p*    | input matches `p` for 0 or more times (repetition)  |
|  p+    | input mathces `p` for 1 or more times (repetition)  |
|  p?    | input mathces `p` for 0 or 1 time (optional)        |
| p1, p2 | input matches `p1` followed by `p2` (sequence)      |
| p1\|p2 | input matches `p1` or `p2` (ordered choice)         |
|  !!p   | input matches `p`; never consume input (peek)       |
|  !p    | input doesn't match `p`; never consume input (peek) |
| (...)  | common grouping                                     |
| <...>  | labeled grouping                                    |


## The basics

```
emoji ::= emoji_grapheme+ ;
word ::= alpha+ ;
digit1 ::= digit+ ;
digit0 ::= digit* ;
bin_digit ::= "0" | "1" ;
hex_digit ::= digit | "a" | "b" | "c" | "d" | "e" | "f" | "A" | "B" | "C" | "D" | "E" | "F" ;
oct_digit ::= "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" ;
number ::= digit1 ;
punctuation ::= period | exclamation | question | comma | colon | semicolon | dash | apostrophe | left_parenthesis | right_parenthesis | left_angle | right_angle | left_brace | right_brace | left_bracket | right_bracket ;
symbol ::= ampersand | bar | at | slash | backslash | hashtag | equal | tilde | plus | asterisk | asterisk | caret | underscore ;
paragraph_symbol ::= ampersand | at | slash | backslash | asterisk | caret | underscore ;
text ::= (word | space | number | punctuation | symbol | emoji)+ ;
paragraph_rest ::= (word | space | number | punctuation | paragraph_symbol | quote | emoij)+ ;
paragraph_starter ::= (word | number | quote | left_angle | right_angle | left_bracket | right_bracket | period | exclamation | question | comma | colon | semicolon | left_parenthesis | right_parenthesis | emoji)+ ;
identifier ::= (word | emoji), (word | number | dash | slash | emoji)* ;
boolean_literal ::= true_literal | false_literal ;
true_literal ::= english_true_literal | true_symbol ;
false_literal ::= english_false_literal | false_symbol ;
true_symbol ::= "✓" ;
false_symbol ::= "✗" ;
english_true_literal ::= "true" ;
english_false_literal ::= "false" ;
carriage_newline ::= "\r\n" ;
newline ::= new_line_char | carriage_newline ;
whitespace ::= space*, newline+ ;
number_literal ::= (hexadecimal_literal | octal_literal | binary_literal | decimal_literal | float_literal), kind_annotation? ;
float_literal ::= "."?, digit1, "."?, digit0 ;
decimal_literal ::= "0d", <digit1> ;
hexadecimal_literal ::= "0x", <hex_digit+> ;
octal_literal ::= "0o", <oct_digit+> ;
binary_literal ::= "0b", <bin_digit+> ;
value ::= empty | boolean_literal | number_literal | string ;
empty ::= underscore+ ;
```

## Blocks


### Data

```
select_all ::= colon ;
subscript ::= (select_all | expression | tilde), space*, comma?, space* ;
subscript_index ::= left_brace, <subscript+>, <right_brace> ;
single_subscript_index ::= left_brace, <subscript>, <right_brace> ;
dot_index ::= period, <identifier>, single_subscript_index? ;
swizzle ::= period, identifier, comma, !space, <identifier, (",", identifier)*> ;
reshape_column ::= left_brace, colon, right_brace ;
index ::= swizzle | dot_index | reshape_column | subscript_index ;
data ::= (table | identifier), index*, transpose? ;
kind_annotation ::= left_angle, <(identifier | underscore), (",", (identifier | underscore))*>, <right_angle> ;
```

### Tables

```
table ::= hashtag, <identifier> ;
binding ::= s*, identifier, kind_annotation?, <!(space+, colon)>, colon, s+,
            <empty | expression | identifier | value>, <!!right_bracket | (s*, comma, <s+>) | s+> ;
   where s ::= space | newline | tab ;
binding_strict ::= s*, identifier, kind_annotation?, <!(space+, colon)>, colon, <s+>,
                   <empty | expression | identifier | value>, <!!right_bracket | (s*, comma, <s+>) | s+> ;
   where s ::= space | newline | tab ;
function_binding ::= identifier, <colon>, <space+>, <expression | identifier | value>, space*, comma?, space* ;
table_column ::= (space | tab)*, (expression | value | data), comma?, (space | tab)* ;
table_row ::= (space | tab)*, table_column+, semicolon?, newline? ;
attribute ::= identifier, kind_annotation?, space*, comma?, space* ;
table_header ::= bar, <attribute+>, <bar>, space*, newline? ;
anonymous_table ::= left_bracket, (space | newline | tab)*, table_header?,
                    ((comment, newline) | table_row)*, (space | newline | tab)*, <right_bracket> ;
empty_table ::= left_bracket, (space | newline | tab)*, table_header?, (space | newline | tab)*, right_bracket ;
inline_table ::= left_bracket, binding, <binding_strict*>, <right_bracket> ;
```

### Statements

```
stmt_operator ::= split_operator | flatten_operator | set_operator | update_operator | add_row_operator | equal ;
comment_sigil ::= "--" ;
comment ::= (space | tab)*, comment_sigil, <text>, <!!newline> ;
add_row_operator ::= "+=" ;
add_row ::= table, <!stmt_operator>, space*, add_row_operator, <space+>, <expression | inline_table | anonymous_table> ;
add_update_operator ::= ":+=" ;
subtract_update_operator ::= ":-=" ;
multiply_update_operator ::= ":*=" ;
divide_update_operator ::= ":/=" ;
update_operator ::= add_update_operator | subtract_update_operator | multiply_update_operator | divide_update_operator ;
update_data ::= data, <!stmt_operator>, space*, update_operator, <space+>, <expression> ;
set_operator ::= ":=" ;
set_data ::= data, <!stmt_operator>, space*, set_operator, <space+>, <expression> ;
split_data ::= (identifier | table), <!stmt_operator>, space*, split_operator, <space+>, <expression> ;
flatten_data ::= identifier, <!stmt_operator>, space*, flatten_operator, <space+>, <expression> ;
variable_define ::= identifier, <!stmt_operator>, space*, equal, <space+>, <expression> ;
table_define ::= table, kind_annotation?, <!stmt_operator>, space*, equal, <space+>, <expression> ;
split_operator ::= ">-" ;
flatten_operator ::= "-<" ;
whenever_oeprator ::= "~" ;
until_operator ::= "~|" ;
wait_operator ::= "|~" ;
whenever_data ::= whenever_operator, <space>, <!space>, <variable_define | expression | data> ;
wait_data ::= wait_operator, <space>, <!space>, <variable_define | expression | data> ;
until_data ::= until_operator, <space>, <!space>, <variable_define | expression | data> ;
statement ::= (table_define | variable_define | split_data  | flatten_data | whenever_data | wait_data |
               until_data   | set_data        | update_data | add_row      | comment ), space*, <newline+> ;
```

### Expressions


#### Math expressions

```
parenthetical_expression ::= left_parenthesis, <l0>, <right_parenthesis> ;
negation ::= dash, !(dash | space), <data | value> ;
function ::= identifier, left_parenthesis, <function_binding+>, <right_parenthesis> ;
user_function ::= left_bracket, function_output*, <right_bracket>, <space+>, <equal>, <space+>, <identifier>,
                  <left_parenthesis>, <function_input*>, <right_parenthesis>, <newline>, <function_body> ;
function_output ::= identifier, <kind_annotation>, space*, comma?, space* ;
function_input ::= identifier, <kind_annotation>, space*, comma?, space* ;
function_body ::= indented_tfm+, whitespace* ;
matrix_multiply ::= "**" ;
add ::= "+" ;
subtract ::= "-" ;
multiply ::= "*" ;
divide ::= "/" ;
exponent ::= "^" ;
range_op ::= colon ;
l0 ::= l1, l0_infix* ;
l0_infix ::= <!(space+, colon)>, range_op, <!space>, <l1> ;
l1 ::= l2, l1_infix* ;
l1_op ::= add | subtract ;
l1_infix ::= <!l1_op>, space*, !negation, !comment_sigil, l1_op, <space+>, <l2> ;
l2 ::= l3, l2_infix* ;
l2_op ::= matrix_multiply | multiply | divide ;
l2_infix ::= <!l2_op>, space*, l2_op, <space+>, <l3> ;
l3 ::= l4, l3_infix* ;
l3_op ::= exponent ;
l3_infix ::= <!l3_op>, space*, l3_op, <space+>, <l4> ;
l4 ::= l5, l4_infix* ;
l4_op ::= and | or | xor ;
l4_infix ::= <!l4_op>, space*, l4_op, <space+>, <l5> ;
l5 ::= l6, l5_infix* ;
l5_op ::= not_equal | equal_to | greater_than_equal | greater_than | less_than_equal | less_than ;
l5_infix ::= <!l5_op>, space*, l5_op, <space+>, <l6> ;
l6 ::= empty_table | string | anonymous_table | function | value | not | data | negation | parenthetical_expression ;
math_expression ::= l0 ;
```

#### Filter expressions

```
not_equal ::= "!=" | "¬=" | "≠" ;
equal_to ::= "==" ;
greater_than ::= ">" ;
less_than ::= "<" ;
greater_than_equal ::= ">=" | "≥" ;
less_than_equal ::= "<=" | "≤" ;
```

#### Logic expressions

```
or ::= "|" ;
and ::= "&" ;
not ::= "!" | "¬" ;
xor ::= "xor" | "⊕" | "⊻" ;
```

#### Other expressions

```
string ::= quote, (!quote, <text>)*, quote ;
transpose ::= "'" ;
expression ::= (empty_table | inline_table | math_expression | string | anonymous_table), transpose? ;
```

### Block basics

```
transformation ::= statement;
empty_line ::= space*, newline ;
indented_tfm ::= !empty_line, space, <space>, <!space>, <transformation> ;
block ::= indented_tfm+, whitespace* ;
```

## Markdown

```
ul_title ::= space*, text, space*, newline, equal+, space*, newline* ;
title ::= ul_title ;
ul_subtitle ::= space*, text, space*, newline, dash+, space*, newline* ;
subtitle ::= ul_subtitle ;
inline_code ::= grave, text, grave, space* ;
paragraph_text ::= paragraph_starter, paragraph_rest? ;
paragraph ::= (inline_code | paragraph_text)+, whitespace*, newline* ;
unordered_list ::= list_item+, newline?, whitespace* ;
list_item ::= dash, <space+>, <paragraph>, newline* ;
formatted_text ::= (!grave, !eof, <paragraph_rest | carriage_return | new_line_char>)* ;
code_block ::= grave, <grave>, <grave>, <newline>, formatted_text, <grave{3}, newline, whitespace*> ;
```

## Mechdown

```
mech_code_block ::= grave{3}, !!"mec", <"mech:">, text?, <newline>, <block>, <grave{3}, newline>, whitespace* ;
```

## Start here

```
section_element ::= user_function | block | mech_code_block | code_block | statement | subtitle | paragraph | unordered_list;
section ::= (!eof, <section_element>, whitespace?)+ ;
body ::= whitespace*, section+ ;
program ::= whitespace?, title?, <body>, whitespace?, space* ;
parse_mech_fragment ::= statement ;
parse_mech ::= program | statement ;
```
## Table
```ebnf
table_title = "│#", identifier, ["+"], space, "(", number, space, "x", space, number, ")", {space}, "│", newline;
table_type = "U8"|"U16"|"U32"|"U64"|"U128"|"I8"|"I16"|"I32"|"I64"|"I128"|"F32"|"F64"|"Bool"|"String";
<!-- table_topline = "╭",{"-"}, "╮",newline; -->
table_line = "╭" | "├" | "╰",{"-",["┼" | "┬" | "┴"],"-"},"╮" | "┤" | "╯",newline;
<!-- table_botline = "╰",{"-",["┴"],"-"}, "╯",newline; -->
table_label = "│" , [{identifier, {space}, "│"}], newline
table = table_line, table_title, table_line, [table_label],table_line, ["│", {table_type, {space}, "│"}, newline], {"│", {expressions, {space}, "│"}, newline},table_line;
```