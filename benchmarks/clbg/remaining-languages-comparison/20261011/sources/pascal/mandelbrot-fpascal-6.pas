(* The Computer Language Benchmarks Game
   https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

   - contributed by Ales Katona
   - modified by Vincent Snijders
   - optimized and multithreaded by Jean de La Taille
   - modified by Jeppe Johansen
   - modified by Peter Blackman (Restore 'CalculatePoint' as leaf function, better use of registers)
   - refactored and modified to use PasMP by Akira1364
*)

program Mandelbrot;

uses CMem, {$IFDEF UNIX}CThreads,{$ENDIF} SysUtils, Math, PasMP;

const N: Int32 = 200;

var
  DimX: Int32;
  NInv: Double;
  TextBuf: PByte;

  procedure DoMandelbrot(const Job: PPasMPJob;
                         const ThreadIndex: Int32;
                         const Data: Pointer;
                         const FromIndex, ToIndex: SizeInt);
  var
    BufIndex, Bit, Bits, X: Int32;
    Index: SizeInt;
    CR, CI: Double;

    function CalculatePoint(const CX, CY: Double): Boolean; inline;
    var
      I: Int32;
      Limit: Double = 4;
      Two: Double = 2;
      ZR, ZI, TI, TR: Double;
    begin
      ZR := 0; ZI := ZR; TR := ZR; TI := ZR;
      for I := 1 to 50 do begin
        ZI := Two * ZR * ZI + CY;
        ZR := TR - TI + CX;
        TI := ZI * ZI;
        TR := ZR * ZR;
        if (TR + TI >= Limit) then Exit(True);
      end;
      CalculatePoint := False;
    end;

  begin
    for Index := FromIndex to ToIndex do begin
      BufIndex := Index * DimX;
      Prefetch(TextBuf[BufIndex]);
      Bit := 128;
      Bits := 0;
      CI := ((Index + Index) * NInv) - 1.0;
      for X := 0 to Pred(N) do begin
        CR := ((X + X) * NInv) - 1.5;
        if CalculatePoint(CR, CI) then Bits := Bits or Bit;
        Bit := Bit shr 1;
        if Bit = 0 then begin
          TextBuf[BufIndex] := not Bits;
          Inc(BufIndex);
          Bits := 0;
          Bit := 128;
        end;
      end;
    end;
  end;

var
  Len: SizeUInt;
  IO: PText;

begin
  if ParamCount > 0 then Val(ParamStr(1), N);
  DimX := Ceil(N / 8);
  NInv := 1 / N;
  Len := DimX * N;
  GetMem(TextBuf, Len);
  with TPasMP.CreateGlobalInstance() do
    Invoke(ParallelFor(nil, 0, Pred(N), @DoMandelbrot));
  IO := @Output;
  Write(IO^, 'P4', #10, N, ' ', N, #10);
  Flush(IO^);
  FileWrite(StdOutputHandle, TextBuf[0], Len);
  FreeMem(TextBuf);
end.
