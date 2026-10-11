--
-- The Computer Language Benchmarks Game
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
-- contributed by Francois Fabien (juillet 2012)
--
-- regex-dna benchmark using PCRE
-- with concurrent processing (multi-core).
--
-- compile with:
--   gnatmake -O3 -gnatn -gnatp Regexredux.adb
--
pragma Restrictions (No_Abort_Statements);
pragma Restrictions (Max_Select_Alternatives => 0);
pragma Restrictions (Max_Asynchronous_Select_Nesting => 0);
pragma Suppress (All_Checks);

with Text_IO;           use Text_IO;
with Block_Input;       use Block_Input;
with Pcre;              use Pcre;
with DNA;               use DNA;
with System.Task_Info;  use System.Task_Info;
with Ada.Strings.Fixed; use Ada.Strings.Fixed;
with Ada.Strings.Maps;  use Ada.Strings;

procedure Regexredux is

   package Int_IO is new Integer_IO (Integer);

   Max_Size  : constant := 51_000_000; -- max input size
   Input_Str : String_Access := new String (1 .. Max_Size);

   DNA_Code : String_Access;

   Initial_Length, Code_Length, Seq_Length : Natural := 0;

   type Count_Nbrs is range 0 .. 2 ** 15 - 1;
   for Count_Nbrs'Size use 16;

   type Counts_Arrays is array (Variant_Index) of Count_Nbrs;
   Counts : Counts_Arrays;

   Core_Nbr : constant Positive := Number_Of_Processors;

   -- remove headers and ends of line from input file.
   procedure Strip_Input
     (Source, Dest : in String_Access;
      Last_Src     : in Natural;
      Last_Dest    : out Natural)
   is

      EOL        : constant Maps.Character_Set := Maps.To_Set (ASCII.LF);
      End_Of_Cmt : Natural;
      Test_Char  : Character;
      Next, Pos  : Positive                    := 1;
   begin
      while Next <= Last_Src loop
         Test_Char := Source.all (Next);
         case Test_Char is
            when ASCII.LF =>
               Next := Next + 1;
            when '>' =>
               End_Of_Cmt := Index (Source.all (Next .. Next + 100), EOL);
               Next       := End_Of_Cmt + 1;
            when others =>
               Dest.all (Pos) := Test_Char;
               Pos            := Pos + 1;
               Next           := Next + 1;
         end case;
      end loop;
      Last_Dest := Pos - 1;
   end Strip_Input;

   procedure Count_Variant (Var : in Variant_Index; Count : out Count_Nbrs) is
      Regexp       : Pcre_Type;
      Regexp_Extra : Pcre_Extra_type;
      Retcode      : Integer;
      Position     : Natural    := 0;
      L_Count      : Count_Nbrs := 0;
      m0, m1       : Integer;
   begin
      Compile
        (Pattern       => Variant_Labels (Var).all,
         Options       => 0,
         Matcher       => Regexp,
         Matcher_Extra => Regexp_Extra);

      loop
         Match
           (Regexp,
            Regexp_Extra,
            DNA_Code.all (1)'Address,
            Code_Length,
            Position,
            0,
            m0,
            m1,
            Retcode);
         exit when Retcode < 0;
         L_Count  := L_Count + 1;
         Position := m1;
      end loop;
      Free (Regexp);
      Free (Regexp_Extra);
      Count := L_Count;

   end Count_Variant;

   procedure Replace_Variants
     (Source    : in String_Access;
      Last_Src  : in Natural;
      Last_Dest : out Natural)
   is

      Dest_Size    : constant Natural := Integer (Float (Last_Src) * 1.35);
      Dest         : String_Access    := new String (1 .. Dest_Size);
      Regexp       : Pcre_Type;
      Regexp_Extra : Pcre_Extra_type;
      Retcode      : Integer;
      Position     : Integer;
      Char_To_Copy : Integer;
      Start, Stop  : Integer          := 1;
      m0, m1       : Integer;
      IUB          : Mut;
   begin

      Compile
        (Pattern       => "[BDHKMNRSVWY]",
         Options       => 0,
         Matcher       => Regexp,
         Matcher_Extra => Regexp_Extra);

      Position := 0;
      loop
         Match
           (Regexp,
            Regexp_Extra,
            Source.all (1)'Address,
            Last_Src,
            Position,
            0,
            m0,
            m1,
            Retcode);
         exit when Retcode < 0;
         Char_To_Copy         := m0 - Position;
         Stop                 := Start + Char_To_Copy - 1;
         Dest (Start .. Stop) := Source (Position + 1 .. m0);
         -- insert IUB into destination string
         IUB                          := to_Mut (Source.all (m1));
         Start                        := Stop + Iub_Table (IUB).Len + 1;
         Dest (Stop + 1 .. Start - 1) := Iub_Table (IUB).Alt.all;
         Position                     := m1;

      end loop;
      -- copy remaining part of the source
      Char_To_Copy         := Last_Src - Position;
      Stop                 := Start + Char_To_Copy - 1;
      Dest (Start .. Stop) := Source (Position + 1 .. Last_Src);
      Free (Regexp);
      Free (Regexp_Extra);
      Free (Dest);

      Last_Dest := Stop;
   end Replace_Variants;

   procedure Parallel_Job (Cores : in Positive) is

      -- synchronize the variant countings and replacing.
      protected Dispatcher is
         entry Give (Variant : out Natural);
         entry Take (Variant : in Natural; Count : in Count_Nbrs);
         entry Endrep;
         entry Report (Result : out Counts_Arrays);
      private
         Done               : Boolean := False;
         Counted, Replaced  : Boolean := False;
         Cur_Job, Jobs_Done : Natural := 0;
         Var_Counts         : Counts_Arrays;
      end Dispatcher;

      protected body Dispatcher is
         -- assign a job to a worker
         entry Give (Variant : out Natural) when True is
         begin
            if Cur_Job = Var_Size then
               -- when all jobs are given send termination signal to workers
               Variant := 0;
            else
               Cur_Job := Cur_Job + 1;
               Variant := Cur_Job;
            end if;
         end Give;
         -- retrieve the result from a worker task
         entry Take (Variant : in Natural; Count : in Count_Nbrs) when True is
         begin
            Var_Counts (Variant_Index (Variant))  := Count;
            Jobs_Done                             := Jobs_Done + 1;
            if Jobs_Done = Var_Size then -- can transfer the result
               Counted := True;
               Done    := Replaced;-- shortcut for Counted and Replaced
            end if;
         end Take;
         -- signalling end of replacement
         entry Endrep when True is
         begin
            Replaced := True;
            Done     := Counted; -- shortcut for Counted and Replaced
         end Endrep;
         -- when all jobs are completed, give way to the main
         entry Report (Result : out Counts_Arrays) when Done is
         begin
            Result := Var_Counts;
         end Report;

      end Dispatcher;

      -- Each Worker compute a single variant count in parallel
      task type Workers;

      task body Workers is
         Var     : Natural;
         L_Count : Count_Nbrs;
      begin
         Busy : loop
            Dispatcher.Give (Var);
            exit Busy when Var = 0;
            Count_Variant (Variant_Index (Var), L_Count);
            Dispatcher.Take (Var, L_Count);
         end loop Busy;
      end Workers;

      Workshop : array (1 .. Cores) of Workers;

      task Replacer;

      task body Replacer is
      begin
         Replace_Variants (DNA_Code, Code_Length, Seq_Length);
         Dispatcher.Endrep;
      end Replacer;

   begin
      -- wait for the jobs to be completed
      Dispatcher.Report (Counts);
      delay 0.01; -- leave time for tasks termination.
   end Parallel_Job;

begin
   Open_Stdin;
   Read (Input_Str.all, Initial_Length);
   Close_Stdin;

   DNA_Code := new String (1 .. Initial_Length);

   Strip_Input (Input_Str, DNA_Code, Initial_Length, Code_Length);
   Free (Input_Str);

   if Core_Nbr > 1 then
      Parallel_Job (Core_Nbr);
   else
      for V in Variant_Index'Range loop
         Count_Variant (V, Counts (V));
      end loop;
      Replace_Variants (DNA_Code, Code_Length, Seq_Length);
   end if;

   for V in Variant_Index'Range loop
      Put_Line (Variant_Labels (V).all & Count_Nbrs'Image (Counts (V)));
   end loop;

   New_Line;
   Int_IO.Put (Initial_Length, Width => 6);
   New_Line;
   Int_IO.Put (Code_Length, Width => 6);
   New_Line;
   Int_IO.Put (Seq_Length, Width => 6);
   New_Line;

   Free (DNA_Code);
end Regexredux;
------------------------------------------------------------------------------
-- Constants
------------------------------------------------------------------------------
with Unchecked_Conversion;
with Unchecked_Deallocation;

package DNA is

   type String_Access is access all String;
   procedure Free is new Unchecked_Deallocation (String, String_Access);


   Var_Size : constant := 9;
   type Variant_Index is range 1 .. Var_Size;
   for Variant_Index'Size use 8;

   Variant_Labels : constant array (Variant_Index) of String_Access :=
     (new String'("agggtaaa|tttaccct"),
      new String'("[cgt]gggtaaa|tttaccc[acg]"),
      new String'("a[act]ggtaaa|tttacc[agt]t"),
      new String'("ag[act]gtaaa|tttac[agt]ct"),
      new String'("agg[act]taaa|ttta[agt]cct"),
      new String'("aggg[acg]aaa|ttt[cgt]ccct"),
      new String'("agggt[cgt]aa|tt[acg]accct"),
      new String'("agggta[cgt]a|t[acg]taccct"),
      new String'("agggtaa[cgt]|[acg]ttaccct"));

   type Mut is ('B','D','H','K','M','N','R','S','V','W','Y');
   for Mut'Size use Character'Size;

   for Mut use -- map to character values
     ('B' => 66,
      'D' => 68,
      'H' => 72,
      'K' => 75,
      'M' => 77,
      'N' => 78,
      'R' => 82,
      'S' => 83,
      'V' => 86,
      'W' => 87,
      'Y' => 89);

   function to_Mut is new Unchecked_Conversion (
      Source => Character,
      Target => Mut);

   type Iub_Rec is record
      Alt : String_Access;
      Len : Positive;
   end record;

   Iub_Table : constant array (Mut) of Iub_Rec :=
     ((new String'("(c|g|t)"), 7),
      (new String'("(a|g|t)"), 7),
      (new String'("(a|c|t)"), 7),
      (new String'("(g|t)"), 5),
      (new String'("(a|c)"), 5),
      (new String'("(a|c|g|t)"), 9),
      (new String'("(a|g)"), 5),
      (new String'("(c|g)"), 5),
      (new String'("(a|c|g)"), 7),
      (new String'("(a|t)"), 5),
      (new String'("(c|t)"), 5));
end DNA;
-----------------------------------------------------------------------
--  interface to library PCRE : regular expression
-----------------------------------------------------------------------
with System; use System;

package Pcre is

   Pcre_Error : exception;

   type Pcre_Type is private;
   type Pcre_Extra_type is private;

   Null_Pcre       : constant Pcre_Type;
   Null_Pcre_Extra : constant Pcre_Extra_type;

   procedure Compile
     (Pattern       : in String;
      Options       : in Integer;
      Matcher       : out Pcre_Type;
      Matcher_Extra : out Pcre_Extra_type);

   procedure Match
     (Matcher             : in Pcre_Type;
      Matcher_Extra       : in Pcre_Extra_type;
      Subject             : System.Address;
      -- Address of the first element of a string;
      Length, Startoffset : in Integer;
      Options             : in Integer;
      Match_0, Match_1    : out Integer;
      Result              : out Integer);

   procedure Free (M : Pcre_Type);

   procedure Free (M : Pcre_Extra_type);

private

   type Pcre_Type is new System.Address;
   type Pcre_Extra_type is new System.Address;

   Null_Pcre       : constant Pcre_Type       := Pcre_Type (Null_Address);
   Null_Pcre_Extra : constant Pcre_Extra_type :=
      Pcre_Extra_type (Null_Address);

end Pcre;
------------------------------------------------------------------------------
with Interfaces.C.Strings;     use Interfaces.C.Strings;
with Interfaces.C;             use Interfaces.C;
with Ada.Unchecked_Conversion;

package body Pcre is

   pragma Linker_Options ("-lpcre");

   use Interfaces;

   function To_chars_ptr is new Ada.Unchecked_Conversion (
      Address,
      chars_ptr);

   function Pcre_Compile
     (pattern   : chars_ptr;
      options   : Integer;
      errptr    : access chars_ptr;
      erroffset : access Integer;
      tableptr  : chars_ptr)
      return      Pcre_Type;
   pragma Import (C, Pcre_Compile, "pcre_compile");

   function Pcre_Study
     (code    : Pcre_Type;
      options : Integer;
      errptr  : access chars_ptr)
      return    Pcre_Extra_type;
   pragma Import (C, Pcre_Study, "pcre_study");

   function Pcre_Exec
     (code        : Pcre_Type;
      extra       : Pcre_Extra_type;
      subject     : chars_ptr;
      length      : Integer;
      startoffset : Integer;
      options     : Integer;
      ovector     : System.Address;
      ovecsize    : C.int)
      return        Integer;
   pragma Import (C, Pcre_Exec, "pcre_exec");

   procedure Compile
     (Pattern       : in String;
      Options       : in Integer;
      Matcher       : out Pcre_Type;
      Matcher_Extra : out Pcre_Extra_type)
   is
      Regexp       : Pcre_Type;
      Regexp_Extra : Pcre_Extra_type;
      Error_Ptr    : aliased chars_ptr;
      Error_Offset : aliased Integer;
      Pat          : chars_ptr := New_String (Pattern);
   begin
      Regexp :=
         Pcre_Compile
           (Pat,
            Options,
            Error_Ptr'Access,
            Error_Offset'Access,
            Null_Ptr);
      Free (Pat);

      if Regexp = Null_Pcre then
         raise Pcre_Error;
      end if;
      Matcher      := Regexp;
      Regexp_Extra := Pcre_Study (Regexp, 0, Error_Ptr'Access);
      if Regexp_Extra = Null_Pcre_Extra then
         raise Pcre_Error;
      end if;
      Matcher_Extra := Regexp_Extra;
   end Compile;

   procedure Match
     (Matcher             : in Pcre_Type;
      Matcher_Extra       : in Pcre_Extra_type;
      Subject             : System.Address;
      -- Address of the first element of a string;
      Length, Startoffset : in Integer;
      Options             : in Integer;
      Match_0, Match_1    : out Integer;
      Result              : out Integer)
   is
      Vecsize : constant := 3; -- top-level matching

      m : array (0 .. Vecsize - 1) of C.int;
      pragma Convention (C, m);
      pragma Volatile (m); -- used by the C library

      Start  : constant chars_ptr :=
         To_chars_ptr (Subject);
   begin

      Result  :=
         Pcre_Exec
           (Matcher,
            Matcher_Extra,
            Start,
            Length,
            Startoffset,
            Options,
            m (0)'Address,
            C.int (Vecsize));
      Match_0 := Integer (m (0));
      Match_1 := Integer (m (1));

   end Match;

   type Access_Free is access procedure (Item : System.Address);
   Pcre_Free : Access_Free;
   pragma Import (C, Pcre_Free, "pcre_free");

   procedure Free (M : Pcre_Type) is
   begin
      Pcre_Free (System.Address (M));
   end Free;

   procedure Free (M : Pcre_Extra_type) is
   begin
      Pcre_Free (System.Address (M));
   end Free;

end Pcre;
-------------------------------------------------------------------------------
package Block_Input is

   procedure Read (Item : in out String; Last : out Natural);

   procedure Open_Stdin;

   procedure Close_Stdin;

   pragma Inline (Read);

end Block_Input;
------------------------------------------------------------------------------
with Ada.Streams.Stream_IO;
with Unchecked_Conversion;

package body Block_Input is

   use Ada.Streams;

   Stdin : Stream_IO.File_Type;

   procedure Read (Item : in out String; Last : out Natural) is

      Block_Size : constant := 16 * 1024;

      subtype Index is Stream_Element_Offset range
         Stream_Element_Offset (1) .. Stream_Element_Offset (Block_Size);
      subtype XString is String (1 .. Block_Size);
      subtype XBytes is Stream_Element_Array (Index);
      function To_String is new Unchecked_Conversion (
         Source => XBytes,
         Target => XString);

      One_Block : XBytes;
      Str_Block : XString;
      Final     : Stream_Element_Offset;
      Start     : Natural := Item'First;
      Stop      : Natural;
   begin
      while not Stream_IO.End_Of_File (Stdin) loop
         Stream_IO.Read (Stdin, One_Block, Final);
         Str_Block            := To_String (One_Block);
         Stop                 := Start + Natural (Final) - 1;
         Item (Start .. Stop) := Str_Block (1 .. Natural (Final));
         Start                := Stop + 1;
      end loop;
      Last := Stop;
   end Read;

   procedure Open_Stdin is
   begin
      Stream_IO.Open
        (File => Stdin,
         Mode => Stream_IO.In_File,
         Name => "/dev/stdin");
   end Open_Stdin;

   procedure Close_Stdin is
   begin
      Stream_IO.Close (Stdin);
   end Close_Stdin;

end Block_Input;
