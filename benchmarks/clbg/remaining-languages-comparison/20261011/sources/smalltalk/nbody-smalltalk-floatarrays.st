"Computer Language Benchmarks Game. Local native Pharo adaptations, 2026."
Object subclass: #BenchmarksGame
    instanceVariableNames: ''
    classVariableNames: ''
    poolDictionaries: ''
    category: 'Benchmarks-Game'!
!BenchmarksGame class methodsFor: 'application'!

bodies
    | b mass |
    mass := 4.0d0 * Float pi * Float pi.
    b := #(
        #(0.0d0 0.0d0 0.0d0 0.0d0 0.0d0 0.0d0 1.0d0)
        #(4.84143144246472090d0 -1.16032004402742839d0 -1.03622044471123109d-1 1.66007664274403694d-3 7.69901118419740425d-3 -6.90460016972063023d-5 9.54791938424326609d-4)
        #(8.34336671824457987d0 4.12479856412430479d0 -4.03523417114321381d-1 -2.76742510726862411d-3 4.99852801234917238d-3 2.30417297573763929d-5 2.85885980666130812d-4)
        #(1.28943695621391310d1 -1.51111514016986312d1 -2.23307578892655734d-1 2.96460137564761618d-3 2.37847173959480950d-3 -2.96589568540237556d-5 4.36624404335156298d-5)
        #(1.53796971148509165d1 -2.59193146099879641d1 1.79258772950371181d-1 2.68067772490389322d-3 1.62824170038242295d-3 -9.51592254519715870d-5 5.15138902046611451d-5)
    ) collect: [:row | Float64Array withAll: row].
    b do: [:row |
        4 to: 6 do: [:k | row at: k put: (row at: k) * 365.24d0].
        row at: 7 put: (row at: 7) * mass].
    4 to: 6 do: [:k |
        b first at: k put: (b sum: [:row | (row at: k) * (row at: 7)]) negated / mass].
    ^b!
energy: bodies
    | total a b dx dy dz |
    total := 0.0d0.
    1 to: bodies size do: [:i |
        a := bodies at: i.
        total := total + (0.5d0 * (a at: 7) * ((a at: 4) squared + (a at: 5) squared + (a at: 6) squared)).
        i + 1 to: bodies size do: [:j |
            b := bodies at: j.
            dx := (a at: 1) - (b at: 1).
            dy := (a at: 2) - (b at: 2).
            dz := (a at: 3) - (b at: 3).
            total := total - ((a at: 7) * (b at: 7) / (dx * dx + (dy * dy) + (dz * dz)) sqrt)]].
    ^total!
advance: bodies
    | a b dx dy dz distance magnitude am bm |
    1 to: bodies size do: [:i |
        a := bodies at: i.
        i + 1 to: bodies size do: [:j |
            b := bodies at: j.
            dx := (a at: 1) - (b at: 1).
            dy := (a at: 2) - (b at: 2).
            dz := (a at: 3) - (b at: 3).
            distance := (dx * dx + (dy * dy) + (dz * dz)) sqrt.
            magnitude := 0.01d0 / (distance * distance * distance).
            am := (a at: 7) * magnitude.
            bm := (b at: 7) * magnitude.
            a at: 4 put: (a at: 4) - (dx * bm).
            a at: 5 put: (a at: 5) - (dy * bm).
            a at: 6 put: (a at: 6) - (dz * bm).
            b at: 4 put: (b at: 4) + (dx * am).
            b at: 5 put: (b at: 5) + (dy * am).
            b at: 6 put: (b at: 6) + (dz * am)]].
    bodies do: [:body |
        1 to: 3 do: [:k | body at: k put: (body at: k) + (0.01d0 * (body at: k + 3))]]!
do: n
    | b |
    b := self bodies.
    Stdio stdout nextPutAll: ((self energy: b) printShowingDecimalPlaces: 9);
        lf.
    n timesRepeat: [self advance: b].
    Stdio stdout nextPutAll: ((self energy: b) printShowingDecimalPlaces: 9);
        lf
! !
