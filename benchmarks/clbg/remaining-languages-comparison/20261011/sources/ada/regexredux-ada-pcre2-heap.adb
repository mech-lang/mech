-- Full regex-redux; standard heap strings and borrowed views avoid stack copies.
-- Every application PCRE2/JIT binding and buffer callback counts toward LOC.
with Ada.Text_IO; use Ada.Text_IO;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Ada.Strings.Unbounded.Aux;
with Interfaces.C;
with GNAT.OS_Lib;
with System; use System;
procedure Regexredux is
   package C renames Interfaces.C;
   package Aux renames Ada.Strings.Unbounded.Aux;
   use type C.int;
   use type C.size_t;
   pragma Linker_Options ("-lpcre2-8");
   type Offsets is array (0..1) of C.size_t;
   type Offsets_Access is access all Offsets;
   function Compile(P : Address; Length : C.size_t; Options : C.unsigned; Error : access C.int; Offset : access C.size_t; Context : Address) return Address;
   pragma Import(C,Compile,"pcre2_compile_8");
   function JIT(Code : Address; Options : C.unsigned) return C.int;
   pragma Import(C,JIT,"pcre2_jit_compile_8");
   function New_Data(Code,Context : Address) return Address;
   pragma Import(C,New_Data,"pcre2_match_data_create_from_pattern_8");
   function Match(Code,Subject : Address; Length,Start : C.size_t; Options : C.unsigned; Data,Context : Address) return C.int;
   pragma Import(C,Match,"pcre2_match_8");
   function Vector(Data : Address) return Offsets_Access;
   pragma Import(C,Vector,"pcre2_get_ovector_pointer_8");
   function Substitute_All(Code,Subject : Address; Length,Start : C.size_t; Options : C.unsigned; Data,Context,Replacement : Address; Replacement_Length : C.size_t; Output : Address; Output_Length : access C.size_t) return C.int;
   pragma Import(C,Substitute_All,"pcre2_substitute_8");
   procedure Free_Code(Code : Address);
   pragma Import(C,Free_Code,"pcre2_code_free_8");
   procedure Free_Data(Data : Address);
   pragma Import(C,Free_Data,"pcre2_match_data_free_8");
   function Pattern(P : String) return Address is
      Error : aliased C.int;
      Offset : aliased C.size_t;
      Code : constant Address := Compile(P'Address,C.size_t(P'Length),0,Error'Access,Offset'Access,Null_Address);
      Result : C.int;
   begin
      if Code=Null_Address then raise Constraint_Error; end if;
      Result := JIT(Code,1);
      if Result<0 then raise Constraint_Error; end if;
      return Code;
   end Pattern;
   function Count(P : String; S : Unbounded_String) return Natural is
      Subject : Aux.Big_String_Access;
      Subject_Length : Natural;
      Code : constant Address := Pattern(P);
      Data : constant Address := New_Data(Code,Null_Address);
      Offset : C.size_t := 0;
      RC : C.int;
      Total : Natural := 0;
   begin
      Aux.Get_String(S,Subject,Subject_Length);
      loop
         RC := Match(Code,Subject.all'Address,C.size_t(Subject_Length),Offset,0,Data,Null_Address);
         exit when RC<0;
         Offset := Vector(Data)(1);
         Total := Total+1;
      end loop;
      if RC < -1 then raise Constraint_Error; end if;
      Free_Data(Data);
      Free_Code(Code);
      return Total;
   end Count;
   function Substitute(P,R : String; S : Unbounded_String) return Unbounded_String is
      Code : constant Address := Pattern(P);
      Subject : Aux.Big_String_Access;
      Subject_Length : Natural;
      Needed : aliased C.size_t := 0;
      RC : C.int;
      Result : Unbounded_String;
      procedure Fill(Output : out String) is
      begin
         RC := Substitute_All(Code,Subject.all'Address,C.size_t(Subject_Length),0,16#100#,Null_Address,Null_Address,R'Address,C.size_t(R'Length),Output'Address,Needed'Access);
         if RC<0 then raise Constraint_Error; end if;
      end Fill;
   begin
      Aux.Get_String(S,Subject,Subject_Length);
      RC := Substitute_All(Code,Subject.all'Address,C.size_t(Subject_Length),0,16#1100#,Null_Address,Null_Address,R'Address,C.size_t(R'Length),Null_Address,Needed'Access);
      if RC /= -48 then raise Constraint_Error; end if;
      Aux.Set_String(Result,Positive(Needed),Fill'Access);
      Delete(Result,Integer(Needed)+1,Length(Result));
      Free_Code(Code);
      return Result;
   end Substitute;
   Variants : constant array(1..9) of Unbounded_String := (To_Unbounded_String("agggtaaa|tttaccct"),To_Unbounded_String("[cgt]gggtaaa|tttaccc[acg]"),To_Unbounded_String("a[act]ggtaaa|tttacc[agt]t"),To_Unbounded_String("ag[act]gtaaa|tttac[agt]ct"),To_Unbounded_String("agg[act]taaa|ttta[agt]cct"),To_Unbounded_String("aggg[acg]aaa|ttt[cgt]ccct"),To_Unbounded_String("agggt[cgt]aa|tt[acg]accct"),To_Unbounded_String("agggta[cgt]a|t[acg]taccct"),To_Unbounded_String("agggtaa[cgt]|[acg]ttaccct"));
   Substitutions : constant array(1..5) of Unbounded_String := (To_Unbounded_String("tHa[Nt]"),To_Unbounded_String("aND|caN|Ha[DS]|WaS"),To_Unbounded_String("a[NSt]|BY"),To_Unbounded_String("<[^>]*>"),To_Unbounded_String("\|[^|][^|]*\|"));
   Replacements : constant array(1..5) of Unbounded_String := (To_Unbounded_String("<4>"),To_Unbounded_String("<3>"),To_Unbounded_String("<2>"),To_Unbounded_String("|"),To_Unbounded_String("-"));
   Text : Unbounded_String;
   Buffer : String(1..65536);
   Got : Integer;
   Original,Clean : Natural;
   package Numbers is new Integer_IO(Natural);
begin
   loop
      Got := GNAT.OS_Lib.Read(GNAT.OS_Lib.Standin,Buffer'Address,Buffer'Length);
      exit when Got=0;
      if Got<0 then raise Constraint_Error; end if;
      Append(Text,Buffer(1..Got));
   end loop;
   Original := Length(Text);
   Text := Substitute(">.*\n|\n","",Text);
   Clean := Length(Text);
   for P of Variants loop
      Put(To_String(P) & " ");
      Numbers.Put(Count(To_String(P),Text),Width=>0);
      New_Line;
   end loop;
   for I in 1..5 loop
      Text := Substitute(To_String(Substitutions(I)),To_String(Replacements(I)),Text);
   end loop;
   New_Line;
   Numbers.Put(Original,Width=>0);
   New_Line;
   Numbers.Put(Clean,Width=>0);
   New_Line;
   Numbers.Put(Length(Text),Width=>0);
   New_Line;
end Regexredux;
