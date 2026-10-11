"Computer Language Benchmarks Game. Local native Pharo adaptations, 2026."
Object subclass: #BenchmarksGame
    instanceVariableNames: ''
    classVariableNames: ''
    poolDictionaries: ''
    category: 'Benchmarks-Game'!
!BenchmarksGame class methodsFor: 'application'!

emit: header sequence: sequence to: out using: complement
    | bytes left right temp length |
    header ifNil: [^self].
    bytes := sequence asByteArray.
    left := 1.
    right := bytes size.
    [left <= right] whileTrue: [
        temp := complement at: (bytes at: left) + 1.
        bytes at: left put: (complement at: (bytes at: right) + 1).
        bytes at: right put: temp.
        left := left + 1.
        right := right - 1].
    out nextPutAll: header asByteArray;
        nextPut: 10.
    1 to: bytes size by: 60 do: [:i |
        length := 60 min: bytes size - i + 1.
        out next: length putAll: bytes startingAt: i;
            nextPut: 10]!
do: n
    | input out buffer header line complement |
    input := ZnCharacterReadStream on: (ZnBufferedReadStream on: Stdio stdin).
    out := ZnBufferedWriteStream on: Stdio stdout binary.
    buffer := WriteStream on: (String new: 1024).
    header := nil.
    complement := ByteArray new: 256 withAll: 0.
    'ABCDGHKMNRSTVWY' with: 'TVGHCDMKNYSABWR' do: [:a :b |
        complement at: a asInteger + 1 put: b asInteger.
        complement at: a asLowercase asInteger + 1 put: b asInteger].
    [input atEnd] whileFalse: [
        line := input nextLine.
        (line beginsWith: '>')
            ifTrue: [
                self emit: header sequence: buffer contents to: out using: complement.
                header := line.
                buffer := WriteStream on: (String new: 1024)]
            ifFalse: [(line beginsWith: ';') ifFalse: [buffer nextPutAll: line]]].
    self emit: header sequence: buffer contents to: out using: complement.
    out flush
! !
