Rebol [Title: "CLBG revcomp baseline source proposal"]
io: do (join system/script/path %../../support/clbg_io.reb)
alphabet: io/ascii "ACGTUMRWSYKVHDBNacgtumrwsykvhdbn"
complements: io/ascii "TGCAAKYWSRMBDHVNTGCAAKYWSRMBDHVN"

emit-record: func [output [binary!] header [binary!] sequence [binary!] /local position column byte found] [
    append output header
    append output 10
    position: length? sequence
    column: 0
    while [position > 0] [
        byte: pick sequence position
        found: find alphabet byte
        if none? found [io/fail "Unsupported revcomp sequence byte"]
        append output (pick complements (index? found))
        column: column + 1
        if column = 60 [append output 10 column: 0]
        position: position - 1
    ]
    if column > 0 [append output 10]
]

input: io/read-input
output: make binary! (length? input)
header: none
sequence: make binary! 256
foreach line (io/lines input) [
    unless empty? line [
        case [
            (first line) = 62 [
                if not none? header [emit-record output header sequence]
                header: copy line
                clear sequence
            ]
            (first line) = 59 []
            not none? header [append sequence line]
        ]
    ]
]
if not none? header [emit-record output header sequence]
io/write-output output
