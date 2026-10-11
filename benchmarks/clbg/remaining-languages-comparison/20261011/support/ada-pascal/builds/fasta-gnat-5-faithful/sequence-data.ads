    
package Sequence.Data is
   
   pragma Pure (Data);
   
   Homosapiens : constant Nucleotide_Set(0..3) :=
    (('a', 0.3029549426680), ('c', 0.1979883004921),
     ('g', 0.1975473066391), ('t', 0.3015094502008));

   IUB : constant Nucleotide_Set(0..14) :=
    (('a', 0.27), ('c', 0.12), ('g', 0.12), ('t', 0.27),
     ('B', 0.02), ('D', 0.02), ('H', 0.02), ('K', 0.02),
     ('M', 0.02), ('N', 0.02), ('R', 0.02), ('S', 0.02),
     ('V', 0.02), ('W', 0.02), ('Y', 0.02));

   ALU : constant String :=
     "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG" &
     "GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA" &
     "CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT" &
     "ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA" &
     "GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG" &
     "AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC" &
     "AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA";
   
end Sequence.Data;
