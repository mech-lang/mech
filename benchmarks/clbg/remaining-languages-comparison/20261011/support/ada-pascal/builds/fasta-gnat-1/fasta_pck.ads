-- The Computer Language Benchmarks Game
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
-- Contributed by Pascal Obry on 2005/04/07

package Fasta_Pck is

   type Real is new Long_Float;

   type Aminoacid is record
      C : Character;
      P : Real;
   end record;

   type Aminoacid_Set is array (Positive range <>) of Aminoacid;

   procedure Make_Cumulative (Gene_List : in out Aminoacid_Set);

   function Select_Random (Gene_List : in Aminoacid_Set) return Character;

   procedure Make_Random_Fasta
     (Id, Desc : in String; Gene_List : in Aminoacid_Set; N : in Positive);

   procedure Make_Repeat_Fasta (Id, Desc, S : in String; N : in Positive);

end Fasta_Pck;
