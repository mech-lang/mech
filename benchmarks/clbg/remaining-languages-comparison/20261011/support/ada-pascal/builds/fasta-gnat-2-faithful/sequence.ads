
package Sequence is
   
   pragma Pure (Sequence);
   
   type Real is digits 15;

   type Nucleotide is record
      C : Character;
      P : Real;
   end record;

   Max_Length_of_Code : constant := 15;

   subtype Nucleotide_Index is Integer range 0 .. Max_Length_of_Code-1;

   type Nucleotide_Set is array (Nucleotide_Index range <>) of Nucleotide;

end Sequence;
