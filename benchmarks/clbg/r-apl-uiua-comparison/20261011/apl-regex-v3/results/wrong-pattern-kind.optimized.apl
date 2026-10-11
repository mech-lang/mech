⎕IO←0
OUT←'w'⎕FIO[3]'/dev/fd/3'
⊣'.build/apl-support/libregex-count-v3-apl.dylib'⎕FX'Count'
⊣'.build/apl-support/libregex-apl.dylib'⎕FX'Substitute'
Result←3 Count 'abc'
)OFF
