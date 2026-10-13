Rebol [Title: "CLBG fannkuchredux baseline source proposal"]
io: do (join system/script/path %../../support/clbg_io.reb)
n: io/size
if any [n < 1 n > 12] [io/fail "Fannkuch size must be in 1..12"]
permutation: make block! n
rotations: make block! n
total: 1
repeat index n [append permutation (index - 1) append rotations 0 total: total * index]
rank: 0
checksum: 0
maximum-flips: 0
while [rank < total] [
    work: copy permutation
    flips: 0
    while [(first work) <> 0] [
        left: 1
        right: (first work) + 1
        while [left < right] [
            saved: pick work left
            poke work left (pick work right)
            poke work right saved
            left: left + 1
            right: right - 1
        ]
        flips: flips + 1
    ]
    checksum: either (remainder rank 2) = 0 [checksum + flips] [checksum - flips]
    maximum-flips: max maximum-flips flips
    rank: rank + 1
    if rank < total [
        width: 2
        carried: true
        while [carried] [
            if width > n [io/fail "Unexpected permutation carry"]
            saved: first permutation
            position: 1
            while [position < width] [
                poke permutation position (pick permutation (position + 1))
                position: position + 1
            ]
            poke permutation width saved
            count: (pick rotations width) + 1
            either count < width [poke rotations width count carried: false] [
                poke rotations width 0
                width: width + 1
            ]
        ]
    ]
]
io/write-output io/ascii rejoin [checksum "^/Pfannkuchen(" n ") = " maximum-flips "^/"]
