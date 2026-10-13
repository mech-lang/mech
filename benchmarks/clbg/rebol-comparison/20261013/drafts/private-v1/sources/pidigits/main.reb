Rebol [Title: "CLBG pidigits baseline source proposal"]
io: do (join system/script/path %../../support/clbg_io.reb)
big: do (join system/script/path %../../support/clbg_big.reb)
n: io/size
q: big/from-int 1
r: big/from-int 0
t: big/from-int 1
k: 1
emitted: 0
column: 0
output: make binary! 1024
while [emitted < n] [
    numerator3: big/add (big/multiply q (big/from-int 3)) r
    numerator4: big/add (big/multiply q (big/from-int 4)) r
    quotient3: big/floor-divide numerator3 t
    quotient4: big/floor-divide numerator4 t
    either (big/compare quotient3 quotient4) = 0 [
        digit: big/to-int quotient3
        if any [digit < 0 digit > 9] [io/fail "Invalid spigot digit"]
        append output (48 + digit)
        emitted: emitted + 1
        column: column + 1
        if column = 10 [
            append output io/ascii rejoin ["^-:" emitted "^/"]
            column: 0
        ]
        next-q: big/multiply q (big/from-int 10)
        next-r: big/multiply (big/subtract r (big/multiply (big/from-int digit) t)) (big/from-int 10)
        q: next-q
        r: next-r
    ] [
        factor: big/from-int ((2 * k) + 1)
        next-q: big/multiply q (big/from-int k)
        next-r: big/multiply (big/add (big/multiply (big/from-int 2) q) r) factor
        next-t: big/multiply t factor
        q: next-q
        r: next-r
        t: next-t
        k: k + 1
    ]
]
if column > 0 [
    loop (10 - column) [append output 32]
    append output io/ascii rejoin ["^-:" emitted "^/"]
]
io/write-output output
