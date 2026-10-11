"Computer Language Benchmarks Game. Local native Pharo adaptations, 2026."
Object subclass: #BenchmarksGame
    instanceVariableNames: ''
    classVariableNames: ''
    poolDictionaries: ''
    category: 'Benchmarks-Game'!
!BenchmarksGame class methodsFor: 'application'!

do: n
    | q r t k digit emitted out |
    q := 1.
    r := 0.
    t := 1.
    k := 0.
    emitted := 0.
    out := Stdio stdout.
    [emitted < n] whileTrue: [
        k := k + 1.
        r := (r + (q * 2)) * (k * 2 + 1).
        q := q * k.
        t := t * (k * 2 + 1).
        q <= r ifTrue: [
            digit := (q * 3 + r) // t.
            digit = ((q * 4 + r) // t) ifTrue: [
                out nextPut: (Character digitValue: digit).
                emitted := emitted + 1.
                r := (r - (t * digit)) * 10.
                q := q * 10.
                (emitted \\ 10 = 0 or: [emitted = n]) ifTrue: [
                    emitted \\ 10 = 0 ifFalse: [
                        1 to: 10 - (emitted \\ 10) do: [:unused | out nextPut: Character space]].
                    out nextPut: Character tab;
                        nextPut: $:;
                        print: emitted;
                        lf]]]
    ]
! !
