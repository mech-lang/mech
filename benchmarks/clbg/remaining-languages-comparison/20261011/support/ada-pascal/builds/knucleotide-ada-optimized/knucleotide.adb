-- Native Ada; seven complete, independently growing standard hash maps.
with Ada.Text_IO; use Ada.Text_IO;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Characters.Handling;
with Ada.Containers.Hashed_Maps;
with Interfaces; use Interfaces;
with GNAT.OS_Lib;
procedure KNucleotide is
   function Hash (Key : Unsigned_64) return Ada.Containers.Hash_Type is
   begin
      return Ada.Containers.Hash_Type ((Key xor Shift_Right (Key,32)) and 16#FFFF_FFFF#);
   end Hash;
   package Counts is new Ada.Containers.Hashed_Maps (Unsigned_64, Natural, Hash, "=");
   package Floats is new Float_IO (Long_Float);
   package Numbers is new Integer_IO (Natural);
   Widths : constant array (1..7) of Positive := (1,2,3,4,6,12,18);
   Alphabet : constant String := "ACGT";
   Queries : constant array (3..7) of Unbounded_String := (To_Unbounded_String("GGT"),To_Unbounded_String("GGTA"),To_Unbounded_String("GGTATT"),To_Unbounded_String("GGTATTTTAATT"),To_Unbounded_String("GGTATTTTAATTTATAGT"));
   DNA : Unbounded_String;
   Active : Boolean := False;
   DNALength : Natural;
   Buffer : String(1..65536);
   Got : Integer;
   Maps : array (1..7) of Counts.Map;
   function Base (C : Character) return Unsigned_64 is
   begin
      case C is
         when 'A' => return 0;
         when 'C' => return 1;
         when 'G' => return 2;
         when 'T' => return 3;
         when others => raise Constraint_Error;
      end case;
   end Base;
   function Encode (S : String) return Unsigned_64 is
      Key : Unsigned_64 := 0;
   begin
      for C of S loop Key := Shift_Left(Key,2) or Base(C); end loop;
      return Key;
   end Encode;
   function Decode (Key : Unsigned_64; Width : Positive) return String is
      Result : String (1..Width);
      K : Unsigned_64 := Key;
   begin
      for I in reverse Result'Range loop
         Result(I) := Alphabet(Integer(K and 3)+1);
         K := Shift_Right(K,2);
      end loop;
      return Result;
   end Decode;
   procedure Count (Index : Positive; Data : String) is
      Key : Unsigned_64 := 0;
      Width : constant Positive := Widths(Index);
      Mask : constant Unsigned_64 := Shift_Left(1,2*Width)-1;
      P : Counts.Cursor;
   begin
      Maps(Index).Reserve_Capacity(16);
      for I in Data'Range loop
         Key := (Shift_Left(Key,2) or Unsigned_64(Character'Pos(Data(I)))) and Mask;
         if I >= Width then
            P := Maps(Index).Find(Key);
            if Counts.Has_Element(P) then
               Maps(Index).Replace_Element(P,Counts.Element(P)+1);
            else
               Maps(Index).Insert(Key,1);
            end if;
         end if;
      end loop;
   end Count;
   type Row is record Key : Unsigned_64; Value : Natural; end record;
   type Row_Array is array (Positive range <>) of Row;
begin
   loop
      Got := GNAT.OS_Lib.Read(GNAT.OS_Lib.Standin,Buffer'Address,Buffer'Length);
      exit when Got=0;
      if Got<0 then raise Constraint_Error; end if;
      Append(DNA,Buffer(1..Got));
   end loop;
   declare
      Data : String := To_String(DNA);
      First : Positive := 1;
      Last : Natural;
      Used : Natural := 0;
   begin
      DNA := Null_Unbounded_String;
      while First<=Data'Last loop
         Last := First;
         while Last<=Data'Last and then Data(Last)/=ASCII.LF loop Last := Last+1; end loop;
         if Data(First)='>' then
            exit when Active;
            Active := Last-First>=6 and then Data(First..First+5)=">THREE";
         elsif Active then
            for J in First..Last-1 loop
               if Data(J)/=ASCII.CR then
                  Used := Used+1;
                  Data(Used) := Character'Val(Integer(Base(Ada.Characters.Handling.To_Upper(Data(J)))));
               end if;
            end loop;
         end if;
         First := Last+1;
      end loop;
      DNALength := Used;
      declare
         task type Worker(Index : Positive);
         task body Worker is
         begin
            Count(Index,Data(1..Used));
         end Worker;
         type Worker_Access is access Worker;
         Workers : array(1..7) of Worker_Access;
      begin
         for I in 1..7 loop Workers(I) := new Worker(I); end loop;
      end;
   end;
   for I in 1..2 loop
      declare
         Rows : Row_Array(1..Natural(Maps(I).Length));
         Next : Positive := 1;
         Temp : Row;
      begin
         for P in Maps(I).Iterate loop
            Rows(Next) := (Counts.Key(P),Counts.Element(P));
            Next := Next+1;
         end loop;
         for A in Rows'Range loop
            for B in A+1..Rows'Last loop
               if Rows(B).Value>Rows(A).Value or else (Rows(B).Value=Rows(A).Value and then Rows(B).Key<Rows(A).Key) then
                  Temp := Rows(A);
                  Rows(A) := Rows(B);
                  Rows(B) := Temp;
               end if;
            end loop;
         end loop;
         for R of Rows loop
            Put(Decode(R.Key,Widths(I)) & " ");
            Floats.Put(100.0*Long_Float(R.Value)/Long_Float(DNALength-Widths(I)+1),Fore=>0,Aft=>3,Exp=>0);
            New_Line;
         end loop;
         New_Line;
      end;
   end loop;
   for I in 3..7 loop
      declare
         S : constant String := To_String(Queries(I));
         P : constant Counts.Cursor := Maps(I).Find(Encode(S));
         V : Natural := 0;
      begin
         if Counts.Has_Element(P) then V := Counts.Element(P); end if;
         Numbers.Put(V,Width=>0);
         Put_Line(ASCII.HT & S);
      end;
   end loop;
end KNucleotide;
