Rebol [Title: "CLBG spectralnorm baseline source proposal"]
io: do (join system/script/path %../../support/clbg_io.reb)

coefficient: func [i [integer!] j [integer!] /local diagonal denominator] [
    diagonal: i + j
    denominator: (((diagonal * (diagonal + 1)) // 2) + i) + 1
    1.0 / denominator
]

product: func [input [block!] transposed [logic!] /local result n i j sum factor] [
    n: length? input
    result: make block! n
    i: 0
    while [i < n] [
        sum: 0.0
        j: 0
        while [j < n] [
            factor: either transposed [coefficient j i] [coefficient i j]
            sum: sum + (factor * (pick input (j + 1)))
            j: j + 1
        ]
        append result sum
        i: i + 1
    ]
    result
]

ata: func [input [block!]] [product (product input false) true]
n: io/size
if n = 0 [io/fail "Spectralnorm size must be positive"]
u: make block! n
loop n [append u 1.0]
v: make block! n
loop 10 [v: ata u u: ata v]
uv: 0.0
vv: 0.0
repeat index n [
    uv: uv + ((pick u index) * (pick v index))
    vv: vv + ((pick v index) * (pick v index))
]
io/write-output io/ascii rejoin [(io/fixed (square-root (uv / vv)) 9) "^/"]
