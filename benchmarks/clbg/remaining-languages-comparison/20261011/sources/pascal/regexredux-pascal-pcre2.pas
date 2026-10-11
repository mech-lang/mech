{ Full regex-redux in Pascal with counted standard PCRE2/JIT FFI. }
{$mode objfpc}{$H+}
{$linklib pcre2-8}
uses SysUtils, ctypes;
type PSize = ^SizeUInt;
function Compile(Pattern: PChar; N: SizeUInt; Options: cuint32; var Error: cint; var At: SizeUInt; Context: Pointer): Pointer; cdecl; external 'pcre2-8' name 'pcre2_compile_8';
function JIT(Code: Pointer; Options: cuint32): cint; cdecl; external 'pcre2-8' name 'pcre2_jit_compile_8';
function MatchData(Code,Context: Pointer): Pointer; cdecl; external 'pcre2-8' name 'pcre2_match_data_create_from_pattern_8';
function Match(Code: Pointer; Subject: PChar; N,Start: SizeUInt; Options: cuint32; Data,Context: Pointer): cint; cdecl; external 'pcre2-8' name 'pcre2_match_8';
function Vector(Data: Pointer): PSize; cdecl; external 'pcre2-8' name 'pcre2_get_ovector_pointer_8';
function ReplaceAll(Code: Pointer; Subject: PChar; N,Start: SizeUInt; Options: cuint32; Data,Context: Pointer; Replacement: PChar; RLen: SizeUInt; Output: PChar; var OLen: SizeUInt): cint; cdecl; external 'pcre2-8' name 'pcre2_substitute_8';
procedure FreeCode(Code: Pointer); cdecl; external 'pcre2-8' name 'pcre2_code_free_8';
procedure FreeData(Data: Pointer); cdecl; external 'pcre2-8' name 'pcre2_match_data_free_8';
function Pattern(const S: AnsiString): Pointer;
var E: cint; At: SizeUInt;
begin
  Result := Compile(PChar(S),Length(S),0,E,At,nil);
  if Result=nil then raise Exception.Create('PCRE2 compile');
  JIT(Result,1);
end;
function Count(const P,S: AnsiString): LongInt;
var Code,Data: Pointer; At: SizeUInt; RC: cint;
begin
  Code := Pattern(P);
  Data := MatchData(Code,nil);
  At := 0;
  Result := 0;
  repeat
    RC := Match(Code,PChar(S),Length(S),At,0,Data,nil);
    if RC<0 then Break;
    At := Vector(Data)[1];
    Inc(Result);
  until At>SizeUInt(Length(S));
  if RC < -1 then raise Exception.Create('PCRE2 match');
  FreeData(Data);
  FreeCode(Code);
end;
procedure Substitute(const P,R: AnsiString; var S: AnsiString);
var Code: Pointer; OutText: AnsiString; Needed: SizeUInt; RC: cint;
begin
  Code := Pattern(P);
  Needed := 0;
  RC := ReplaceAll(Code,PChar(S),Length(S),0,$1100,nil,nil,PChar(R),Length(R),nil,Needed);
  if RC<>-48 then raise Exception.Create('PCRE2 sizing');
  SetLength(OutText,Needed);
  RC := ReplaceAll(Code,PChar(S),Length(S),0,$100,nil,nil,PChar(R),Length(R),PChar(OutText),Needed);
  if RC<0 then raise Exception.Create('PCRE2 substitute');
  SetLength(OutText,Needed);
  S := OutText;
  FreeCode(Code);
end;
const
  Variants: array[0..8] of AnsiString = ('agggtaaa|tttaccct','[cgt]gggtaaa|tttaccc[acg]','a[act]ggtaaa|tttacc[agt]t','ag[act]gtaaa|tttac[agt]ct','agg[act]taaa|ttta[agt]cct','aggg[acg]aaa|ttt[cgt]ccct','agggt[cgt]aa|tt[acg]accct','agggta[cgt]a|t[acg]taccct','agggtaa[cgt]|[acg]ttaccct');
  Substitutions: array[0..4] of AnsiString = ('tHa[Nt]','aND|caN|Ha[DS]|WaS','a[NSt]|BY','<[^>]*>','\|[^|][^|]*\|');
  Replacements: array[0..4] of AnsiString = ('<4>','<3>','<2>','|','-');
var
  S: AnsiString;
  Buffer: array[0..65535] of Byte;
  Got,Used,Capacity,I,Original,Clean: LongInt;
begin
  Used := 0;
  Capacity := 65536;
  SetLength(S,Capacity);
  repeat
    Got := FileRead(StdInputHandle,Buffer,SizeOf(Buffer));
    if Got<=0 then Break;
    if Used+Got>Capacity then begin
      Capacity := Capacity*2;
      SetLength(S,Capacity);
    end;
    Move(Buffer,S[Used+1],Got);
    Inc(Used,Got);
  until False;
  SetLength(S,Used);
  Original := Used;
  Substitute('>.*\n|\n','',S);
  Clean := Length(S);
  for I := 0 to 8 do WriteLn(Variants[I],' ',Count(Variants[I],S));
  for I := 0 to 4 do Substitute(Substitutions[I],Replacements[I],S);
  WriteLn;
  WriteLn(Original);
  WriteLn(Clean);
  WriteLn(Length(S));
end.
