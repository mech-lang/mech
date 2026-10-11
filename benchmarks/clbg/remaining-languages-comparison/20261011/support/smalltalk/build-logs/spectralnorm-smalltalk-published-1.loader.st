Smalltalk compiler evaluate: '"* The Computer Language Benchmarks Game
    https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
    contributed by Paolo Bonzini 
    reworked by Isaac Gouy *"'.
OldClassDefinitionBuilder buildFromAST: (OCParser parseExpression: 'Object subclass: #BenchmarksGame
   instanceVariableNames: ''''
   classVariableNames: ''''
   poolDictionaries: ''''
   category: ''Benchmarks-Game''').
(Smalltalk globals at: #BenchmarksGame) class compile: 'spectralnorm: n
   | u v vBv vv |
   u := Array new: n withAll: 1.0d0.
   10 timesRepeat:
      [v := u multiplyAtAv.
       u := v multiplyAtAv].
   vBv := 0.0d0.
   vv := 0.0d0.
   1 to: n do:
      [:i |
       vBv := vBv + ((u at: i) * (v at: i)).
       vv := vv + ((v at: i) * (v at: i))].
   ^(vBv / vv) sqrt' classified: 'private'.
(Smalltalk globals at: #BenchmarksGame) class compile: 'do: n
   Stdio stdout
      print: (self spectralnorm: n) digits: 9;
      nl' classified: 'initialize-release'.
(Smalltalk globals at: #SmallInteger) compile: 'matrixA: anInteger
   ^1.0d0 / ((self + anInteger - 2) * (self + anInteger - 1) /2  + self)' classified: 'benchmarks game'.
(Smalltalk globals at: #Array) compile: 'multiplyAtv
   | n atv sum |
   n := self size.
   atv := Array new: n.
   1 to: n do: [:i|
      sum := 0.0d0.
      1 to: n do: [:j|
         sum := sum + ((j matrixA: i) * (self at: j)) ].
      atv at: i put: sum].
   ^atv' classified: 'benchmarks game'.
(Smalltalk globals at: #Array) compile: 'multiplyAtAv
   ^(self multiplyAv) multiplyAtv' classified: 'benchmarks game'.
(Smalltalk globals at: #Array) compile: 'multiplyAv
   | n av sum |
   n := self size.
   av := Array new: n.
   1 to: n do: [:i|
      sum := 0.0d0.
      1 to: n do: [:j|
         sum := sum + ((i matrixA: j) * (self at: j)) ].
      av at: i put: sum].
   ^av' classified: 'benchmarks game'.
(Smalltalk globals at: #StdioStream) compile: 'nl
   self nextPut: Character lf' classified: 'benchmarks game'.
(Smalltalk globals at: #StdioStream) compile: 'print: number digits: decimalPlaces
   | precision rounded |
   decimalPlaces <= 0 ifTrue: [^ number rounded printString].
   precision := (10 raisedTo: decimalPlaces negated) asFloat.
   rounded := number roundTo: precision.
   self nextPutAll: 
      ((rounded asScaledDecimal: decimalPlaces) printString copyUpTo: $s)' classified: 'benchmarks game'.
