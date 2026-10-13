Rebol [Title: "CLBG mandelbrot baseline source proposal"]
io: do (join system/script/path %../../support/clbg_io.reb)
n: io/size
output: make binary! 1024
append output io/ascii rejoin ["P4^/" n " " n "^/"]
y: 0
while [y < n] [
    ci: ((2.0 * y) / n) - 1.0
    x: 0
    byte: 0
    bits: 0
    while [x < n] [
        cr: ((2.0 * x) / n) - 1.5
        zr: 0.0
        zi: 0.0
        zr2: 0.0
        zi2: 0.0
        iteration: 0
        while [all [iteration < 50 (zr2 + zi2) <= 4.0]] [
            zi: ((2.0 * zr) * zi) + ci
            zr: (zr2 - zi2) + cr
            zr2: zr * zr
            zi2: zi * zi
            iteration: iteration + 1
        ]
        byte: (byte * 2) + (either (zr2 + zi2) <= 4.0 [1] [0])
        bits: bits + 1
        if bits = 8 [append output byte byte: 0 bits: 0]
        x: x + 1
    ]
    if bits > 0 [
        loop (8 - bits) [byte: byte * 2]
        append output byte
    ]
    y: y + 1
]
io/write-output output
