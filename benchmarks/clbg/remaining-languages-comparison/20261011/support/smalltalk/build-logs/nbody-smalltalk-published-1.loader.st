Smalltalk compiler evaluate: '"* The Computer Language Benchmarks Game
    https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
    contributed by Isaac Gouy *"'.
OldClassDefinitionBuilder buildFromAST: (OCParser parseExpression: 'Object subclass: #NBodySystem
   instanceVariableNames: ''bodies''
   classVariableNames: ''''
   poolDictionaries: ''''
   category: ''Benchmarks-Game''').
OldClassDefinitionBuilder buildFromAST: (OCParser parseExpression: 'Object subclass: #Body
   instanceVariableNames: ''x y z vx vy vz mass''
   classVariableNames: ''''
   poolDictionaries: ''''
   category: ''Benchmarks-Game''').
OldClassDefinitionBuilder buildFromAST: (OCParser parseExpression: 'Object subclass: #BenchmarksGame
   instanceVariableNames: ''''
   classVariableNames: ''''
   poolDictionaries: ''''
   category: ''Benchmarks-Game''').
(Smalltalk globals at: #Body) class compile: 'daysPerYear
   ^365.24d0' classified: 'constants'.
(Smalltalk globals at: #Body) class compile: 'sun
   ^self new
      x: 0.0d0
      y: 0.0d0
      z: 0.0d0
      vx: 0.0d0
      vy: 0.0d0
      vz: 0.0d0
      mass: self solarMass' classified: 'constants'.
(Smalltalk globals at: #Body) class compile: 'uranus
   ^self new
      x: 1.28943695621391310d1
      y: -1.51111514016986312d1
      z: -2.23307578892655734d-1
      vx: 2.96460137564761618d-3 * self daysPerYear
      vy: 2.37847173959480950d-3 * self daysPerYear
      vz: -2.96589568540237556d-5 * self daysPerYear
      mass: 4.36624404335156298d-5 * self solarMass' classified: 'constants'.
(Smalltalk globals at: #Body) class compile: 'saturn
   ^self new
      x: 8.34336671824457987d0
      y: 4.12479856412430479d0
      z: -4.03523417114321381d-1
      vx: -2.76742510726862411d-3 * self daysPerYear
      vy: 4.99852801234917238d-3 * self daysPerYear
      vz: 2.30417297573763929d-5 * self daysPerYear
      mass: 2.85885980666130812d-4 * self solarMass' classified: 'constants'.
(Smalltalk globals at: #Body) class compile: 'solarMass
   ^4.0d0 * self pi * self pi' classified: 'constants'.
(Smalltalk globals at: #Body) class compile: 'pi
   ^3.141592653589793d0' classified: 'constants'.
(Smalltalk globals at: #Body) class compile: 'jupiter
   ^self new
      x: 4.84143144246472090d0
      y: -1.16032004402742839d0
      z: -1.03622044471123109d-1
      vx: 1.66007664274403694d-3 * self daysPerYear
      vy: 7.69901118419740425d-3 * self daysPerYear
      vz: -6.90460016972063023d-5 * self daysPerYear
      mass: 9.54791938424326609d-4 * self solarMass' classified: 'constants'.
(Smalltalk globals at: #Body) class compile: 'neptune
   ^self new
      x: 1.53796971148509165d1
      y: -2.59193146099879641d1
      z: 1.79258772950371181d-1
      vx: 2.68067772490389322d-3 * self daysPerYear
      vy: 1.62824170038242295d-3 * self daysPerYear
      vz: -9.51592254519715870d-5 * self daysPerYear
      mass: 5.15138902046611451d-5 * self solarMass' classified: 'constants'.
(Smalltalk globals at: #Body) compile: 'z
   ^z' classified: 'accessing'.
(Smalltalk globals at: #Body) compile: 'x
   ^x' classified: 'accessing'.
(Smalltalk globals at: #Body) compile: 'mass
   ^mass' classified: 'accessing'.
(Smalltalk globals at: #Body) compile: 'x: d1 y: d2 z: d3 vx: d4 vy: d5 vz: d6 mass: d7
   x := d1.
   y := d2. 
   z := d3. 
   vx := d4.
   vy := d5.
   vz := d6.
   mass := d7' classified: 'accessing'.
(Smalltalk globals at: #Body) compile: 'y
   ^y' classified: 'accessing'.
(Smalltalk globals at: #Body) compile: 'offsetMomentum: anArray 
   | m |
   m := self class solarMass.
   vx := (anArray at: 1) negated / m.
   vy := (anArray at: 2) negated / m.
   vz := (anArray at: 3) negated / m' classified: 'nbody'.
(Smalltalk globals at: #Body) compile: 'decreaseVelocity: dx y: dy z: dz m: m
   vx := vx - (dx * m).
   vy := vy - (dy * m).
   vz := vz - (dz * m)' classified: 'nbody'.
(Smalltalk globals at: #Body) compile: 'positionAfter: dt
   x := x + (dt * vx).
   y := y + (dt * vy).
   z := z + (dt * vz)' classified: 'nbody'.
(Smalltalk globals at: #Body) compile: 'and: aBody velocityAfter: dt        
   | dx dy dz distance mag |
   dx := x - aBody x.
   dy := y - aBody y.
   dz := z - aBody z.
   
   distance := ((dx*dx) + (dy*dy) + (dz*dz)) sqrt.
   mag := dt / (distance * distance * distance).

   self decreaseVelocity: dx y: dy z: dz m: aBody mass * mag.   
   aBody increaseVelocity: dx y: dy z: dz m: mass * mag' classified: 'nbody'.
(Smalltalk globals at: #Body) compile: 'potentialEnergy: aBody
   | dx dy dz distance |
   dx := x - aBody x.
   dy := y - aBody y.
   dz := z - aBody z.

   distance := ((dx*dx) + (dy*dy) + (dz*dz)) sqrt.
   ^mass * aBody mass / distance' classified: 'nbody'.
(Smalltalk globals at: #Body) compile: 'addMomentumTo: anArray
   anArray at: 1 put: (anArray at: 1) + (vx * mass).
   anArray at: 2 put: (anArray at: 2) + (vy * mass).
   anArray at: 3 put: (anArray at: 3) + (vz * mass).
   ^anArray' classified: 'nbody'.
(Smalltalk globals at: #Body) compile: 'increaseVelocity: dx y: dy z: dz m: m
   vx := vx + (dx * m).
   vy := vy + (dy * m).
   vz := vz + (dz * m)' classified: 'nbody'.
(Smalltalk globals at: #Body) compile: 'kineticEnergy
   ^0.5d0 * mass * ((vx * vx) + (vy * vy) + (vz * vz))' classified: 'nbody'.
(Smalltalk globals at: #NBodySystem) compile: 'initialize
   bodies := OrderedCollection new
      add: Body sun; add: Body jupiter; add: Body saturn;
      add: Body uranus; add: Body neptune; yourself.

   bodies first offsetMomentum:
      (bodies inject: (Array with: 0.0d0 with: 0.0d0 with: 0.0d0)
         into: [:m :each | each addMomentumTo: m])' classified: 'initialize-release'.
(Smalltalk globals at: #NBodySystem) compile: 'after: dt
   1 to: bodies size do: [:i|
      i+1 to: bodies size do: [:j|                            
         (bodies at: i) and: (bodies at: j) velocityAfter: dt].
   ].   
   bodies do: [:each| each positionAfter: dt]' classified: 'nbody'.
(Smalltalk globals at: #NBodySystem) compile: 'energy
   | e |
   e := 0.0d0.
   1 to: bodies size do: [:i|       
      e := e + (bodies at: i) kineticEnergy.

      i+1 to: bodies size do: [:j| 
         e := e - ((bodies at: i) potentialEnergy: (bodies at: j))].
   ].
   ^e' classified: 'nbody'.
(Smalltalk globals at: #BenchmarksGame) class compile: 'do: n
   | bodies |
   bodies := NBodySystem new initialize.
   Stdio stdout print: bodies energy digits: 9; nl.
   n timesRepeat: [bodies after: 0.01d0].
   Stdio stdout print: bodies energy digits: 9; nl' classified: 'initialize-release'.
(Smalltalk globals at: #StdioStream) compile: 'nl
   self nextPut: Character lf' classified: 'benchmarks game'.
(Smalltalk globals at: #StdioStream) compile: 'print: number digits: decimalPlaces
   | precision rounded |
   decimalPlaces <= 0 ifTrue: [^ number rounded printString].
   precision := (10 raisedTo: decimalPlaces negated) asFloat.
   rounded := number roundTo: precision.
   self nextPutAll: 
      ((rounded asScaledDecimal: decimalPlaces) printString copyUpTo: $s)' classified: 'benchmarks game'.
