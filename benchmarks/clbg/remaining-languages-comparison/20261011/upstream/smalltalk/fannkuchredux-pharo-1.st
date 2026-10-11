
"* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
   contributed by Paolo Bonzini 
   modified by Isaac Gouy *"!

Object subclass: #BenchmarksGame
   instanceVariableNames: ''
   classVariableNames: ''
   poolDictionaries: ''
   category: 'benchmarks game'!

Object subclass: #PermGeneratorRedux
   instanceVariableNames: 'timesRotated perm atEnd'
   classVariableNames: ''
   poolDictionaries: ''
   category: 'benchmarks game'!


!PermGeneratorRedux class methodsFor: 'instance creation'!

new: size
   ^self new
      initialize: size;
      yourself! !


!PermGeneratorRedux methodsFor: 'accessing'!

atEnd
   ^atEnd!

maxPfannkuchenTo: output
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
   ^max!

next
   | result |
   result := perm copy.
   self makeNext.
   ^result! !

!PermGeneratorRedux methodsFor: 'initialize-release'!

initialize: size
   perm := (1 to: size) asArray.
   timesRotated := Array new: size withAll: 0.
   atEnd := false.!

makeNext
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
   atEnd := true! !


!BenchmarksGame class methodsFor: 'private'!

fannkuchRedux: n to: output
   ^(PermGeneratorRedux new: n) maxPfannkuchenTo: output! !

!BenchmarksGame class methodsFor: 'initialize-release'!

do: n
   | f |
   f := self fannkuchRedux: n to: Stdio stdout.
   Stdio stdout
      nextPutAll: 'Pfannkuchen(', n printString, ') = ';
      print: f; nl! !


!Array methodsFor: 'benchmarks game'!

pfannkuchen
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
   ^k! !


!StdioStream methodsFor: 'benchmarks game'!

nl
   self nextPut: Character lf! !
    