
with LCG_Random;
with Line_IO;

package body Sequence.Creation is

   package Real_Random_Nums is new LCG_Random (Real);
   use Real_Random_Nums;
   
   overriding
   procedure Initialize (Active : in out Environment) is
   begin
      Line_IO.Open_Stdout;
   end Initialize;
   
   overriding
   procedure Finalize (Active : in out Environment) is
   begin
      Line_IO.Close_Stdout;
   end Finalize;


   Line_Length : constant := 60;
   End_of_Line : String renames Line_IO.End_of_Line;

   subtype Line_End_Positions is Positive
      range Line_Length + 1 .. Line_Length + End_of_Line'Length;

   Line_Buffer : String (1 .. Line_Length + End_of_Line'Length);

   Nucleo_Cumulative : array (Nucleotide_Index) of Nucleotide;

   procedure Make_Random_Fasta
     (Title       : in String; 
      Nucleotides : in Nucleotide_Set; 
      N           : in Positive)
   is
      function Random_Nucleotide return Character is
         r : constant Real := Gen_Random (1.0);
      begin
         Choose_Random: 
         for n of Nucleo_Cumulative loop
            if n.P > r then
               return n.C;
            end if;
         end loop Choose_Random;
         return 'J';
      end Random_Nucleotide;

      Sum  : Real := 0.0;
      Remaining_Chars  : constant Natural := N mod Line_Length;
      No_of_Full_Lines : constant Natural := N  /  Line_Length;
   begin
      Line_IO.Print (Title & End_of_Line);

      for k in Nucleotides'Range loop
         Nucleo_Cumulative(k).C := Nucleotides(k).C;
         Sum := Sum + Nucleotides(k).P;
         Nucleo_Cumulative(k).P := Sum;
      end loop;

      Line_Buffer(Line_End_Positions) := End_of_Line;

      for k in 1 .. No_of_Full_Lines loop
         for i in 1 .. Line_Length loop
            Line_Buffer(i) := Random_Nucleotide;
         end loop;
         Line_IO.Print (Line_Buffer);
      end loop;

      if Remaining_Chars > 0 then
         for i in 1 .. Remaining_Chars loop
            Line_Buffer(i) := Random_Nucleotide;
         end loop;
         Line_IO.Print (Line_Buffer(1 .. Remaining_Chars) & End_of_Line);
      end if;

   end Make_Random_Fasta;

   procedure Make_Repeat_Fasta 
     (Title : in String;
      S     : in String; 
      N     : in Positive) 
   is
      S_App : constant String := S & S(S'First .. S'First + Line_Length);

      Pos : Positive := S_App'First;
      Remaining_Chars : Natural := N;
      No_of_Chars_Output : Natural := 0;
   begin
      Line_IO.Print (Title & End_of_Line);

      while Remaining_Chars > 0 loop

         No_of_Chars_Output := Integer'Min (Remaining_Chars, Line_Length);

         Line_IO.Print (S_App (Pos .. Pos + No_of_Chars_Output - 1));
         Line_IO.Print (End_of_Line);

         Remaining_Chars := Remaining_Chars - No_of_Chars_Output;

         Pos := Pos + No_of_Chars_Output;
         if Pos > S'Last then
            Pos := Pos - S'Length;
         end if;

      end loop;

   end Make_Repeat_Fasta;

end Sequence.Creation;
