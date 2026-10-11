"Computer Language Benchmarks Game. Local native Pharo adaptations, 2026."
Object subclass: #BenchmarksGame
    instanceVariableNames: ''
    classVariableNames: ''
    poolDictionaries: ''
    category: 'Benchmarks-Game'!
!BenchmarksGame class methodsFor: 'application'!

multiply: vector transpose: transposed
    | result n sum denominator step |
    n := vector size.
    result := Float64Array new: n.
    0 to: n - 1 do: [:i |
        denominator := i * (i + 1) // 2 + (transposed ifTrue: [1] ifFalse: [i + 1]).
        step := i + (transposed ifTrue: [2] ifFalse: [1]).
        sum := 0.0d0.
        1 to: n do: [:j |
            sum := sum + ((vector at: j) / denominator).
            denominator := denominator + step.
            step := step + 1].
        result at: i + 1 put: sum].
    ^result!
ata: vector
    ^self multiply: (self multiply: vector transpose: false) transpose: true!
do: n
    | u v uv vv |
    u := Float64Array new: n withAll: 1.0d0.
    10 timesRepeat: [
        v := self ata: u.
        u := self ata: v].
    uv := 0.0d0.
    vv := 0.0d0.
    1 to: n do: [:i |
        uv := uv + ((u at: i) * (v at: i)).
        vv := vv + ((v at: i) * (v at: i))].
    Stdio stdout nextPutAll: ((uv / vv) sqrt printShowingDecimalPlaces: 9);
        lf
! !
