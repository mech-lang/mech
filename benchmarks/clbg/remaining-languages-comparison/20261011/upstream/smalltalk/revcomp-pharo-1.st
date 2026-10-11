
"* The Computer Language Benchmarks Game
    https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
   contributed by Eliot Miranda and Isaac Gouy *"!


Object subclass: #BenchmarksGame
   instanceVariableNames: ''
   classVariableNames: ''
   poolDictionaries: ''
   category: 'benchmarks game'!


!BenchmarksGame class methodsFor: 'private'!

readFasta: sequenceName from: input
   | sc gt lf prefix description buffer byte line |
   sc := $; asciiValue.
   gt := $> asciiValue.
   lf := Character lf asciiValue.
   prefix := '>',sequenceName.

   "* find start of particular fasta sequence *"
   [(input atEnd) or: [
         (input peek = gt)
            ifTrue: [((line := (input upTo: lf) asString)
               findString: prefix) = 1]
            ifFalse: [input skipTo: lf. false].
          ]
   ] whileFalse.

   "* line-by-line read - it would be a lot faster to block read *"
   description := line.
   buffer := ReadWriteStream on: (String new: 1028).
   [(input atEnd) or: [(byte := input peek) = gt]] whileFalse: [
      (byte = sc)
         ifTrue: [input upTo: newline]
         ifFalse: [buffer nextPutAll: (input upTo: lf) asString]
      ].
   ^Association key: description value: buffer contents!

reverseComplement: sequence named: sequenceName to: output
   | complement newline lineLength n |
   (sequenceName isNil) ifTrue: [^self].

   complement := String new: 128 withAll: $*.

   'ABCDGHKMNRSTVWY' with: 
   'TVGHCDMKNYSABWR'
      do: [:a :b|
         complement at: a asInteger put: b.
         complement at: a asLowercase asInteger put: b].

   newline := Character lf.
   lineLength := 60.
   n := sequence size.

   output nextPutAll: sequenceName; nextPut: newline.

   [n > 0] whileTrue: [ 
         1 to: ((n < lineLength) ifTrue: [n] ifFalse: [lineLength]) do:
            [:i | output nextPut: 
               (complement at: (sequence at: n - i + 1) asInteger)].
         output nextPut: newline.
         n := n - lineLength. 
      ]! !

!BenchmarksGame class methodsFor: 'initialize-release'!

do: n
   | input output |
   input := Stdio stdin.
   output := Stdio stdout.
   #('ONE' 'TWO' 'THREE') do:
      [:sequenceName|   | fasta |
         fasta := self readFasta: sequenceName from: input.
         self reverseComplement: fasta value named: fasta key to: output.
      ].
   output flush! !

!StdioStream instance methodsFor: 'positioning'!

skipTo: anObject 
   "Set the access position of the receiver to be past the next occurrence of 
   anObject. Answer whether anObject is found."

   [self atEnd]
      whileFalse: [self next = anObject ifTrue: [^true]].
   ^false! !

    