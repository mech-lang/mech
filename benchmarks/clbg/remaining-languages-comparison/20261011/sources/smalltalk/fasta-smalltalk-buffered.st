"Computer Language Benchmarks Game. Local native Pharo adaptations, 2026."
Object subclass: #BenchmarksGame
    instanceVariableNames: ''
    classVariableNames: ''
    poolDictionaries: ''
    category: 'Benchmarks-Game'!
!BenchmarksGame class methodsFor: 'application'!

alu
    ^'GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG',
      'GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA',
      'CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT',
      'ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA',
      'GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG',
      'AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC',
      'AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA'!
random: length alphabet: alphabet probabilities: probabilities seed: seed to: out
    | cumulative total bytes buffer amount r lo hi mid remaining state |
    remaining := length.
    state := seed.
    total := 0.0d0.
    cumulative := probabilities collect: [:p | total := total + p].
    bytes := alphabet asByteArray.
    buffer := ByteArray new: 61.
    [remaining > 0] whileTrue: [
        amount := remaining min: 60.
        1 to: amount do: [:i |
            state := (state * 3877 + 29573) \\ 139968.
            r := state asFloat / 139968.0d0.
            lo := 1.
            hi := cumulative size.
            [lo < hi] whileTrue: [
                mid := (lo + hi) // 2.
                r < (cumulative at: mid)
                    ifTrue: [hi := mid]
                    ifFalse: [lo := mid + 1]].
            buffer at: i put: (bytes at: lo)].
        buffer at: amount + 1 put: 10.
        out next: amount + 1 putAll: buffer startingAt: 1.
        remaining := remaining - amount].
    ^state!
do: n
    | out alu length index buffer amount seed |
    out := ZnBufferedWriteStream on: Stdio stdout binary.
    out nextPutAll: '>ONE Homo sapiens alu' asByteArray;
        nextPut: 10.
    alu := self alu asByteArray.
    length := n * 2.
    index := 1.
    buffer := ByteArray new: 61.
    [length > 0] whileTrue: [
        amount := length min: 60.
        1 to: amount do: [:i |
            buffer at: i put: (alu at: index).
            index := index \\ alu size + 1].
        buffer at: amount + 1 put: 10.
        out next: amount + 1 putAll: buffer startingAt: 1.
        length := length - amount].
    out nextPutAll: '>TWO IUB ambiguity codes' asByteArray;
        nextPut: 10.
    seed := self random: n * 3 alphabet: 'acgtBDHKMNRSVWY'
        probabilities: #(0.27d0 0.12d0 0.12d0 0.27d0 0.02d0 0.02d0 0.02d0 0.02d0 0.02d0 0.02d0 0.02d0 0.02d0 0.02d0 0.02d0 0.02d0)
        seed: 42 to: out.
    out nextPutAll: '>THREE Homo sapiens frequency' asByteArray;
        nextPut: 10.
    self random: n * 5 alphabet: 'acgt'
        probabilities: #(0.3029549426680d0 0.1979883004921d0 0.1975473066391d0 0.3015094502008d0)
        seed: seed to: out.
    out flush
! !
