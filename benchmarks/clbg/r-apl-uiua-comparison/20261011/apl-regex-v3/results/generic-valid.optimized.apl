⎕IO←0
OUT←'w'⎕FIO[3]'/dev/fd/3'
⊣'.build/apl-support/libregex-count-v3-apl.dylib'⎕FX'Count'
⊣'.build/apl-support/libregex-apl.dylib'⎕FX'Substitute'
Result←'x' Count[0 1] 'abc'
⊣(('%d',⎕UCS 10)(0=Result))⎕FIO[22]OUT
Result←'aa' Count[0 1] 'aaaaa'
⊣(('%d',⎕UCS 10)(2=Result))⎕FIO[22]OUT
Result←'a|aa' Count[0 1] 'aaa'
⊣(('%d',⎕UCS 10)(3=Result))⎕FIO[22]OUT
Result←'a\+' Count[0 1] 'a+ a+ aa'
⊣(('%d',⎕UCS 10)(2=Result))⎕FIO[22]OUT
Result←'(a)(b)' Count[0 1] 'ababab'
⊣(('%d',⎕UCS 10)(3=Result))⎕FIO[22]OUT
Result←'(.)\1' Count[0 1] 'aabbcc'
⊣(('%d',⎕UCS 10)(3=Result))⎕FIO[22]OUT
Result←'' Count[0 1] 'ab'
⊣(('%d',⎕UCS 10)(3=Result))⎕FIO[22]OUT
Result←'' Count[0 1] ''
⊣(('%d',⎕UCS 10)(1=Result))⎕FIO[22]OUT
Result←'a' Count[0 1] ''
⊣(('%d',⎕UCS 10)(0=Result))⎕FIO[22]OUT
Result←'(?=a)' Count[0 1] 'aaa'
⊣(('%d',⎕UCS 10)(3=Result))⎕FIO[22]OUT
Result←'a*?' Count[0 1] 'aa'
⊣(('%d',⎕UCS 10)(5=Result))⎕FIO[22]OUT
Result←'(?:|a)' Count[0 1] 'a'
⊣(('%d',⎕UCS 10)(3=Result))⎕FIO[22]OUT
Result←'a*' Count[0 1] 'aa'
⊣(('%d',⎕UCS 10)(2=Result))⎕FIO[22]OUT
Result←'λ+' Count[0 1] 'xλλyλ'
⊣(('%d',⎕UCS 10)(2=Result))⎕FIO[22]OUT
Result←'.' Count[0 1] '💡a'
⊣(('%d',⎕UCS 10)(2=Result))⎕FIO[22]OUT
Result←'\p{L}+' Count[0 1] 'λΩ 123 A'
⊣(('%d',⎕UCS 10)(2=Result))⎕FIO[22]OUT
Result←'a' Count[8 1] 'AaA'
⊣(('%d',⎕UCS 10)(3=Result))⎕FIO[22]OUT
Result←'^a' Count[1032 1] (⎕UCS 97 10 65 10 97)
⊣(('%d',⎕UCS 10)(3=Result))⎕FIO[22]OUT
Result←'\w+' Count[655360 1] 'λΩ ! A'
⊣(('%d',⎕UCS 10)(2=Result))⎕FIO[22]OUT
Result←'(?i)a' Count[0 1] 'AaA'
⊣(('%d',⎕UCS 10)(3=Result))⎕FIO[22]OUT
Result←'(*CRLF)' Count[0 1] (⎕UCS 13 10)
⊣(('%d',⎕UCS 10)(2=Result))⎕FIO[22]OUT
Result←'\x00' Count[1] ('a',(⎕UCS 0),'a')
⊣(('%d',⎕UCS 10)(1=Result))⎕FIO[22]OUT
Result←('(a)' '$1') Substitute[1] 'ababa'
⊣(('%d',⎕UCS 10)('$1b$1b$1'≡Result))⎕FIO[22]OUT
Result←('x+' '') Substitute[1] 'aaxxbbb'
⊣(('%d',⎕UCS 10)('aabbb'≡Result))⎕FIO[22]OUT
Result←('(?=b)' 'X') Substitute[1] 'abc'
⊣(('%d',⎕UCS 10)('aXbc'≡Result))⎕FIO[22]OUT
Result←('' 'X') Substitute[1] 'ab'
⊣(('%d',⎕UCS 10)('XaXbX'≡Result))⎕FIO[22]OUT
Result←('a' 'b') Substitute[1] ''
⊣(('%d',⎕UCS 10)(''≡Result))⎕FIO[22]OUT
Result←('.+' '') Substitute[1] 'all'
⊣(('%d',⎕UCS 10)(''≡Result))⎕FIO[22]OUT
Result←('a' 'abcdefghijklmnop') Substitute[1] 'aaaaa'
⊣(('%d',⎕UCS 10)('abcdefghijklmnopabcdefghijklmnopabcdefghijklmnopabcdefghijklmnopabcdefghijklmnop'≡Result))⎕FIO[22]OUT
Result←('λ+' 'Ω') Substitute[1] 'xλλyλ'
⊣(('%d',⎕UCS 10)('xΩyΩ'≡Result))⎕FIO[22]OUT
Result←('aa' 'X') Substitute[1] 'aaaaa'
⊣(('%d',⎕UCS 10)('XXa'≡Result))⎕FIO[22]OUT
Result←('a\+' 'X') Substitute[1] 'a+ a+ aa'
⊣(('%d',⎕UCS 10)('X X aa'≡Result))⎕FIO[22]OUT
Result←('a|bb' 'X') Substitute[1] 'abbabba'
⊣(('%d',⎕UCS 10)('XXXXX'≡Result))⎕FIO[22]OUT
⊣⎕FIO[4]OUT
)OFF
