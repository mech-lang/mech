Rebol [Title: "CLBG fasta baseline source proposal"]
io: do (join system/script/path %../../support/clbg_io.reb)
alu: io/ascii {GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCAGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCCAGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA}
iub: [97 0.27 99 0.12 103 0.12 116 0.27 66 0.02 68 0.02 72 0.02 75 0.02
    77 0.02 78 0.02 82 0.02 83 0.02 86 0.02 87 0.02 89 0.02]
homo: [97 0.3029549426680 99 0.1979883004921 103 0.1975473066391 116 0.3015094502008]

cumulative: func [probabilities [block!] /local result total code probability] [
    result: make block! (length? probabilities)
    total: 0.0
    foreach [code probability] probabilities [total: total + probability append result code append result total]
    result
]

repeat-sequence: func [output [binary!] count [integer!] /local position column] [
    position: 1
    column: 0
    loop count [
        append output (pick alu position)
        position: position + 1
        if position > length? alu [position: 1]
        column: column + 1
        if column = 60 [append output 10 column: 0]
    ]
    if column > 0 [append output 10]
]

random-sequence: func [output [binary!] count [integer!] distribution [block!] seed [integer!]
    /local draw position code column] [
    column: 0
    loop count [
        seed: remainder ((seed * 3877) + 29573) 139968
        draw: seed / 139968.0
        position: 2
        while [all [position < length? distribution draw >= (pick distribution position)]] [
            position: position + 2
        ]
        code: pick distribution (position - 1)
        append output code
        column: column + 1
        if column = 60 [append output 10 column: 0]
    ]
    if column > 0 [append output 10]
    seed
]

n: io/size
output: make binary! 1024
append output io/ascii ">ONE Homo sapiens alu^/"
repeat-sequence output (2 * n)
append output io/ascii ">TWO IUB ambiguity codes^/"
seed: random-sequence output (3 * n) (cumulative iub) 42
append output io/ascii ">THREE Homo sapiens frequency^/"
seed: random-sequence output (5 * n) (cumulative homo) seed
io/write-output output
