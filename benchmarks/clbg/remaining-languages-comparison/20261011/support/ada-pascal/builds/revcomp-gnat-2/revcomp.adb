--
--  The Computer Language Benchmarks Game
--  https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
--  Contributed by Pascal Obry on 2005/03/19
--  Modified by Bill Findlay  on 2005/04/04
--  Updated by Georg Bauhaus and Jonathan Parker (May 2012)

with Text_Input; use Text_Input;
with Line_IO;

procedure Revcomp is

   Multitasking_Version_Desired : constant Boolean := True;

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

   Max_Line_Length : constant := 60;

   End_Of_Line : constant String := Line_IO.Separator;

   procedure Reverse_Fasta
     (Fasta_Line   : in     String_Access;
      Fasta_Start  : in     Natural;
      Fasta_Finish : in     Natural;
      Bundle       : in out String)
   is
      L : Natural := Bundle'First; -- Leftmost char
      R : Natural := Fasta_Finish; -- Rightmost char
      c0, c1 : Character;
   begin
      if R < Fasta_Start then return; end if;

      c1 := Fasta_Line(R);
      loop
         Bundle(L) := Complement(c1);
         R := R - 1;
         L := L + 1;
         if R > Fasta_Start then
            c0 := Fasta_Line(R);
            c1 := Fasta_Line(R-1);
            Bundle(L) := Complement(c0);
            L := L + 1;
            R := R - 1;
         else
            if R = Fasta_Start then
               Bundle(L) := Complement(Fasta_Line(R));
            end if;
            exit;
         end if;
      end loop;

   end Reverse_Fasta;

   procedure Put_Reversed_Fasta
     (Fasta_Section     : in String_Access;
      Fasta_Data_Length : in Natural)
   is
      Lines_per_Bundle : constant := 2000;
      Line_Feed_Len    : constant Natural := End_Of_Line'Length;
      Line_Bundle : String(1 .. Lines_per_Bundle*(Max_Line_Length + Line_Feed_Len));
      L        : Natural := Fasta_Data_Length;
      B_start  : Natural := Line_Bundle'First;
      B_finish : Natural := B_start + Max_Line_Length - 1;
   begin

      -- Append line feed string (End_Of_Line) to 2000 Line_Bundle lines:

      while L >= Lines_per_Bundle * Max_Line_Length loop
         B_start := Line_Bundle'First;
         for j in 1 .. Lines_per_Bundle loop
            B_finish := B_start + Max_Line_Length - 1;
            Reverse_Fasta
              (Fasta_Line   => Fasta_Section,
               Fasta_Start  => L - Max_Line_Length + 1,
               Fasta_Finish => L,
               Bundle       => Line_Bundle(B_start .. B_finish));
            Line_Bundle(B_finish + 1 .. B_finish + Line_Feed_Len) := End_Of_Line;
            B_start := B_finish + Line_Feed_Len + 1;
            L       := L - Max_Line_Length;
         end loop;
         Line_IO.Put (Line_Bundle);
      end loop;

      while L >= Max_Line_Length loop
         Reverse_Fasta
           (Fasta_Line   => Fasta_Section,
            Fasta_Start  => L - Max_Line_Length + 1,
            Fasta_Finish => L,
            Bundle       => Line_Bundle(1 .. Max_Line_Length));
         Line_IO.Put_Line (Line_Bundle (1..Max_Line_Length));
         L := L - Max_Line_Length;
      end loop;

      if L > 0 then
         Reverse_Fasta
           (Fasta_Line   => Fasta_Section,
            Fasta_Start  => 1,
            Fasta_Finish => L,
            Bundle       => Line_Bundle(1 .. L));
         Line_IO.Put_Line (Line_Bundle (1 .. L));
      end if;

   end Put_Reversed_Fasta;

   procedure Read_Reverse_Write_a_Section_p
     (Job_Is_Complete : out Boolean)
   is
      Section_o_Fasta : String_Pointer (2**20 * 128);
      Header          : String(1..Max_Line_Length) := (others => '?');
      Section_Length  : Natural := 0;
      Header_Length   : Natural := 0;
   begin
      Job_Is_Complete := False;

      Text_Input.Read_Section
        (Data_Buffer     => Section_o_Fasta.Buffer,
         Data_Length     => Section_Length,
         Next_Header     => Header,
         Header_Length   => Header_Length,
         Max_Line_Length => 100); -- use anything >= actual limit of 60.

      if Header_Length < 1 then   -- null Header marks final section.
         Job_Is_Complete := True;
      end if;

      if Section_Length > 0 then
         Put_Reversed_Fasta (Section_o_Fasta.Buffer, Section_Length);
      end if;
      if Header_Length > 0 then
         Line_IO.Put_Line (Header(1..Header_Length));
      end if;

   end Read_Reverse_Write_a_Section_p;

   task type Read_Reverse_Write_a_Section is
      entry Start_Reading;
      entry Done_Reading_Start_Writing (Reached_End_Of_File : out Boolean);
      entry Done_Writing;
      pragma Storage_Size (2**20);
   end Read_Reverse_Write_a_Section;

   task body Read_Reverse_Write_a_Section is
      Section_o_Fasta : String_Pointer (2**20 * 128);
      Header          : String(1..Max_Line_Length) := (others => '?');
      Section_Length  : Natural := 0;
      Header_Length   : Natural := 0;
      Hit_End_Of_File : Boolean := False;
   begin
      loop
      select
         accept Start_Reading;

         Text_Input.Read_Section
           (Data_Buffer     => Section_o_Fasta.Buffer,
            Data_Length     => Section_Length,
            Next_Header     => Header,
            Header_Length   => Header_Length,
            Max_Line_Length => 100); -- use anything >= actual limit of 60.

         if Header_Length < 1 then   -- null Header marks final section.
            Hit_End_Of_File := True;
         end if;

         accept Done_Reading_Start_Writing (Reached_End_Of_File : out Boolean) do
            Reached_End_Of_File := Hit_End_Of_File;
         end Done_Reading_Start_Writing;

         if Section_Length > 0 then
            Put_Reversed_Fasta (Section_o_Fasta.Buffer, Section_Length);
         end if;
         if Header_Length > 0 then
            Line_IO.Put_Line (Header(1..Header_Length));
         end if;

         accept Done_Writing;
      or
         terminate;
      end select;
      end loop;
   end Read_Reverse_Write_a_Section;

   Job_Is_Complete : Boolean;

begin

   if Multitasking_Version_Desired then -- Do computation concurrently with Input

      declare
         type Task_Id_Type is mod 2;
         Do_a_Section : array (Task_Id_Type) of Read_Reverse_Write_a_Section;
         i : Task_Id_Type := Task_Id_Type'First;
         Reached_End_Of_File : Boolean := False;
      begin

         Read_Reverse_Write_a_Section_p (Job_Is_Complete);
         --  All this does is handle the 1st line of the file (the Header).

         Do_a_Section(i).Start_Reading;
         --  Start 1st task reading 1st section.

         loop

            Do_a_Section(i).Done_Reading_Start_Writing (Reached_End_Of_File);
            -- Block here until task i says its done reading the section.
            -- After completion of this rendezvous, task i is unblocked. Task i
            -- then begins computing and writing the reversed data. Task i 
            -- remains unblocked until it finishes writing.

            -- Task i is done reading so we can unblock task i+1 to start reading:
            if not Reached_End_Of_File then
               Do_a_Section(i+1).Start_Reading;
            end if;

            Do_a_Section(i).Done_Writing;
            -- Block here until task i says it's done writing. (If task i+1 were
            -- to write while task i writes, then their output is interleaved.)
            -- Next go to top of loop to unblock task i+1 so that it can write.

            exit when Reached_End_Of_File;
            i := i + 1;

         end loop;

      end;

   else -- Use a Procedure rather than Tasks:

      loop
         Read_Reverse_Write_a_Section_p (Job_Is_Complete);
         exit when Job_Is_Complete;
      end loop;

   end if; -- Multitasking_Version_Desired

   Line_IO.Close;

end Revcomp;
