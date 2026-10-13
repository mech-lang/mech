Rebol [Title: "CLBG nbody baseline source proposal"]
io: do (join system/script/path %../../support/clbg_io.reb)
solar-mass: (4.0 * 3.141592653589793) * 3.141592653589793
days-per-year: 365.24

planet: func [x y z vx vy vz mass] [
    reduce [x y z (vx * days-per-year) (vy * days-per-year) (vz * days-per-year) (mass * solar-mass)]
]

energy: func [bodies [block!] /local total i j left right dx dy dz squared speed2] [
    total: 0.0
    i: 1
    while [i <= 5] [
        left: pick bodies i
        speed2: ((left/4 * left/4) + (left/5 * left/5)) + (left/6 * left/6)
        total: total + ((0.5 * left/7) * speed2)
        j: i + 1
        while [j <= 5] [
            right: pick bodies j
            dx: left/1 - right/1
            dy: left/2 - right/2
            dz: left/3 - right/3
            squared: ((dx * dx) + (dy * dy)) + (dz * dz)
            total: total - ((left/7 * right/7) / (square-root squared))
            j: j + 1
        ]
        i: i + 1
    ]
    total
]

advance: func [bodies [block!] /local i j left right dx dy dz squared magnitude axis displacement slot] [
    i: 1
    while [i < 5] [
        left: pick bodies i
        j: i + 1
        while [j <= 5] [
            right: pick bodies j
            dx: left/1 - right/1
            dy: left/2 - right/2
            dz: left/3 - right/3
            squared: ((dx * dx) + (dy * dy)) + (dz * dz)
            magnitude: 0.01 / (squared * (square-root squared))
            repeat axis 3 [
                displacement: pick (reduce [dx dy dz]) axis
                slot: axis + 3
                poke left slot ((pick left slot) - ((displacement * right/7) * magnitude))
                poke right slot ((pick right slot) + ((displacement * left/7) * magnitude))
            ]
            j: j + 1
        ]
        i: i + 1
    ]
    foreach left bodies [
        repeat axis 3 [poke left axis ((pick left axis) + (0.01 * (pick left (axis + 3))))]
    ]
]

bodies: reduce [
    reduce [0.0 0.0 0.0 0.0 0.0 0.0 solar-mass]
    (planet 4.84143144246472090 -1.16032004402742839 -1.03622044471123109e-1
        1.66007664274403694e-3 7.69901118419740425e-3 -6.90460016972063023e-5 9.54791938424326609e-4)
    (planet 8.34336671824457987 4.12479856412430479 -4.03523417114321381e-1
        -2.76742510726862411e-3 4.99852801234917238e-3 2.30417297573763929e-5 2.85885980666130812e-4)
    (planet 1.28943695621391310e1 -1.51111514016986312e1 -2.23307578892655734e-1
        2.96460137564761618e-3 2.37847173959480950e-3 -2.96589568540237556e-5 4.36624404335156298e-5)
    (planet 1.53796971148509165e1 -2.59193146099879641e1 1.79258772950371181e-1
        2.68067772490389322e-3 1.62824170038242295e-3 -9.51592254519715870e-5 5.15138902046611451e-5)
]
momentum: copy [0.0 0.0 0.0]
foreach body bodies [
    repeat axis 3 [poke momentum axis ((pick momentum axis) + ((pick body (axis + 3)) * body/7))]
]
sun: first bodies
repeat axis 3 [poke sun (axis + 3) ((negate pick momentum axis) / solar-mass)]
output: io/ascii rejoin [(io/fixed (energy bodies) 9) "^/"]
n: io/size
loop n [advance bodies]
append output io/ascii rejoin [(io/fixed (energy bodies) 9) "^/"]
io/write-output output
