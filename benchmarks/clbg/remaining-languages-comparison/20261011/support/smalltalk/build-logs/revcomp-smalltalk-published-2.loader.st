Smalltalk compiler evaluate: '"* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
   contributed by Eliot Miranda and Isaac Gouy
   like-a-script contributed by Isaac Gouy *"'.
OldClassDefinitionBuilder buildFromAST: (OCParser parseExpression: 'Object subclass: #BenchmarksGame
   instanceVariableNames: ''''
   classVariableNames: ''''
   poolDictionaries: ''''
   category: ''Benchmarks-Game''').
(Smalltalk globals at: #BenchmarksGame) class compile: 'do: n
   | input lineLength output buffer line |
   input := ZnCharacterReadStream on: (ZnBufferedReadStream on: Stdio stdin).
   lineLength := 60.
   output := ZnCharacterWriteStream on: (ZnBufferedWriteStream on: Stdio stdout).
   buffer := ReadWriteStream on: (String new: 1028).

   #(''ONE'' ''TWO'' ''THREE'') do: [:sequenceName|   
      | prefix description hasDescription complement sequence m |

      prefix := ''>'', sequenceName, ''*''. 
      buffer reset.

      (line notNil and: [prefix match: line])
         ifTrue: [
            description := line]
         ifFalse: [
               [input atEnd or: [(((line := input nextLine) at: 1) = $>) 
                  and: [(hasDescription := prefix match: line) 
                     ifTrue: [description := line]. hasDescription
                  ]]
               ] whileFalse.
            ].

      [input atEnd or: [((line := input nextLine) at: 1) = $>]]
         whileFalse: [ 
            ((line at: 1) = $;) ifFalse: [buffer nextPutAll: line]].


      complement := String new: 128 withAll: $*.

      ''ABCDGHKMNRSTVWY'' with: 
      ''TVGHCDMKNYSABWR''
         do: [:a :b|
            complement at: a asInteger put: b.
            complement at: a asLowercase asInteger put: b].

      sequence := buffer contents.
      m := sequence size.

      output nextPutAll: description; lf.

      [m > 0] whileTrue: [ 
         1 to: ((m < lineLength) ifTrue: [m] ifFalse: [lineLength]) do:
            [:i | output nextPut: 
               (complement at: (sequence at: m - i + 1) asInteger)].
         output lf.
         m := m - lineLength. 
      ].
      output flush.
   ]' classified: 'initialize-release'.
