# Mech Grammar

For now, the formal specification of the Mech grammar will be the Rust implementation. I will try to reflect that grammar in this document in [EBNF](https://en.wikipedia.org/wiki/Extended_Backus–Naur_form). Then this document can be used to generate Mech parsers in any number of languages.

## Primitives

```ebnf
newline = "\n"
whitespace =  " " | "\t" | "," | newline
```

## Values

```ebnf
digit excluding zero = "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" 
digit                = "0" | digit excluding zero
natural number = digit excluding zero, { digit } ;
```

## Blocks

```ebnf
block = {constraint}
constraint = space, space, statement | expression
statement = 
expression = 
```