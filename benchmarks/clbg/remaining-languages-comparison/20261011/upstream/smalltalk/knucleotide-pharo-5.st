
"* The Computer Language Benchmarks Game
    https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
    contributed by Andres Valloud
    modified by Isaac Gouy for Pharo *"!

Object subclass: #BenchmarksGame
   instanceVariableNames: ''
   classVariableNames: ''
   poolDictionaries: ''
   category: 'benchmarks game'!

!SequenceableCollection methodsFor: 'benchmarks game'!

substringFrequencies: aLength using: aDictionary
   | buffer |
   buffer := String new: aLength.
   1 to: self size - aLength + 1 do:
      [:i |
         | answer |
         buffer replaceFrom: 1 to: aLength with: self startingAt: i.
         answer := aDictionary
            at: buffer
            putValueOf: [:sum | sum + 1]
            ifAbsentPutValueOf: 1.
         answer = 1 ifTrue: [buffer := String new: aLength].
      ].
   ^aDictionary! !

!Dictionary methodsFor: 'benchmarks game'!

at: key putValueOf: putBlock ifAbsentPutValueOf: absentBlock
   "* Set the value at key to be the value of evaluating putBlock
    with the existing value. If key is not found, create a new
    entry for key and set is value to the evaluation of
    absentBlock. Answer the result of evaluating either block. *"

   | index element anObject |
   key == nil ifTrue:
      [^self
         subscriptBoundsErrorFor: #at:putValueOf:ifAbsentPutValueOf:
         index: key
         value: absentBlock value].
   index := self findElementOrNil: key.
   element := self basicAt: index.
   element == nil
      ifTrue: [self atNewIndex: index put:
         (self createKey: key value: (anObject := absentBlock value))]
      ifFalse: [element value: (anObject := putBlock value: element value)].
   ^anObject! !

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
         ifTrue: [input upTo: lf]
         ifFalse: [buffer nextPutAll: (input upTo: lf) asString]
      ].
   ^Association key: description value: buffer contents !

knucleotideFrom: input to: output
   "Same as av3, but create less strings while updating the frequencies"

   | sequence writeFrequencies writeCount |

   sequence := (self readFasta: 'THREE' from: input) value asUppercase.

   writeFrequencies :=
      [:k | | frequencies count |
      frequencies := SortedCollection sortBlock: [:a :b|
      (a value = b value) ifTrue: [b key < a key] ifFalse: [b value < a value]].

   count := 0.0.
   (sequence substringFrequencies: k using: (Dictionary new: 1024))
      associationsDo: [:each|
         frequencies add: each. count := count + each value].

   frequencies do: [:each | | percentage |
      percentage := (each value / count) * 100.0.
      output
         nextPutAll: each key; space;
         print: percentage digits: 3; nl]].

   writeCount := [:nucleotideFragment | | frequencies count |
      frequencies := sequence substringFrequencies: nucleotideFragment size
         using: (Dictionary new: 1024).
      count := frequencies at: nucleotideFragment ifAbsent: [0].
      output print: count; tab; nextPutAll: nucleotideFragment; nl].

   writeFrequencies value: 1. output nl.
   writeFrequencies value: 2. output nl.

   writeCount value: 'GGT'.
   writeCount value: 'GGTA'.
   writeCount value: 'GGTATT'.
   writeCount value: 'GGTATTTTAATT'.
   writeCount value: 'GGTATTTTAATTTATAGT'.! !


!BenchmarksGame class methodsFor: 'initialize-release'!

do: n
   self knucleotideFrom: Stdio stdin to: Stdio stdout! !

!Stream methodsFor: 'benchmarks game'!

print: number digits: decimalPlaces
   | precision rounded |
   decimalPlaces <= 0 ifTrue: [^ number rounded printString].
   precision := (10 raisedTo: decimalPlaces negated) asFloat.
   rounded := number roundTo: precision.
   self nextPutAll: 
      ((rounded asScaledDecimal: decimalPlaces) printString copyUpTo: $s)!

nl
   self nextPut: Character lf! !


!StdioStream instance methodsFor: 'positioning'!

skipTo: anObject 
   "Set the access position of the receiver to be past the next occurrence of 
   anObject. Answer whether anObject is found."

   [self atEnd]
      whileFalse: [self next = anObject ifTrue: [^true]].
   ^false! !



    