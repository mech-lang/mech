Smalltalk compiler evaluate: '"* The Computer Language Benchmarks Game
    https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
    contributed by Paolo Bonzini 
    modified by Andres Valloud *"'.
OldClassDefinitionBuilder buildFromAST: (OCParser parseExpression: 'Object subclass: #BenchmarksGame
   instanceVariableNames: ''''
   classVariableNames: ''''
   poolDictionaries: ''''
   category: ''Benchmarks-Game''').
OldClassDefinitionBuilder buildFromAST: (OCParser parseExpression: 'Object subclass: #PiDigitSpigot
   instanceVariableNames: ''numer accum denom k''
   classVariableNames: ''''
   poolDictionaries: ''''
   category: ''Benchmarks-Game''').
(Smalltalk globals at: #BenchmarksGame) class compile: 'pidigitsTo: v width: width to: output
   | n i pidigits |
   n := v.
   i := 0.
   pidigits := PiDigitSpigot new.
   [n > 0] whileTrue:
      [n < width
         ifTrue:
            [n timesRepeat: [output nextPut: (Character digitValue: pidigits next)].
            n + 1 to: width do: [:each | output nextPut: Character space].
            i := i + n]
         ifFalse:
            [width timesRepeat: [output nextPut: (Character digitValue: pidigits next)].
            i := i + width].

      output tab; nextPut: $:; print: i; nl.

      n := n - width]' classified: 'private'.
(Smalltalk globals at: #BenchmarksGame) class compile: 'do: n
   self pidigitsTo: n width: 10 to: Stdio stdout' classified: 'initialize-release'.
(Smalltalk globals at: #PiDigitSpigot) class compile: 'new
   ^super basicNew initialize' classified: 'instance creation'.
(Smalltalk globals at: #PiDigitSpigot) compile: 'initialize
    numer := denom := 1.
    k := accum := 0.' classified: 'private'.
(Smalltalk globals at: #PiDigitSpigot) compile: 'extract
    | tmp digit |
    numer > accum ifTrue: [^nil].
    tmp := numer + numer + numer + accum.
    digit := tmp // denom.
    digit = ((tmp + numer) // denom) ifFalse: [^nil].
    ^digit' classified: 'private'.
(Smalltalk globals at: #PiDigitSpigot) compile: 'eliminate: digit
    accum := accum - (denom * digit).
    accum := accum * 10.
    numer := numer * 10' classified: 'private'.
(Smalltalk globals at: #PiDigitSpigot) compile: 'step
    | y2 |
    k := k + 1.
    y2 := k * 2 + 1.
    accum := (numer + numer + accum) * y2.
    numer := numer * k.
    denom := denom * y2.' classified: 'private'.
(Smalltalk globals at: #PiDigitSpigot) compile: 'atEnd
    ^false' classified: 'stream'.
(Smalltalk globals at: #PiDigitSpigot) compile: 'next
    | digit |
    [ self step. (digit := self extract) isNil ] whileTrue.
    self eliminate: digit.
    ^digit' classified: 'stream'.
(Smalltalk globals at: #StdioStream) compile: 'tab
   self nextPut: Character tab' classified: 'benchmarks game'.
(Smalltalk globals at: #StdioStream) compile: 'nl
   self nextPut: Character lf' classified: 'benchmarks game'.
