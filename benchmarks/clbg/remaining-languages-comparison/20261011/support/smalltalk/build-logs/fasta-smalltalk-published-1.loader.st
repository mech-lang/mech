Smalltalk compiler evaluate: '"* The Computer Language Benchmarks Game
    https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
    contributed by Isaac Gouy
    modified by Eliot Miranda *"'.
OldClassDefinitionBuilder buildFromAST: (OCParser parseExpression: 'Object subclass: #BenchmarksGame
   instanceVariableNames: ''''
   classVariableNames: ''''
   poolDictionaries: ''''
   category: ''Benchmarks-Game''').
OldClassDefinitionBuilder buildFromAST: (OCParser parseExpression: 'Object subclass: #RandomNumber
   instanceVariableNames: ''seed scale''
   classVariableNames: ''FModulus Increment Modulus Multiplier''
   poolDictionaries: ''''
   category: ''Benchmarks-Game''').
OldClassDefinitionBuilder buildFromAST: (OCParser parseExpression: 'ReadStream subclass: #RepeatStream
   instanceVariableNames: ''repeatPtr repeatLimit''
   classVariableNames: ''''
   poolDictionaries: ''''
   category: ''Benchmarks-Game''').
OldClassDefinitionBuilder buildFromAST: (OCParser parseExpression: 'RepeatStream subclass: #RandomStream
   instanceVariableNames: ''random percentages''
   classVariableNames: ''''
   poolDictionaries: ''''
   category: ''Benchmarks-Game''').
(Smalltalk globals at: #BenchmarksGame) class compile: 'fasta: n to: out
   | r lineLength |
   lineLength := 60.
   self
      writeFasta: ''ONE Homo sapiens alu''
      from:
         ( RepeatStream
            to: n*2
            on:''GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG'',
               ''GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA'',
               ''CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT'',
               ''ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA'',
               ''GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG'',
               ''AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC'',
               ''AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA'' )
      to: out
      lineLength: lineLength.

   r := RandomNumber to: 1. "Shared random sequence"

   self
      writeFasta: ''TWO IUB ambiguity codes''
      from:
         (( RandomStream
            to: n*3
            on: #(   #($a 0.27d0)
                  #($c 0.12d0)
                  #($g 0.12d0)
                  #($t 0.27d0)

                  #($B 0.02d0)
                  #($D 0.02d0)
                  #($H 0.02d0)
                  #($K 0.02d0)
                  #($M 0.02d0)
                  #($N 0.02d0)
                  #($R 0.02d0)
                  #($S 0.02d0)
                  #($V 0.02d0)
                  #($W 0.02d0)
                  #($Y 0.02d0)))
         random: r;
         yourself)
      to: out
      lineLength: lineLength.

   self
      writeFasta: ''THREE Homo sapiens frequency''
      from:
         (( RandomStream
            to: n*5
            on: #(   #($a 0.3029549426680d0)
                  #($c 0.1979883004921d0)
                  #($g 0.1975473066391d0)
                  #($t 0.3015094502008d0)))
            random: r;
            yourself)
      to: out
      lineLength: lineLength.

   out flush.' classified: 'private'.
(Smalltalk globals at: #BenchmarksGame) class compile: 'writeFasta: aString from: inStream to: outStream lineLength: lineLength
   | i |
   outStream nextPut: $>; nextPutAll: aString; nl.
   i := 0.
   [inStream atEnd] whileFalse:
      [i == lineLength ifTrue: [outStream nl. i := 0].
      outStream nextPut: inStream next.
      i := i + 1].
   outStream nl' classified: 'private'.
(Smalltalk globals at: #BenchmarksGame) class compile: 'do: n
   self fasta: n to: Stdio stdout' classified: 'initialize-release'.
(Smalltalk globals at: #RandomNumber) class compile: 'initialize
   FModulus := 139968.0d0.
   Increment := 29573.
   Modulus := 139968.
   Multiplier := 3877.' classified: 'class initialization'.
(Smalltalk globals at: #RandomNumber) class compile: 'to: anInteger
   ^self basicNew to: anInteger' classified: 'initialize-release'.
(Smalltalk globals at: #RandomNumber) compile: 'to: anInteger
   seed := 42.
   scale := anInteger' classified: 'private'.
(Smalltalk globals at: #RandomNumber) compile: 'next
   seed := (seed * Multiplier + Increment) \\ Modulus.
   ^(seed * scale) / FModulus' classified: 'accessing'.
(Smalltalk globals at: #RepeatStream) class compile: 'to: anInteger on: aCollection
   ^(super on: aCollection) to: anInteger' classified: 'instance creation'.
(Smalltalk globals at: #RepeatStream) compile: 'next
   position >= readLimit ifTrue: [ self position: 0 ].
   repeatPtr := repeatPtr + 1.
   ^collection at: (position := position + 1)' classified: 'accessing'.
(Smalltalk globals at: #RepeatStream) compile: 'atEnd
   ^repeatPtr >= repeatLimit' classified: 'testing'.
(Smalltalk globals at: #RepeatStream) compile: 'to: anInteger
   repeatPtr := 0.
   repeatLimit := anInteger' classified: 'initialize-release'.
(Smalltalk globals at: #RandomStream) compile: 'random: aRandomNumber
"* Share the random number generator so we can get the expected results. *"
   random := aRandomNumber' classified: 'accessing'.
(Smalltalk globals at: #RandomStream) compile: 'next
   | r |
   r := random next.
   repeatPtr := repeatPtr + 1.
   1 to: percentages size do: [:i|
      (r < (percentages at: i)) ifTrue: [^collection at: i]]' classified: 'accessing'.
(Smalltalk globals at: #RandomStream) compile: 'on: aCollection
   | size cp |
   repeatPtr := 0.
   random := RandomNumber to: 1.0d0.
   size := aCollection size.
   percentages := Array new: size.
   collection := Array new: size.
   cp := 0.0d0.
   1 to: size do: [:i|
      collection at: i put: (aCollection at: i) first.
      percentages at: i put: (cp := cp + (aCollection at: i) last).
   ]' classified: 'initialize-release'.
(Smalltalk globals at: #StdioStream) compile: 'nl
   self nextPut: Character lf' classified: 'benchmarks game'.
Smalltalk compiler evaluate: 'RandomNumber initialize'.
