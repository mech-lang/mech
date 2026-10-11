⎕IO←0
OUT←'w'⎕FIO[3]'/dev/fd/3'
⊣'.build/apl-support/libregex-apl.dylib'⎕FX'Substitute'
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
⊣⎕FIO[4]OUT
)OFF
