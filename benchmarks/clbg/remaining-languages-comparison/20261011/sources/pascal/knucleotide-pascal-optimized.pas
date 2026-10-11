{ Native Free Pascal; complete seven-width growable dictionaries. }
{$mode objfpc}{$H+}
uses CThreads, CMem, SysUtils, Generics.Collections, MTProcs;
type
  TCounts = specialize TDictionary<QWord, LongInt>;
  TEntry = record Key: QWord; Count: LongInt; end;
const
  Widths: array[0..6] of LongInt = (1,2,3,4,6,12,18);
  Queries: array[2..6] of AnsiString = ('GGT','GGTA','GGTATT','GGTATTTTAATT','GGTATTTTAATTTATAGT');
  Alphabet = 'ACGT';
var
  DNA, Line: AnsiString;
  Active: Boolean = False;
  Maps: array[0..6] of TCounts;
  I, J, K, V, N, A, B: LongInt;
  Code, Mask, Key: QWord;
  Rows: array of TEntry;
  Temp: TEntry;
  OutBuf: array[0..65535] of Byte;
  Raw: AnsiString;
  ReadBuf: array[0..65535] of Byte;
  Got,Used,Capacity,Start,Finish: LongInt;
function Encode(const S: AnsiString): QWord;
var C: Char;
begin
  Result := 0;
  for C in S do Result := (Result shl 2) or QWord(Pos(C,Alphabet)-1);
end;
function Decode(Key: QWord; Width: LongInt): AnsiString;
begin
  SetLength(Result,Width);
  while Width > 0 do begin
    Result[Width] := Alphabet[(Key and 3)+1];
    Key := Key shr 2;
    Dec(Width);
  end;
end;
procedure CountMaps(Index: PtrInt; Data: Pointer; Item: TMultiThreadProcItem);
var I,J,K,V: LongInt; Code,Mask: QWord;
begin
  for I := Index to Index do begin
    Maps[I] := TCounts.Create(16);
    K := Widths[I];
    Mask := (QWord(1) shl (2*K))-1;
    Code := 0;
    for J := 1 to N do begin
      Code := ((Code shl 2) or QWord(Byte(DNA[J]))) and Mask;
      if J>=K then begin
        if not Maps[I].TryGetValue(Code,V) then V := 0;
        Maps[I].AddOrSetValue(Code,V+1);
      end;
    end;
  end;
end;
begin
  SetTextBuf(Output,OutBuf);
  Used := 0;
  Capacity := 65536;
  SetLength(Raw,Capacity);
  repeat
    Got := FileRead(StdInputHandle,ReadBuf,SizeOf(ReadBuf));
    if Got<=0 then Break;
    if Used+Got>Capacity then begin
      Capacity := Capacity*2;
      SetLength(Raw,Capacity);
    end;
    Move(ReadBuf,Raw[Used+1],Got);
    Inc(Used,Got);
  until False;
  SetLength(Raw,Used);
  Start := 1;
  N := 0;
  while Start<=Used do begin
    Finish := Start;
    while (Finish<=Used) and (Raw[Finish]<>#10) do Inc(Finish);
    if Raw[Start]='>' then begin
      if Active then Break;
      Active := Copy(Raw,Start,6)='>THREE';
    end else if Active then
      for J := Start to Finish-1 do
        if Raw[J]<>#13 then begin
          Inc(N);
          Raw[N] := Char(Pos(UpCase(Raw[J]),Alphabet)-1);
        end;
    Start := Finish+1;
  end;
  SetLength(Raw,N);
  DNA := Raw;
  N := Length(DNA);
  ProcThreadPool.MaxThreadCount := 8;
  ProcThreadPool.DoParallel(@CountMaps,0,6,nil);
  for I := 0 to 1 do begin
    SetLength(Rows,Maps[I].Count);
    J := 0;
    for Key in Maps[I].Keys do begin
      Rows[J].Key := Key;
      Rows[J].Count := Maps[I][Key];
      Inc(J);
    end;
    for A := 0 to High(Rows)-1 do
      for B := A+1 to High(Rows) do
        if (Rows[B].Count>Rows[A].Count) or ((Rows[B].Count=Rows[A].Count) and (Rows[B].Key<Rows[A].Key)) then begin
          Temp := Rows[A];
          Rows[A] := Rows[B];
          Rows[B] := Temp;
        end;
    for Temp in Rows do WriteLn(Decode(Temp.Key,Widths[I]),' ',100.0*Temp.Count/(N-Widths[I]+1):0:3);
    WriteLn;
  end;
  for I := 2 to 6 do begin
    if not Maps[I].TryGetValue(Encode(Queries[I]),V) then V := 0;
    WriteLn(V,#9,Queries[I]);
  end;
  for I := 0 to 6 do Maps[I].Free;
end.
