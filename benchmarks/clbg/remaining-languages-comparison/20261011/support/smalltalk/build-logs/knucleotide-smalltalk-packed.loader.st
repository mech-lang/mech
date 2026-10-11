OldClassDefinitionBuilder buildFromAST: (OCParser parseExpression: '"Computer Language Benchmarks Game. Local native Pharo adaptations, 2026."
Object subclass: #BenchmarksGame
    instanceVariableNames: ''''
    classVariableNames: ''''
    poolDictionaries: ''''
    category: ''Benchmarks-Game''').
(Smalltalk globals at: #BenchmarksGame) class compile: 'readThree
    | input buffer active line |
    input := ZnCharacterReadStream on: (ZnBufferedReadStream on: Stdio stdin).
    buffer := WriteStream on: (String new: 1024).
    active := false.
    [input atEnd] whileFalse: [
        line := input nextLine.
        (line beginsWith: ''>'')
            ifTrue: [
                active ifTrue: [^buffer contents asUppercase].
                active := line beginsWith: ''>THREE'']
            ifFalse: [
                (active and: [line isNotEmpty and: [line first ~= $;]]) ifTrue: [buffer nextPutAll: line]]].
    ^buffer contents asUppercase' classified: 'application'.
(Smalltalk globals at: #BenchmarksGame) class compile: 'countsIn: sequence length: length
    | counts mask code association |
    counts := Dictionary new.
    mask := (1 bitShift: length * 2) - 1.
    code := 0.
    1 to: sequence size do: [:i |
        code := ((code bitShift: 2) bitOr: (sequence at: i)) bitAnd: mask.
        i >= length ifTrue: [
            association := counts associationAt: code ifAbsent: [nil].
            association
                ifNil: [counts at: code put: 1]
                ifNotNil: [association value: association value + 1]]].
    ^counts' classified: 'application'.
(Smalltalk globals at: #BenchmarksGame) class compile: 'keyOf: string
    ^string inject: 0 into: [:key :char | (key bitShift: 2) bitOr: (''ACGT'' indexOf: char) - 1]' classified: 'application'.
(Smalltalk globals at: #BenchmarksGame) class compile: 'decode: key length: length
    | text remaining |
    text := String new: length.
    remaining := key.
    length to: 1 by: -1 do: [:i |
        text at: i put: (''ACGT'' at: (remaining bitAnd: 3) + 1).
        remaining := remaining bitShift: -2].
    ^text' classified: 'application'.
(Smalltalk globals at: #BenchmarksGame) class compile: 'do: n
    | string sequence tables out rows total |
    string := self readThree.
    sequence := ByteArray new: string size.
    1 to: string size do: [:i |
        (''ACGT'' includes: (string at: i)) ifFalse: [self error: ''Expected A, C, G or T DNA''].
        sequence at: i put: (''ACGT'' indexOf: (string at: i)) - 1].
    tables := Dictionary new.
    #(1 2 3 4 6 12 18) do: [:length |
        tables at: length put: (self countsIn: sequence length: length)].
    out := Stdio stdout.
    #(1 2) do: [:length |
        rows := (tables at: length) associations asSortedCollection: [:a :b |
            a value = b value ifTrue: [a key < b key] ifFalse: [a value > b value]].
        total := sequence size - length + 1.
        rows do: [:row |
            out nextPutAll: (self decode: row key length: length);
                nextPut: Character space;
                nextPutAll: (row value * 100.0d0 / total printShowingDecimalPlaces: 3);
                lf].
        out lf].
    #(''GGT'' ''GGTA'' ''GGTATT'' ''GGTATTTTAATT'' ''GGTATTTTAATTTATAGT'') do: [:query |
        out print: ((tables at: query size) at: (self keyOf: query) ifAbsent: [0]);
            nextPut: Character tab;
            nextPutAll: query;
            lf]' classified: 'application'.
