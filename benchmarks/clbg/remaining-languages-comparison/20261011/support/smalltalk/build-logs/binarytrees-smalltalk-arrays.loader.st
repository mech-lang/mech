OldClassDefinitionBuilder buildFromAST: (OCParser parseExpression: '"Computer Language Benchmarks Game. Local native Pharo adaptations, 2026."
Object subclass: #BenchmarksGame
    instanceVariableNames: ''''
    classVariableNames: ''''
    poolDictionaries: ''''
    category: ''Benchmarks-Game''').
(Smalltalk globals at: #BenchmarksGame) class compile: 'tree: depth
    ^depth = 0
        ifTrue: [Array with: nil with: nil]
        ifFalse: [Array with: (self tree: depth - 1) with: (self tree: depth - 1)]' classified: 'application'.
(Smalltalk globals at: #BenchmarksGame) class compile: 'check: tree
    ^tree first isNil
        ifTrue: [1]
        ifFalse: [1 + (self check: tree first) + (self check: tree second)]' classified: 'application'.
(Smalltalk globals at: #BenchmarksGame) class compile: 'do: n
    | maximum long iterations check out |
    maximum := 6 max: n.
    out := Stdio stdout.
    check := self check: (self tree: maximum + 1).
    out nextPutAll: ''stretch tree of depth '';
        print: maximum + 1;
        nextPut: Character tab;
        nextPutAll: '' check: '';
        print: check;
        lf.
    long := self tree: maximum.
    4 to: maximum by: 2 do: [:depth |
        iterations := 1 bitShift: maximum - depth + 4.
        check := 0.
        iterations timesRepeat: [check := check + (self check: (self tree: depth))].
        out print: iterations;
            nextPutAll: String tab, '' trees of depth '';
            print: depth;
            nextPutAll: String tab, '' check: '';
            print: check;
            lf].
    out nextPutAll: ''long lived tree of depth '';
        print: maximum;
        nextPutAll: String tab, '' check: '';
        print: (self check: long);
        lf' classified: 'application'.
