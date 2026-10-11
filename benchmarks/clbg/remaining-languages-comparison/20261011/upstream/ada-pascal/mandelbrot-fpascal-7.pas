(* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
   - Based on "Go #4" by Martin Koistinen
   - Contributed by Akira1364
*)

program Mandelbrot;

uses CMem, {$IFDEF UNIX}CThreads,{$ENDIF} SysUtils, Math, PasMP;

const
  Limit = 4.0;
  MaxIter = 50;
  Size: Int32 = 200;

var
  BytesPerRow: Int32;
  Rows: array of array of Byte;
  Initial_R, Initial_I: array of Double;

  procedure RenderRows(const Job: PPasMPJob;
                       const ThreadIndex: Int32;
                       const Data: Pointer;
                       const FromIndex, ToIndex: SizeInt); inline;
  var
    Res, B: Byte;
    XByte, I, J, X: Int32;
    Index: SizeInt;
    ZRA, ZRB, ZIA, ZIB, TRA, TRB,
    TIA, TIB, CRA, CRB, CI: Double;
  begin
    for Index := FromIndex to ToIndex do begin
      SetLength(Rows[Index], BytesPerRow);
      for XByte := 0 to Pred(BytesPerRow) do begin
        Res := 0;
        I := 0;
        CI := Initial_I[Index];
        repeat
          X := XByte shl 3;
          CRA := Initial_R[X + I];
          CRB := Initial_R[X + I + 1];
          ZRA := CRA;
          ZIA := CI;
          ZRB := CRB;
          ZIB := CI;
          B := 0;
          for J := 0 to Pred(MaxIter) do begin
            TRA := ZRA * ZRA;
            TIA := ZIA * ZIA;
            ZIA := 2 * ZRA * ZIA + CI;
            ZRA := TRA - TIA + CRA;
            if TRA + TIA > Limit then begin
              B := B or 2;
              if B = 3 then Break;
            end;
            TRB := ZRB * ZRB;
            TIB := ZIB * ZIB;
            ZIB := 2 * ZRB * ZIB + CI;
            ZRB := TRB - TIB + CRB;
            if TRB + TIB > Limit then begin
              B := B or 1;
              if B = 3 then Break;
            end;
          end;
          Res := (Res shl 2) or B;
          I += 2;
        until I = 8;
        Rows[Index][XByte] := not Res;
      end;
    end;
  end;

var
  Inv, InvScaled: Double;
  XY, Y: SizeInt;
  IO: PText;

begin
  SetExceptionMask([exInvalidOp, exOverflow, exPrecision]);
  if ParamCount > 0 then Val(ParamStr(1), Size);
  SetLength(Initial_R, Size);
  SetLength(Initial_I, Size);
  Inv := 2.0 / Double(Size);
  for XY := 0 to Pred(Size) do begin
    InvScaled := Inv * Double(XY);
    Initial_R[XY] := InvScaled - 1.5;
    Initial_I[XY] := InvScaled - 1.0;
  end;
  BytesPerRow := Size shr 3;
  SetLength(Rows, Size);
  with TPasMP.CreateGlobalInstance() do
    Invoke(ParallelFor(nil, 0, Pred(Size), @RenderRows));
  IO := @Output;
  Write(IO^, 'P4', #10, Size, ' ', Size, #10);
  Flush(IO^);
  for Y := 0 to Pred(Size) do
    FileWrite(StdOutputHandle, Rows[Y][0], BytesPerRow);
end.
