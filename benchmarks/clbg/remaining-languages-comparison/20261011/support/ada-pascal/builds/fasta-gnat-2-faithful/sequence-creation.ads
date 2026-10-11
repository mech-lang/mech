

with Ada.Finalization;

package Sequence.Creation is

   procedure Make_Random_Fasta
     (Title       : in String; 
      Nucleotides : in Nucleotide_Set; 
      N           : in Positive);

   procedure Make_Repeat_Fasta 
     (Title : in String;
      S     : in String; 
      N     : in Positive);
   
   type Environment is new Ada.Finalization.Limited_Controlled
     with null record;

private

   overriding
   procedure Initialize (Active : in out Environment);

   overriding
   procedure Finalize (Active : in out Environment);

end Sequence.Creation;
