
"* The Computer Language Benchmarks Game
    http://shootout.alioth.debian.org/
    unoptimized program contributed long ago by Isaac Gouy *"!

Object subclass: #BenchmarksGame
   instanceVariableNames: ''
   classVariableNames: ''
   poolDictionaries: ''
   category: 'benchmarks game'!


!BenchmarksGame class methodsFor: 'private'!

substringFrequencies: aString for: aLength using: aDictionary
   | m |
   m := aString size - aLength + 1.
   1 to: m do: [:i | | fragment |
      fragment := aString copyFrom: i to: i + aLength - 1.

      aDictionary at: fragment put: 
         (aDictionary at: fragment ifAbsent: [0]) + 1 
   ].
   ^aDictionary!


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
         ifTrue: [input upTo: lf]
         ifFalse: [buffer nextPutAll: (input upTo: lf) asString]
      ].
   ^Association key: description value: buffer contents!


knucleotideFrom: input to: output
   | sequence writeFrequencies writeCount |

   sequence := (self readFasta: 'THREE' from: input) value asUppercase.

   writeFrequencies :=
      [:k | | frequencies count |
      frequencies := SortedCollection sortBlock: [:a :b|
         (a value = b value) ifTrue: [b key < a key] ifFalse: [b value < a value]].

      count := 0.0.
      (self substringFrequencies: sequence for: k using: Dictionary new)
         associationsDo: [:each|
            frequencies add: each. count := count + each value].

      frequencies do: [:each | | percentage |
         percentage := (each value / count) * 100.0.
         output 
            nextPutAll: each key; nextPutAll: ' ';
            print: percentage digits: 3; nl]]. 

   writeCount := [:nucleotideFragment | | frequencies count |
      frequencies := self substringFrequencies: sequence 
         for: nucleotideFragment size
         using: Dictionary new.
      count := frequencies at: nucleotideFragment ifAbsent: [0].
      output print: count; tab; nextPutAll: nucleotideFragment; nl].

   writeFrequencies value: 1. output nl.
   writeFrequencies value: 2. output nl.

   writeCount value: 'GGT'.
   writeCount value: 'GGTA'.
   writeCount value: 'GGTATT'.
   writeCount value: 'GGTATTTTAATT'.
   writeCount value: 'GGTATTTTAATTTATAGT'! !


!BenchmarksGame class methodsFor: 'initialize-release'!

do: n
   self knucleotideFrom: Stdio stdin to: Stdio stdout! !


!StdioStream methodsFor: 'benchmarks game'!

tab
   self nextPut: Character tab!

nl
   self nextPut: Character lf!

print: number digits: decimalPlaces
   | precision rounded |
   decimalPlaces <= 0 ifTrue: [^ number rounded printString].
   precision := (10 raisedTo: decimalPlaces negated) asFloat.
   rounded := number roundTo: precision.
   self nextPutAll: 
      ((rounded asScaledDecimal: decimalPlaces) printString copyUpTo: $s)! !


!StdioStream instance methodsFor: 'positioning'!

skipTo: anObject 
   "Set the access position of the receiver to be past the next occurrence of 
   anObject. Answer whether anObject is found."

   [self atEnd]
      whileFalse: [self next = anObject ifTrue: [^true]].
   ^false! !

    