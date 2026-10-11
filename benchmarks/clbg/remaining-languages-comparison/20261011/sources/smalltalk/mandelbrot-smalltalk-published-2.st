
"* The Computer Language Benchmarks Game
    https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
    contributed by Paolo Bonzini 
    reworked by Isaac Gouy *"!


Object subclass: #BenchmarksGame
   instanceVariableNames: ''
   classVariableNames: ''
   poolDictionaries: ''
   category: 'Benchmarks-Game'!


!BenchmarksGame class methodsFor: 'private'!

mandelbrot: extent to: output
   | limit2 m bits zr zi cr ci i tr stepr stepi |
   limit2 := 4.0d0.
   m := 50.

   stepr := 2.0d0 / extent.
   stepi := 2.0d0 / extent.

   0 to: extent - 1 do: [ :y |
      bits := 0.
      ci := stepi * y asFloat - 1.0d0.
      0 to: extent - 1 do: [ :x |
         cr := stepr * x asFloat - 1.5d0.
         zr := 0.0d0.
         zi := 0.0d0.

         bits := bits bitShift: 1.
         i := 0.  
         [
            tr := (zr*zr) - (zi*zi) + cr.
            zi := 2.0d0 * zr * zi + ci.
            zr := tr.
            (zr*zr) + (zi*zi) <= limit2 and: [ (i := i + 1) < m ]
         ] whileTrue.

         i = m ifTrue: [ bits := bits + 1 ].
         (x bitAnd: 7) == 7 ifTrue: [
            output nextPut: bits.
            bits := 0.
         ]
      ]. 
      (extent bitAnd: 7) == 0 ifFalse: [
         bits := bits bitShift: 8 - (extent bitAnd: 7).
         output nextPut: bits.
      ]
   ]! !

!BenchmarksGame class methodsFor: 'initialize-release'!

do: n
   | out |
   Stdio stdout
      nextPutAll: 'P4'; lf; 
      print: n; nextPutAll: ' '; print: n; lf;
      binary.

   out := (ZnBufferedWriteStream on: Stdio stdout).
   self mandelbrot: n to: out.
   out flush! !

    