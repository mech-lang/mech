
/* The Computer Language Benchmarks game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   transliterated from Greg Buchholz's C program by Isaac Gouy 
*/  


import host.pipe
    
    
main args:

  w := 0; h := 0; x := 0; y := 0; bit_num := 0
  byte_acc ::= #[0] 
  i := 0; iterations := 50
  limit ::= 2.0
  Zr := 0.0; Zi := 0.0; Cr := 0.0; Ci := 0.0; Tr := 0.0; Ti := 0.0

  w = args.size == 1 ? int.parse args[0] : 200
  h = w

  print "P4\n$w $h"
  out := pipe.stdout 
  for y = 0; y < h; ++y:   
  
    for  x = 0; x < w; ++x:
  
      Zr = 0.0; Zi = 0.0 
      Cr = (2.0 * x / w - 1.5); Ci = (2.0 * y / h - 1.0)   
 
      for  i = 0; i < iterations; ++i:

        Tr = Zr*Zr - Zi*Zi + Cr
        Ti = 2*Zr*Zi + Ci          
        Zr = Tr; Zi = Ti 
        if Zr*Zr+Zi*Zi > limit*limit:
          break


      if Zr*Zr+Zi*Zi > limit*limit: 
        byte_acc[0] = (byte_acc[0] << 1) | 0x00
      else:
        byte_acc[0] = (byte_acc[0] << 1) | 0x01
                
      bit_num += 1   
      
      if bit-num == 8:
      
        out.write byte_acc        
        byte_acc[0] = 0
        bit_num = 0      
      
      else:
        if x == w - 1:      
          byte_acc[0] = byte_acc[0] << (8 - w%8)   
          out.write byte_acc 
          byte_acc[0] = 0
          bit_num = 0   
          
  out.close 
    