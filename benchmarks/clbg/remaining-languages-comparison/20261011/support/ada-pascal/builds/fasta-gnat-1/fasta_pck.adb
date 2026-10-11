
with Ada.Text_IO; use Ada.Text_IO;

package body Fasta_Pck is

   Line_Length : constant := 60;

   Last        : Natural := 42;

   function Gen_Random (Max : in Real) return Real;
   pragma Inline (Gen_Random);

   function Gen_Random (Max : in Real) return Real is
      IM : constant := 139_968;
      IA : constant :=   3_877;
      IC : constant :=  29_573;
   begin
      Last := (Last * IA + IC) mod IM;
      return (Max * Real (Last)) / Real (IM);
   end Gen_Random;

   procedure Make_Cumulative (Gene_List : in out Aminoacid_Set) is
      C : Real := Gene_List (Gene_List'First).P;
   begin
      for K in Gene_List'First + 1 .. Gene_List'Last loop
         C := C + Gene_List (K).P;
         Gene_List (K).P := C;
      end loop;
   end Make_Cumulative;

   function Select_Random (Gene_List : in Aminoacid_Set) return Character is
      R         : Real := Gen_Random (1.0);
      I, Lo, Hi : Integer;
   begin
      if R < Gene_List (Gene_List'First).P then
         return Gene_List (Gene_List'First).C;
      end if;

      Lo := 0;
      Hi := Gene_List'Last;

      while Hi > Lo + 1 loop
         I := (Hi + Lo) / 2;
         if R < Gene_List (I).P then
            Hi := I;
         else
            Lo := I;
         end if;
      end loop;

      return Gene_List (Hi).C;
   end Select_Random;

   procedure Make_Random_Fasta
     (Id, Desc : in String; Gene_List : in Aminoacid_Set; N : in Positive)
   is
      Todo : Integer := N;
      M    : Integer;
      Pick : String (1 .. Line_Length);
   begin
      Put_Line (">" & Id & ' ' & Desc);

      while Todo > 0 loop
         M := Natural'Min (Todo, Line_Length);

         for K in 1 .. M loop
            Pick (K) := Select_Random (Gene_List);
         end loop;

         Put_Line (Pick (1 .. M));
         Todo := Todo - Line_Length;
      end loop;
   end Make_Random_Fasta;

   procedure Make_Repeat_Fasta (Id, Desc, S : in String; N : in Positive) is
      Todo : Integer := N;
      K    : Positive := S'First;
      M    : Natural;
   begin
      Put_Line (">" & Id & ' ' & Desc);

      while Todo > 0 loop
         M := Natural'Min (Todo, Line_Length);

         while M >= S'Length - K + S'First loop
            Put (S (K .. S'Last));
            M := M - (S'Length - K + S'First);
            K := S'First;
         end loop;

         Put_Line (S (K .. K + M - S'First));
         K := K + M - S'First + 1;

         Todo := Todo - Line_Length;
      end loop;
   end Make_Repeat_Fasta;

end Fasta_Pck;
