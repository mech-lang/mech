{ Native Free Pascal; complete seven-width growable dictionaries. }
{$mode objfpc}{$H+}
uses SysUtils, Generics.Collections;
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
begin
  SetTextBuf(Output,OutBuf);
  while not EOF(Input) do begin
    ReadLn(Line);
    if (Length(Line)>0) and (Line[1]='>') then begin
      if Active then Break;
      Active := Copy(Line,1,6)='>THREE';
    end else if Active then DNA := DNA + UpperCase(TrimRight(Line));
  end;
  N := Length(DNA);
  for I := 0 to 6 do begin
    Maps[I] := TCounts.Create(16);
    K := Widths[I];
    Mask := (QWord(1) shl (2*K))-1;
    Code := 0;
    for J := 1 to N do begin
      Code := ((Code shl 2) or QWord(Pos(DNA[J],Alphabet)-1)) and Mask;
      if J>=K then begin
        if not Maps[I].TryGetValue(Code,V) then V := 0;
        Maps[I].AddOrSetValue(Code,V+1);
      end;
    end;
  end;
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
