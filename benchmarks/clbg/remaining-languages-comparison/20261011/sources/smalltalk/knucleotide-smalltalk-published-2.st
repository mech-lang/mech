
"* The Computer Language Benchmarks Game
    http://shootout.alioth.debian.org/
    contributed by Isaac Gouy *"!

Object subclass: #BenchmarksGame
   instanceVariableNames: ''
   classVariableNames: ''
   poolDictionaries: ''
   category: 'Benchmarks-Game'!


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
   | prefix line hasDescription description buffer |

   prefix := '>', sequenceName, '*'.

   [input atEnd or: [(((line := input nextLine) at: 1) = $>) 
      and: [(hasDescription := prefix match: line) 
         ifTrue: [description := line]. hasDescription
      ]]
   ] whileFalse.

   buffer := ReadWriteStream on: (String new: 1028).
   [input atEnd or: [((line := input nextLine) at: 1) = $>]]
      whileFalse: [ 
         ((line at: 1) = $;) ifFalse: [buffer nextPutAll: line]].

   ^Association key: description value: buffer contents!


knucleotideFrom: input to: output
   | sequence writeFrequencies writeCount |

   sequence := (self readFasta: 'THREE' from: input) value asUppercase.

   writeFrequencies :=
      [:k | | frequencies count |
      frequencies := SortedCollection sortBlock: [:a :b|
         (a value = b value) ifTrue: [a key < b key] ifFalse: [b value < a value]].

      count := 0.0.
      (self substringFrequencies: sequence for: k using: Dictionary new)
         associationsDo: [:each|
            frequencies add: each. count := count + each value].

      frequencies do: [:each | | percentage |
         percentage := (each value / count) * 100.0.
         output 
            nextPutAll: each key; nextPutAll: ' ';
            print: percentage digits: 3; lf]]. 

   writeCount := [:nucleotideFragment | | frequencies count |
      frequencies := self substringFrequencies: sequence 
         for: nucleotideFragment size
         using: Dictionary new.
      count := frequencies at: nucleotideFragment ifAbsent: [0].
      output print: count; tab; nextPutAll: nucleotideFragment; lf].

   writeFrequencies value: 1. output lf.
   writeFrequencies value: 2. output lf.

   writeCount value: 'GGT'.
   writeCount value: 'GGTA'.
   writeCount value: 'GGTATT'.
   writeCount value: 'GGTATTTTAATT'.
   writeCount value: 'GGTATTTTAATTTATAGT'! !


!BenchmarksGame class methodsFor: 'initialize-release'!

do: n
   self knucleotideFrom: 
         (ZnCharacterReadStream on: 
            (ZnBufferedReadStream on: Stdio stdin)) 
      to: Stdio stdout! !


!StdioStream methodsFor: 'benchmarks game'!

tab
   self nextPut: Character tab!

print: number digits: decimalPlaces
   | precision rounded |
   decimalPlaces <= 0 ifTrue: [^ number rounded printString].
   precision := (10 raisedTo: decimalPlaces negated) asFloat.
   rounded := number roundTo: precision.
   self nextPutAll: 
      ((rounded asScaledDecimal: decimalPlaces) printString copyUpTo: $s)! !


    