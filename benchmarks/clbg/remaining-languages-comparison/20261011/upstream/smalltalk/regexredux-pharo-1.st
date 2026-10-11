
"* The Computer Language Benchmarks Game
    http://benchmarksgame.alioth.debian.org/
    contributed by Eliot Miranda
    modified by Isaac Gouy *"!


Object subclass: #BenchmarksGame
   instanceVariableNames: ''
   classVariableNames: ''
   poolDictionaries: ''
   category: 'benchmarks game'!


!BenchmarksGame class methodsFor: 'private'!

matchPatterns
   ^#(   'agggtaaa|tttaccct'
         '[cgt]gggtaaa|tttaccc[acg]'
         'a[act]ggtaaa|tttacc[agt]t'
         'ag[act]gtaaa|tttac[agt]ct'
         'agg[act]taaa|ttta[agt]cct'
         'aggg[acg]aaa|ttt[cgt]ccct'
         'agggt[cgt]aa|tt[acg]accct'
         'agggta[cgt]a|t[acg]taccct'
         'agggtaa[cgt]|[acg]ttaccct'
   )! !

!BenchmarksGame class methodsFor: 'private'!

substitutionPatterns
   ^#(   #('tHa[Nt]' '<4>')
         #('aND|caN|Ha[DS]|WaS' '<3>')
         #('a[NSt]|BY' '<2>')
         #('<[^>]*>' '|')
         #('\\|[^|][^|]*\\|' '-'))! !


!BenchmarksGame class methodsFor: 'private'!

regexDNA: sequence to: output
   | s size1 size2 translation |
   size1 := sequence size.

   "* remove FASTA sequence descriptions and new-lines *"
   s := sequence copyWithRegex: '>[^\r]*\r|\r' matchesReplacedWith: ''.
   size2 := s size.

   "* regex match *"
   self matchPatterns do: [:each| 
      output 
         nextPutAll: each; space; 
         print: (s occurrencesOfRegex: each); nl
      ]. 

   "* regex substitution *"
   translation := Dictionary new.
   self substitutionPatterns do: [:each| 
      translation at: each first put: each last].

   s := s copyWithRegex: '[', 
         (translation keys asArray fold: [:a :b| a, b]), ']'
      matchesTranslatedUsing: [:l| translation at: l].

   output
      nl;
      print: size1; nl; 
      print: size2; nl; 
      print: s size; nl! !


!BenchmarksGame class methodsFor: 'initialize-release'!

do: n
   self regexDNA: Stdio stdin contents asString to: Stdio stdout! !


!Stream methodsFor: 'benchmarks game'!

nl
   self nextPut: Character lf! !

    