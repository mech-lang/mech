# Mech Grammar

For now, the formal specification of the Mech grammar will be the Rust implementation. I will try to reflect that grammar in this document in [EBNF](https://en.wikipedia.org/wiki/Extended_Backus–Naur_form). Then this document can be used to generate Mech parsers in any number of languages.



##Parser
```ebnf
```



### Primitives
```ebnf
space
period
exclamation
question
comma
colon
semicolon
dash
apostrophe
left_parenthesis
right_parenthesis  
left_angle
right_angle
left_brace
right_brace
ampersand
bar
at
slash
hashtag
equal
tilde
plus
asterisk
caret
underscore
tab
```

### Values

```ebnf
--
digit excluding zero = "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" 
digit                = "0" | ?digit excluding zero?
natural number = ?digit excluding zero?, { digit } ;
```


##The Basics
```ebnf
word ***
number ***
punctuation = period | exclamation | question | comma | colon | semicolon | dash | apostrophe | left_parenthesis | right_parenthesis |  left_angle | right_angle | left_brace | right_brace;
symbol = ampersand | bar | at | slash | hashtag | equal | tilde | plus | asterisk | caret | underscore;
single_text = word | space | punctuation | symbol;
text = {word | space | number | punctuation | symbol};
paragraph_rest = {word | space | number | punctuation | symbol | quote};
paragraph_starter =  {word | number | quote | left_angle | right_angle | left_bracket | right_bracket | period | exclamation | question | comma | colon | semicolon | left_parenthesis| right_parenthesis};
identifier = word , {word | number | dash | slash};
carriage_newline = "\r\n";
true_literal = "true";
false_literal = "false";
newline = "\n"
whitespace =  {" "}, newline;
floating_point ***
quantity = number , [floating_point] , [identifier];
rational_number = quantity | number_literal, "/", quantity | number_literal
number_literal = decimal_literal | hexadecimal_literal | octal_literal | binary_literal;
decimal_literal ***
hexadecimal_literal ***
octal_literal ***
binary_literal ***
constant = string | quantity;
empty = {"_"};
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
subscript_index = "[", {subscripts}, "]";
dot_index = period, identifier;
index = dot_index | subscript_index;
data = table | identifier , {index};
```

###Tables
```ebnf
table = hashtag, identifier;
binding = identifier, ": ", empty | expression | identifier | constant, space, [comma], space;
function_binding = indentifier, colon, space, empty | expression | identifier | constant, space, [comma], space;
table_column = {space | tab} , true_literal | false_literal | empty | data | expression | rational_number | number_literal | quantity, [comma], {space| tab};
table_row = {space | tab}, {table_column}, [semicolon], [newline];
attribute = identifier, space, [comma], space;
table_header
table_define = table, space , equal, space, expression;
inline_table = "[", {binding} , "]";

```

###Statements
```ebnf
statements = table_define | variable_define | split_data | join_data | whenever_data | wait_data | until_data | set_data | add_row |       comment;

```

###Expression

####Math Expressions

####Filer Expressions

####Logic Expressions

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

```

##MechDown
```ebnf
```


##Start
```ebnf
program = [title] , body;
body = {space} , section; 
section = [subtitle] , {block | code_block | mech_code_block | paragraph | unordered_list}, {space};
```
