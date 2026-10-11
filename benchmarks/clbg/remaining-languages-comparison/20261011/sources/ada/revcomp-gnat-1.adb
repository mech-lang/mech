--
--  The Computer Language Benchmarks Game
--  https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
-- Contributed by Pascal Obry on 2005/03/19
-- Optimized by Bill Findlay on 2005/04/04

with Ada.Text_IO;                use Ada.Text_IO;
with Ada.Unchecked_Deallocation;

procedure Revcomp is

   Complement : constant array (Character) of Character :=
                  ('A' => 'T', 'C' => 'G', 'G' => 'C', 'T' => 'A', 'U' => 'A',
                   'M' => 'K', 'R' => 'Y', 'W' => 'W', 'S' => 'S', 'Y' => 'R',
                   'K' => 'M', 'V' => 'B', 'H' => 'D', 'D' => 'H', 'B' => 'V',
                   'N' => 'N',
                   'a' => 'T', 'c' => 'G', 'g' => 'C', 't' => 'A', 'u' => 'A',
                   'm' => 'K', 'r' => 'Y', 'w' => 'W', 's' => 'S', 'y' => 'R',
                   'k' => 'M', 'v' => 'B', 'h' => 'D', 'd' => 'H', 'b' => 'V',
                   'n' => 'N',
                   others => '?');

   Max_Length : constant := 60;

   type String_Access is access String;

   procedure Free is new Ada.Unchecked_Deallocation (String, String_Access);

   procedure Get_Fasta (Seq : in out String_Access; Length : out Natural) is
      Ptr  : String_Access;
      J, K : Natural := 0;
      C    : Character;
      EOL  : Boolean;
   begin
      Length := 0;

      loop
         Look_Ahead (C, EOL);
         exit when C /= '>' and C /= ';' ;
         Get_Line (Seq (1 .. Seq'Length), J);
         if Seq (1) = '>' then
            Put_Line (Seq (1 .. J));
         end if;
      end loop;

      loop
         if Length + Max_Length > Seq'Length then
            Ptr := Seq;
            Seq := new String (1 .. 2 * Seq'Length);
            Seq (1 .. Length) := Ptr (1 .. Length);
            Free (Ptr);
         end if;

         Get_Line (Seq (Length + 1 .. Length + Max_Length), J);
         K := J - Length;
         Length := J;

         if K = Max_Length then
            Skip_Line;
         end if;

         Look_Ahead (C, EOL);

         if K < Max_Length or else C = '>' or else C = ';' then
            return;
         end if;
      end loop;
   exception
      when End_Error =>
         return;
   end Get_Fasta;

   procedure Reverse_Fasta (Fasta : in out String) is
      C    : Character;
      L, R : Natural;
   begin
      for I in 0 .. Fasta'Length loop
         L := Fasta'First + I;
         R := Fasta'Last - I;
         exit when L > R;
         C := Fasta (L);
         Fasta (L) := Complement (Fasta (R));
         Fasta (R) := Complement (C);
      end loop;
   end Reverse_Fasta;

   procedure Put_Reversed_Fasta (Fasta : in out String) is
      L : Natural := Fasta'Last;
   begin
      while L >= Max_Length loop
         Reverse_Fasta (Fasta (L - Max_Length + 1 .. L));
         Put_Line (Fasta (L - Max_Length + 1 .. L));
         L := L - Max_Length;
      end loop;
      if L > 0 then
         Reverse_Fasta (Fasta (1 .. L));
         Put_Line (Fasta (1 .. L));
      end if;
   end Put_Reversed_Fasta;

   Seq    : String_Access := new String (1 .. 1_024);
   Length : Natural;

begin
   loop
      Get_Fasta (Seq, Length);
      exit when Length = 0;
      Put_Reversed_Fasta (Seq (1 .. Length));
   end loop;
end Revcomp;
