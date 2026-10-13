Rebol [Title: "CLBG knucleotide baseline source proposal"]
io: do (join system/script/path %../../support/clbg_io.reb)
widths: [1 2 3 4 6 12 18]
letters: io/ascii "ACGT"

base-code: func [byte [integer!]] [
    switch byte [65 [0] 67 [1] 71 [2] 84 [3]]
]

encode-word: func [word [binary!] /local key byte code] [
    key: 0
    foreach byte word [
        code: base-code byte
        if none? code [io/fail "Non-ACGT K sequence byte"]
        key: (key * 4) + code
    ]
    key
]

decode-word: func [key [integer!] width [integer!] /local result position digit] [
    result: make binary! width
    loop width [append result 65]
    position: width
    while [position > 0] [
        digit: remainder key 4
        poke result position (pick letters (digit + 1))
        key: key // 4
        position: position - 1
    ]
    to string! result
]

read-three: func [input [binary!] /local sequence selected line header token token-end byte] [
    sequence: make binary! 1024
    selected: false
    foreach line (io/lines input) [
        unless empty? line [
            either (first line) = 62 [
                if selected [break]
                header: copy next line
                token-end: 1
                while [all [token-end <= length? header not find [9 32] (pick header token-end)]] [
                    token-end: token-end + 1
                ]
                token: copy/part header (token-end - 1)
                selected: token = io/ascii "THREE"
            ] [
                if all [selected (first line) <> 59] [
                    foreach byte line [append sequence (either all [byte >= 97 byte <= 122] [byte - 32] [byte])]
                ]
            ]
        ]
    ]
    sequence
]

frequency: func [output [binary!] table [map!] width [integer!] denominator [integer!]
    /local entries key count entry] [
    entries: make block! 16
    foreach [key count] table [append/only entries (reduce [(decode-word key width) count])]
    sort/compare entries func [left right] [either left/2 = right/2 [left/1 < right/1] [left/2 > right/2]]
    foreach entry entries [
        append output io/ascii rejoin [entry/1 " " (io/fixed ((100.0 * entry/2) / denominator) 3) "^/"]
    ]
    append output 10
]

sequence: read-three io/read-input
tables: make block! 7
foreach width widths [append/only tables (make map! [])]
repeat map-index 7 [
    width: pick widths map-index
    table: pick tables map-index
    position: 1
    last-start: ((length? sequence) - width) + 1
    while [position <= last-start] [
        key: encode-word (copy/part (at sequence position) width)
        count: select table key
        put table key (either none? count [1] [count + 1])
        position: position + 1
    ]
]
output: make binary! 1024
frequency output (pick tables 1) 1 (length? sequence)
frequency output (pick tables 2) 2 ((length? sequence) - 1)
foreach query ["GGT" "GGTA" "GGTATT" "GGTATTTTAATT" "GGTATTTTAATTTATAGT"] [
    width: length? query
    map-index: index? find widths width
    count: select (pick tables map-index) (encode-word io/ascii query)
    append output io/ascii rejoin [either none? count [0] [count] "^-" query "^/"]
]
io/write-output output
