
with Ada.Command_Line; use Ada.Command_Line;
with Fasta_Pck;        use Fasta_Pck;

procedure Fasta is
   Homosapiens : Aminoacid_Set :=
                   (('a', 0.3029549426680), ('c', 0.1979883004921),
                    ('g', 0.1975473066391), ('t', 0.3015094502008));
   Iub         : Aminoacid_Set :=
                   (('a', 0.27), ('c', 0.12), ('g', 0.12), ('t', 0.27),
                    ('B', 0.02), ('D', 0.02), ('H', 0.02), ('K', 0.02),
                    ('M', 0.02), ('N', 0.02), ('R', 0.02), ('S', 0.02),
                    ('V', 0.02), ('W', 0.02), ('Y', 0.02));
   Alu         : constant String :=
                    "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG" &
                    "GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA" &
                    "CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT" &
                    "ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA" &
                    "GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG" &
                    "AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC" &
                    "AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA";

   N           : constant Positive := Positive'Value (Argument (1));

begin
   Make_Cumulative (Iub);
   Make_Cumulative (Homosapiens);

   Make_Repeat_Fasta ("ONE", "Homo sapiens alu", alu, N * 2);
   Make_Random_Fasta ("TWO", "IUB ambiguity codes", Iub, N * 3);
   Make_Random_Fasta
     ("THREE", "Homo sapiens frequency", Homosapiens, N * 5);
end Fasta;
