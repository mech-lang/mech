-- The Computer Language Benchmarks Game
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
-- contributed by Pascal Obry on 2005/04/07
-- modified by Gautier de Montmollin
-- modified by Georg Bauhaus, Jonathan Parker (July 2011)
-- modified by Gautier de Montmollin

with Ada.Command_Line;
with GNAT.Float_Control;
with Sequence.Data, Sequence.Creation;

procedure Fasta is

   N : constant Positive := Positive'Value (Ada.Command_Line.Argument (1));
   
   use Sequence.Data, Sequence.Creation;
   
   Runner : Environment;
begin
   GNAT.Float_Control.Reset;

   Make_Repeat_Fasta (">ONE Homo sapiens alu", ALU, N*2);
   Make_Random_Fasta (">TWO IUB ambiguity codes", IUB, N*3);
   Make_Random_Fasta (">THREE Homo sapiens frequency", Homosapiens, N*5);

end Fasta;
