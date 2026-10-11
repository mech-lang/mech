/* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   Naive transliteration from Drake Diedrich's C program
   contributed by Isaac Gouy 
*/

let IM = 139968
let IA = 3877
let IC = 29573
let SEED = 42

var seed = SEED
func fastaRand(_ max: Double) -> Double {
   seed = (seed * IA + IC) % IM
   return max * Double(seed) / Double(IM)
}

let ALU =
  "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG" +
  "GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA" +
  "CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT" +
  "ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA" +
  "GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG" +
  "AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC" +
  "AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA"
let ALU_C = Array<Character>(ALU)

let IUB = "acgtBDHKMNRSVWY"
let IUB_C =  Array<Character>(IUB)
let IUB_P = [
   0.27, 0.12, 0.12, 0.27, 
   0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02 
]

let HOMOSAPIENS = "acgt"
let HOMOSAPIENS_C =  Array<Character>(HOMOSAPIENS)
let HOMOSAPIENS_P = [
   0.3029549426680,
   0.1979883004921,
   0.1975473066391,
   0.3015094502008
]  

let LINELEN = 60

// slowest character-at-a-time output
func repeatFasta(_ seq: [Character], _ n: Int) {
   let len = seq.count
   var i = 0
   // explicit line buffer    
   var b = ""     
   while i < n {
      b.append(seq[i % len])     
      if i % LINELEN == LINELEN - 1 { print(b)
 b = "" }  
      i += 1
   }      
   if i % LINELEN != 0  { print(b) }
}

func randomFasta(_ seq: [Character], _ probability: [Double], _ n: Int) {
   let len = seq.count
   var i = 0
   // explicit line buffer    
   var b = ""        
   while i < n {   
      var v = fastaRand(1.0) 
      /* slowest idiomatic linear lookup.  Fast if len is short though. */ 
      var j = 0        
      while j < len {      
         v -= probability[j]       
         if v < 0.0 { break }   
         j += 1         
      } 
      b.append(seq[j])      
      if i % LINELEN == LINELEN - 1 { print(b)
 b = "" }    
      i += 1
   }
   if i % LINELEN != 0 { print(b) }     
}

func main(_ n: Int) {   
   print(">ONE Homo sapiens alu")    
   repeatFasta(ALU_C, n*2)

   print(">TWO IUB ambiguity codes")
    
   randomFasta(IUB_C, IUB_P, n*3)  
   
   print(">THREE Homo sapiens frequency")
    
   randomFasta(HOMOSAPIENS_C, HOMOSAPIENS_P, n*5)    
}            

main( 
   (CommandLine.argc > 1) 
      ? Int(CommandLine.arguments[1])! 
      : 1000 )
