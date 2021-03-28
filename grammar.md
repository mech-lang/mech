# Mech Grammar

For now, the formal specification of the Mech grammar will be the Rust implementation. I will try to reflect that grammar in this document in [EBNF](https://en.wikipedia.org/wiki/Extended_Backus–Naur_form). Then this document can be used to generate Mech parsers in any number of languages.



##Parser
```ebnf
```



### Primitives
```ebnf
space = " ";
period = ".";
exclamation = "!";
question = "?";
comma = ".";
colon = ":";
semicolon = ";";
dash = "-";
apostrophe = "'";
quote = "\"";
left_parenthesis = "(";
right_parenthesis = ")";
left_angle = "<";
right_angle = ">";
left_brace = "{";
right_brace = "}";
ampersand = "&";
bar = "|";
at = "@";
slash = "/";
hashtag = "#";
equal = "=";
tilde = "~";
plus = "+";
asterisk = "*";
caret = "^";
underscore = "_";
tab = "\t"
left_bracket = "[";
right_bracket = "]";
carriage_newline = "\r\n";
new_line_char = "\n";
carriage_return = "\r";
grave = "`";
```

### Values
```ebnf
--
digit excluding zero = "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" 
digit                = "0" | ?digit excluding zero?
natural number = ?digit excluding zero?, { digit } ;
alpha = ?letters of alphabet?;
hex_digit = {digit | ?letter A-F? };
oct_digit = "1" | "2" | "3" | "4" | "5" | "6" | "7" | "0";
```

##The Basics
```ebnf
word = {alpha};
number = {digit};
punctuation = period | exclamation | question | comma | colon | semicolon | dash | apostrophe | left_parenthesis | right_parenthesis |  left_angle | right_angle | left_brace | right_brace;
symbol = ampersand | bar | at | slash | hashtag | equal | tilde | plus | asterisk | caret | underscore;
single_text = word | space | punctuation | symbol;
text = {word | space | number | punctuation | symbol};
paragraph_rest = {word | space | number | punctuation | symbol | quote};
paragraph_starter =  {word | number | quote | left_angle | right_angle | left_bracket | right_bracket | period | exclamation | question | comma | colon | semicolon | left_parenthesis| right_parenthesis};
identifier = word , {word | number | dash | slash};
true_literal = "true";
false_literal = "false";
whitespace =  {" "}, newline;
floating_point = "." , {digit}; 
quantity = number , [floating_point] , [identifier];
rational_number = quantity | number_literal, "/", quantity | number_literal
number_literal = decimal_literal | hexadecimal_literal | octal_literal | binary_literal;
decimal_literal = "0d" , {digit};
hexadecimal_literal = "0x" , {hex_digit};
octal_literal = "0o", {oct_digit};
binary_literal = "0b", {"1" | "0"};
constant = string | quantity;
empty = {"_"};
newline = new_line_char | carriage_newline;
```

##Blocks
```ebnf
block = {transformation}, {whitespace};
transformation = space, space, statement, space, ["\n];
```
###Data
```ebnf
select_all = colon;
subscript = select_all | expression, space, [comma], space;
subscript_index = left_bracket, {subscripts}, right_bracket;
dot_index = period, identifier;
index = dot_index | subscript_index;
data = table | identifier , {index};
```

###Tables
```ebnf
table = hashtag, identifier;
binding = identifier, ": ", empty | expression | identifier | constant, space, [comma], space;
function_binding = identifier, colon, space, empty | expression | identifier | constant, space, [comma], space;
table_column = {space | tab} , true_literal | false_literal | empty | data | expression | rational_number | number_literal | quantity, [comma], {space| tab};
table_row = {space | tab}, {table_column}, [semicolon], [newline];
attribute = identifier, space, [comma], space;
table_header = bar , {attribute}, bar, space, [newline];
anonymous_table = left_bracket, space, [table_header], {table_row}, right_bracket;
inline_table = left_bracket, {binding} , right_bracket;
```

###Statements
```ebnf
comment_sigil = "//";
comment = comment_sigil, text;
add_row_operator = "+=";
add_row = table, space, add_row_operator, space, inline_table | anonymous_table;
set_operator = ":=";
set_data = data, space, set_operator, space, expression;
split_data = identifier | table, space, split_operator, space, expression;
join_data = identifier, space, join_operator, space, expression;
variable_define =  identifier, space, equal, space, expression;
table_define = table, space , equal, space, expression;
split_operator = ">-";
join_operator = "-<";
whenever_operator = "~";
until_operator = "~|";
wait_operator = "|~";
whenever_data = whenever_operator, space, variable_define | expression |data;
wait_data = wait_operator, space, variable_define | expression |data;
until_data = until_operator, space, variable_define | expression |data;
statements = table_define | variable_define | split_data | join_data | whenever_data | wait_data | until_data | set_data | add_row |       comment;
```

###Expression

####Math Expressions
```ebnf
parenthetical_expression = left_parenthesis, l0, right_parenthesis;
negation = dash, data | constant;
function = identifier, left_parenthesis, {function_binding}, right_parenthesis;
matrix_multiply = "**";
add = "+";
subtract = "-";
multiply = "*";
divide = "/";
exponent = "^";
range_op = ":";
l0 = l1, {l0_infix};
l0_infix = space, range_op, space, l1;
l1 = l2, {l1_infix};
l1_infix = space, add | subtract, space, l2;
l2 = l3, {l2_infix};
l2_infix = space, multiply | divide | matrix_multiply, space, l3;
l3 = l4, {l3_infix};
l3_infix = space, exponent, space, l4;
l4 = l5, {l4_infix};
l4_infix = space, and | or, space, l5;
l5 = l6, {l5_infix};
l5_infix = space, not_equal | equal_to | greater_than_equal | greater_than | less_than_equal | less_than, space, l6;
l6 = empty | true_literal | false_literal | anonymous_table | function | data | string | rational_number | number_literal | quantity |negation | parenthetical_expression;
math_expression = l0;
```

####Filter Expressions
```ebnf
not_equal = "!=";
equal_to = "==";
geater_than = ">"
less_than = "<";
greater_than_equal = ">=";
less_than_equal = "<=";
```

####Logic Expressions
```ebnf
or = "|";
and = "&";
```

####Other Expressions
```ebnf
expressions = string | inline_table | math_expressions | anonymous table;
string = quote , {string_interpolation | text}, quote;
string_interpolation = "{{" , expression, "}}" ;
```

##MarkDown
```ebnf
title = "#" , {space} , text , {whitespace};
subtitle = hashtag, hashtag, space, text, {whitespace}; 
section_title = hashtag, hashtag, hashtag, space, text, {whitespace};
inline_code = grave, text, grave, space;
paragraph_text = paragraph_starter, [paragraph_rest];
paragraph = {inline_mech_code | inline_code | paragraph_text}, [newline], {whitespace};
unordered_list = {list_item}, [newline], {whitespace};
list_item = dash, space, paragraph, [newline];
formatted_text = {paragraph_rest | newline};
code_block = grave, grave, grave, newline, formatted_text, grave, grave, grave, newline, {whitespace};
```

##MechDown
```ebnf
inline_mech_code = left_bracket, left_bracket, expression, right_bracket, right_bracket, [space];
mech_code_block = grave, grave, grave, "mech", [text], newline, block, grave, grave, grave, newline, {whitespace};
```

##Start
```ebnf
program = [title] , body;
body = {space} , section; 
section = [subtitle] , {block | code_block | mech_code_block | paragraph | unordered_list}, {space};
```
