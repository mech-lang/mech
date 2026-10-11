
with Ada.Text_IO;          use Ada.Text_IO;
with Ada.Integer_Text_IO;  use Ada.Integer_Text_IO;
with GNAT.Spitbol;         use GNAT.Spitbol;

with DNA.Matching;
with DNA.Replacing;        use DNA;
with Block_Input;
with Preprocessing;

with U;

procedure Regexredux is

   Initial_Length, Code_Length, Processed_Length : Natural;
   Input_Text                                    : U.String_Access;

begin  -- Regexredux

   Ada.Integer_Text_IO.Default_Width := 1; --  format output number display

   --  Read FASTA Sequence
   Block_Input.Open_Stdin;
   Input_Text := Block_Input.Read;
   Block_Input.Close_Stdin;

   Initial_Length := Input_Text'Length;

   DNA.Seq := new String (1 .. Initial_Length);
   --  remove unwanted elements
   declare
      Cleaner : Preprocessing.Removal (Input_Text);
   begin
      Cleaner.Run (Clean => Seq);
      Cleaner.Done (Last => Code_Length);
      U.Free (Input_Text);
   end;

   DNA.Matching.Count_Matches (Seq, Limit => Code_Length);

   --  print counts for patterns
   for Variant in Variant_Index loop
      Put (S (Variant_Labels (Variant)) & " ");
      Put (Item => DNA.Matching.Get (Variant));
      New_Line;
   end loop;

   --  perform replacements and get the new length
   DNA.Replacing.Perform_Replacements
     (Limit      => Code_Length,
      New_Length => Processed_Length);

   New_Line;
   Put (Item => Initial_Length);
   New_Line;
   Put (Item => Code_Length);
   New_Line;
   Put (Item => Processed_Length);
   New_Line;

end Regexredux;
