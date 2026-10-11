(* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
   - Based on "Go #4" by Martin Koistinen
   - Contributed by Akira1364
*)

program Mandelbrot;

uses CMem, {$ifdef Unix}CThreads,{$ENDIF} SysUtils, Math, MTProcs;

const
  Limit = 4.0;
  Size: PtrInt = 200;

type
  TData = record
    Rows: PByte;
    InitialR, InitialI: PDouble;
  end;

  PData = ^TData;

var
  BytesPerRow: PtrInt;
  Inv: Double;

  procedure RenderRows(Index: PtrInt;
                       UserData: Pointer;
                       Item: TMultiThreadProcItem);
  var
    Res, B: Byte;
    XByte, I, J, X: PtrInt;
    ZRA, ZRB, ZIA, ZIB, TRA, TRB, TIA, TIB, CRA, CRB, CI: Double;
  begin
    CI := TData(UserData^).InitialI[Index];
    for XByte := Pred(BytesPerRow) downto 0 do begin
      Res := 0;
      I := 0;
      repeat
        X := XByte shl 3;
        with TData(UserData^) do begin
          CRA := InitialR[X + I];
          CRB := InitialR[X + I + 1];
        end;
        ZRA := CRA;
        ZIA := CI;
        ZRB := CRB;
        ZIB := CI;
        B := 0;
        for J := 49 downto 0 do begin
          TRA := ZRA * ZRA;
          TIA := ZIA * ZIA;
          if TRA + TIA > Limit then begin
            B := B or 2;
            if B = 3 then Break;
          end;
          TRB := ZRB * ZRB;
          TIB := ZIB * ZIB;
          if TRB + TIB > Limit then begin
            B := B or 1;
            if B = 3 then Break;
          end;
          ZIA := 2 * ZRA * ZIA + CI;
          ZRA := TRA - TIA + CRA;
          ZIB := 2 * ZRB * ZIB + CI;
          ZRB := TRB - TIB + CRB;
        end;
        Res := (Res shl 2) or B;
        I += 2;
      until I = 8;
      TData(UserData^).Rows[(Index * BytesPerRow) + XByte] := not Res;
    end;
  end;

  procedure MakeLookupTables(Index: PtrInt;
                             UserData: Pointer;
                             Item: TMultiThreadProcItem);
  var InvScaled: Double;
  begin
    InvScaled := Inv * Double(Index);
    with TData(UserData^) do begin
      InitialI[Index] := InvScaled - 1.0;
      InitialR[Index] := InvScaled - 1.5;
    end;
  end;

var
  Data: TData;
  IO: PText;

begin
  SetExceptionMask([exInvalidOp, exOverflow, exPrecision]);
  if ParamCount > 0 then Val(ParamStr(1), Size);
  BytesPerRow := Size shr 3;
  with Data do begin
    GetMem(InitialI, SizeOf(Double) * Size);
    GetMem(InitialR, SizeOf(Double) * Size);
    GetMem(Rows, BytesPerRow * Size);
  end;
  Inv := 2.0 / Double(Size);
  with ProcThreadPool do begin
    DoParallel(@MakeLookupTables, 0, Pred(Size), @Data);
    DoParallel(@RenderRows, 0, Pred(Size), @Data);
  end;
  IO := @Output;
  Write(IO^, 'P4', #10, Size, ' ', Size, #10);
  Flush(IO^);
  FileWrite(StdOutPutHandle, Data.Rows[0], BytesPerRow * Size);
  with Data do begin
    FreeMem(InitialI);
    FreeMem(InitialR);
    FreeMem(Rows);
  end;
end.
