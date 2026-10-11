"Computer Language Benchmarks Game. Local native Pharo adaptations, 2026."
Object subclass: #BenchmarksGame
    instanceVariableNames: ''
    classVariableNames: ''
    poolDictionaries: ''
    category: 'Benchmarks-Game'!
!BenchmarksGame class methodsFor: 'application'!

readInput
    | input output block |
    input := Stdio stdin binary.
    output := WriteStream on: (ByteArray new: 65536).
    [(block := input next: 65536) isNotEmpty] whileTrue: [output nextPutAll: block].
    ^output contents!
patterns
    ^#('agggtaaa|tttaccct'
        '[cgt]gggtaaa|tttaccc[acg]'
        'a[act]ggtaaa|tttacc[agt]t'
        'ag[act]gtaaa|tttac[agt]ct'
        'agg[act]taaa|ttta[agt]cct'
        'aggg[acg]aaa|ttt[cgt]ccct'
        'agggt[cgt]aa|tt[acg]accct'
        'agggta[cgt]a|t[acg]taccct'
        'agggtaa[cgt]|[acg]ttaccct')!
substitutions
    ^#(#('tHa[Nt]' '<4>')
        #('aND|caN|Ha[DS]|WaS' '<3>')
        #('a[NSt]|BY' '<2>')
        #('<[^>]*>' '|')
        #('\|[^|][^|]*\|' '-'))!

do: n
    | raw sequence originalSize count out |
    raw := self readInput asString.
    sequence := raw copyWithRegex: ('>[^', String lf, ']*', String lf, '|', String lf) matchesReplacedWith: ''.
    originalSize := sequence size.
    out := Stdio stdout.
    self patterns do: [:pattern |
        count := 0.
        pattern asRegex matchesIn: sequence do: [:match | count := count + 1].
        out nextPutAll: pattern;
            nextPut: Character space;
            print: count;
            lf].
    self substitutions do: [:substitution |
        sequence := sequence copyWithRegex: substitution first matchesReplacedWith: substitution second].
    out lf;
        print: raw size;
        lf;
        print: originalSize;
        lf;
        print: sequence size;
        lf
! !
