Smalltalk compiler evaluate: '"* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
   contributed by Paolo Bonzini 
   modified by Isaac Gouy *"'.
OldClassDefinitionBuilder buildFromAST: (OCParser parseExpression: 'Object subclass: #BenchmarksGame
   instanceVariableNames: ''''
   classVariableNames: ''''
   poolDictionaries: ''''
   category: ''Benchmarks-Game''').
OldClassDefinitionBuilder buildFromAST: (OCParser parseExpression: 'Object subclass: #PermGeneratorRedux
   instanceVariableNames: ''timesRotated perm atEnd''
   classVariableNames: ''''
   poolDictionaries: ''''
   category: ''Benchmarks-Game''').
(Smalltalk globals at: #PermGeneratorRedux) class compile: 'new: size
   ^self new
      initialize: size;
      yourself' classified: 'instance creation'.
(Smalltalk globals at: #PermGeneratorRedux) compile: 'atEnd
   ^atEnd' classified: 'accessing'.
(Smalltalk globals at: #PermGeneratorRedux) compile: 'maxPfannkuchenTo: output
   | max permutation checksum permCount flipsCount |
   max := 0.
   permCount := 0.
   checksum := 0.
   [self atEnd] whileFalse:
      [permutation := self next.
      permCount := permCount + 1.
      (permCount = 1048576) ifTrue: [permCount := 0].
      flipsCount := permutation pfannkuchen.
      checksum := permCount odd 
         ifTrue: [checksum+flipsCount] 
         ifFalse: [checksum-flipsCount].
      max := max max: flipsCount].
   output print: checksum; nl.
   ^max' classified: 'accessing'.
(Smalltalk globals at: #PermGeneratorRedux) compile: 'next
   | result |
   result := perm copy.
   self makeNext.
   ^result' classified: 'accessing'.
(Smalltalk globals at: #PermGeneratorRedux) compile: 'initialize: size
   perm := (1 to: size) asArray.
   timesRotated := Array new: size withAll: 0.
   atEnd := false.' classified: 'initialize-release'.
(Smalltalk globals at: #PermGeneratorRedux) compile: 'makeNext
   | temp remainder |
   "* Generate the next permutation. *"
   2 to: perm size do: [ :r |
      "* Rotate the first r items to the left. *"
      temp := perm at: 1.
      1 to: r - 1 do: [ :i | perm at: i put: (perm at: i + 1) ].
      perm at: r put: temp.

      remainder := timesRotated at: r 
                                put: ((timesRotated at: r) + 1) \\ r.
      remainder = 0 ifFalse: [ ^self ].

      "* After r rotations, the first r items 
         are in their original positions.
      Go on rotating the first r+1 items. *"
   ].

   "* We are past the final permutation. *"
   atEnd := true' classified: 'initialize-release'.
(Smalltalk globals at: #BenchmarksGame) class compile: 'fannkuchRedux: n to: output
   ^(PermGeneratorRedux new: n) maxPfannkuchenTo: output' classified: 'private'.
(Smalltalk globals at: #BenchmarksGame) class compile: 'do: n
   | f |
   f := self fannkuchRedux: n to: Stdio stdout.
   Stdio stdout
      nextPutAll: ''Pfannkuchen('', n printString, '') = '';
      print: f; nl' classified: 'initialize-release'.
(Smalltalk globals at: #Array) compile: 'pfannkuchen
   | first complement a b k |
   k := 0.
   [ (first := self at: 1) == 1 ] whileFalse: [
      k := k + 1.
      complement := first + 1.
      1 to: first // 2 do: [ :i |
         a := self at: i.
         b := self at: complement - i.
         self at: i put: b.
         self at: complement - i put: a.
      ]
   ].
   ^k' classified: 'benchmarks game'.
(Smalltalk globals at: #StdioStream) compile: 'nl
   self nextPut: Character lf' classified: 'benchmarks game'.
