
"* The Computer Language Benchmarks Game
    https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
    contributed by Isaac Gouy
    modified by Eliot Miranda 
*"!

Object subclass: #BenchmarksGame
   instanceVariableNames: ''
   classVariableNames: ''
   poolDictionaries: ''
   category: 'Benchmarks-Game'!

Object subclass: #TreeNode
   instanceVariableNames: 'left right'
   classVariableNames: ''
   poolDictionaries: ''
   category: 'Benchmarks-Game'!


!BenchmarksGame class methodsFor: 'initialize-release'!

do: n
   | minDepth maxDepth stretchDepth check longLivedTree iterations |
   minDepth := 4.
   maxDepth := minDepth + 2 max: n.
   stretchDepth := maxDepth + 1.

   check := (TreeNode bottomUpTree: stretchDepth) itemCheck.
   Stdio stdout
      nextPutAll: 'stretch tree of depth '; print: stretchDepth; tab;
      nextPutAll: ' check: '; print: check; nl.

   longLivedTree := TreeNode bottomUpTree: maxDepth.
   minDepth to: maxDepth by: 2 do: [:depth|
      iterations := 1 bitShift: maxDepth - depth + minDepth.

      check := 0.
      1 to: iterations do: [:i|
         check := check + (TreeNode bottomUpTree: depth) itemCheck
         ].
      Stdio stdout
         print: iterations; tab;
         nextPutAll: ' trees of depth '; print: depth; tab;
         nextPutAll: ' check: '; print: check; nl
      ].

   Stdio stdout
      nextPutAll: 'long lived tree of depth '; print: maxDepth; tab;
      nextPutAll: ' check: '; print: longLivedTree itemCheck; nl! !

!TreeNode class methodsFor: 'instance creation'!

bottomUpTree: anInteger
   ^(anInteger > 0) 
      ifTrue: [
         self 
            left: (self bottomUpTree: anInteger - 1) 
            right: (self bottomUpTree: anInteger - 1)  
         ]
      ifFalse: [self left: nil right: nil]!

left: leftChild right: rightChild      
   ^(super new) left: leftChild right: rightChild! !


!TreeNode methodsFor: 'accessing'!

itemCheck
   ^left isNil 
      ifTrue: [1] ifFalse: [1 + left itemCheck + right itemCheck]! !

!TreeNode methodsFor: 'initialize-release'!

left: leftChild right: rightChild
   left := leftChild.
   right := rightChild! !


!StdioStream methodsFor: 'benchmarks game'!

tab
   self nextPut: Character tab!

nl
   self nextPut: Character lf! !
    