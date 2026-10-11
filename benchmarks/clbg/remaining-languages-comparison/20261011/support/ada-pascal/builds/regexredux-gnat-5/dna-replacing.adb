
with Ada.Unchecked_Deallocation;
with U;
package body DNA.Replacing is

   function Find_UB (Est, Ub : Positive) return Positive;
   --  position in Seq safe for splitting

   task type Service
     (Sequence : access String; From, To : Natural; Bordering : Boolean)
     --  Perform substitutions for matches between From and To.  If Bordering,
     --  then use the last of the patterns in DNA.Iub, otherwise the ones
     --  preceding it, in sequence.
   is
      entry Save (Pointer : out U.String_Access; Last : out Natural);
      --  Pointer at new text, which runs up to Last.
   end Service;

   procedure Perform_Replacements
     (Limit : Positive; New_Length : out Natural)
   is
      type Worker is access Service;
      Here  : Positive;
      There : Natural;
      Sz    : constant Positive := (Limit + Number_Of_Tasks) / Number_Of_Tasks;
      Work  : array (1 .. Number_Of_Tasks) of Worker;
   begin
      There := 0;
      for N in 1 .. Number_Of_Tasks loop
         Here     := There + 1;
         There    := Find_UB (N*Sz, Limit);
         Work (N) := new Service
           (Sequence => Seq, From => Here, To => There, Bordering => False);
      end loop;
      --  concatenate the buffers and perform the final replacements in that:
      declare
         Wipe    : Worker;
         Scratch : U.String_Access;
         procedure Free is new Ada.Unchecked_Deallocation (Service, Worker);
      begin
         Here  := 1;
         There := 0;
         for Job of Work loop
            Here := Here + There;
            Job.Save (Scratch, There);
            Seq (Here .. Here+There-1) := Scratch (1 .. There);
            Free (Job);
         end loop;
         Wipe := new Service (Sequence => Seq, From => 1, To => Here+There-1,
                              Bordering => True);
         Wipe.Save (Scratch, There);
      end;
      New_Length := There;
   end Perform_Replacements;

   function Safe_Split (Near : String) return Natural is
      N          : aliased Natural;
      Looking_At : constant Pattern :=
        ((Break ("A") & Setcur (N'Access))
           or
         (Break ("a") & Setcur (N'Access) & "aaa"));
   begin
      if Match (Near, Pat => Looking_At) then
         return Near'First + N;
      end if;
      raise Constraint_Error with "cannot safely split up seq";
   end Safe_Split;

   function Find_UB (Est, Ub : Positive) return Positive is
      Limit : constant Natural  := Positive'Min (Est + 1000, Ub);
   begin
      if Est < Ub then
         return Safe_Split (Seq (Est .. Limit));
      else
         return Ub;
      end if;
   end Find_UB;

   task body Service is
      Sub : array (Boolean) of U.String_Access;  --  flipping buffers
      Rpl : U.String_Access;

      --  framing matches and replacements so far:
      Tail  : Positive;
      Start : Positive;
      Hit   : aliased Natural;
      Stop  : aliased Natural;

      Source : Boolean;

      function Last_Repl return Boolean is
         L : constant Natural := Stop - Hit;
      begin
         Sub (not Source)(Tail .. Tail+L-1) := Sub (Source) (Hit+1 .. Hit+L);
         Tail := Tail+L;
         return True;
      end Last_Repl;

      function Next_Repl return Boolean is
         Dest : U.String_Access renames Sub (not Source);
         L1   : constant Natural := Hit - Start + 1;
      begin
         pragma Assert (Tail'Valid);

         Dest (Tail .. Tail+L1-1)           := Sub (Source) (Start .. Hit);
         Dest (Tail+L1
                 .. Tail+L1+Rpl'Length - 1) := Rpl.all;
         Tail                               := Tail + L1 + Rpl'Length;
         Start                              := Stop + 1;
         return False;
      end Next_Repl;

      procedure Run_Matcher (Iub_Pattern : Pattern; Ub : Positive) is

         function Ge return Boolean is (Hit >= Stop);

         Suffix : constant Pattern :=
           (Tab (Stop'Access)
              & Setcur (Hit'Access)
              & Rest
              & Setcur (Stop'Access)
              & (+Last_Repl'Unrestricted_Access));

         Code : constant Pattern :=
           (Setcur (Hit'Access)
              & (+Ge'Unrestricted_Access)
              & Iub_Pattern
              & Setcur (Stop'Access)
              & (+Next_Repl'Unrestricted_Access));
      begin
         Stop  := 0;
         Tail  := 1;
         Start := Sub (Source)'First;
         Match (Sub (Source) (1 .. Ub), Pat => Code);
         Match (Sub (Source) (1 .. Ub), Pat => Suffix);
      end Run_Matcher;

      Ub : Positive;
      Need : constant Positive := 1 + Natural (1.3 * Float (To-From+1));
   begin                              --  Replacement
      Source := True;
      Ub     := To-From+1;
      if Bordering then         -- Sequence is the concatenation
         Sub (Source)     := Sequence;
         Sub (not Source) := new String (1 .. To-From+1);
         Rpl              := new String'(S (Iub (Iub'Last).Replacement));
         Run_Matcher (Iub (Iub'Last).Element, Ub);
         Ub               := Tail - 1;
         Source           := not Source;
      else
         Sub (True)  := new String (1 .. Need);
         Sub (False) := new String (1 .. Need);
         Sub (Source) (1 .. (To-From+1)) := Sequence (From .. To);
         for Job in Iub'First .. Iub'Last-1 loop
            Rpl    := new String'(S (Iub (Job).Replacement));
            Run_Matcher (Iub (Job).Element, Ub);
            Ub     := Tail - 1;
            Source := not Source;
         end loop;
      end if;
      U.Free (Sub (not Source));
      accept Save (Pointer : out U.String_Access; Last : out Natural) do
         Pointer := Sub (Source);
         Last    := Ub;
      end Save;
   end Service;

end DNA.Replacing;
