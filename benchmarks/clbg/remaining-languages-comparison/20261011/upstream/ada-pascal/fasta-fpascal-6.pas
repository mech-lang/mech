(*
  The Computer Language Benchmarks Game
  https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

  - contributed by Ian Osgood
  - modified by Vincent Snijders
  - modified by Steve Fisher
  - modified by Akira1364
*)

program Fasta;

type
  TGene = record
    Code: Char;
    Prob: Double;
  end;
  PGene = ^TGene;

const
  ALU =
    'GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG' + 'GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA' +
    'CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT' + 'ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA' +
    'GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG' + 'AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC' +
    'AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA';

  SourceALU = ALU + 'GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGA';

  IUB: array[0..14] of TGene = (
    (Code: 'a'; Prob: 0.27), (Code: 'c'; Prob: 0.12), (Code: 'g'; Prob: 0.12), (Code: 't'; Prob: 0.27),
    (Code: 'B'; Prob: 0.02), (Code: 'D'; Prob: 0.02), (Code: 'H'; Prob: 0.02), (Code: 'K'; Prob: 0.02),
    (Code: 'M'; Prob: 0.02), (Code: 'N'; Prob: 0.02), (Code: 'R'; Prob: 0.02), (Code: 'S'; Prob: 0.02),
    (Code: 'V'; Prob: 0.02), (Code: 'W'; Prob: 0.02), (Code: 'Y'; Prob: 0.02)
  );

  HomoSapiens: array[0..3] of TGene = (
    (Code: 'a'; Prob: 0.3029549426680), (Code: 'c'; Prob: 0.1979883004921),
    (Code: 'g'; Prob: 0.1975473066391), (Code: 't'; Prob: 0.3015094502008)
  );

  ALULength = 287;

  Width = 60;

var
  IO: PText;
  TextBuf: array[0..65535] of Byte;
  N: PtrInt = 1000;

procedure FastaRepeat(Reps: PtrInt); inline;
var Here: PtrInt;
begin
  Here := 1;
  repeat
    WriteLn(IO^, SourceALU[Here .. Here + Width - 1]);
    Here += Width;
    if Here > ALULength then Here -= ALULength;
    Reps -= Width;
  until Reps <= Width;
  WriteLn(IO^, SourceALU[Here .. Here + Reps - 1]);
end;

function GenRandom(const Limit: PtrInt): Double; inline;
const
  Seed: PtrInt = 42;
  IM = 139968;
  IA = 3877;
  IC = 29573;
begin
  Seed := (Seed * IA + IC) mod IM;
  GenRandom := Limit * Seed * (1 / IM);
end;

procedure FastaRandom(N: PtrInt; const Genes: PGene; const GenesLen: PtrInt);
var
  SumProb: Double;
  Gene, GenesEnd: PGene;
  Line: String[60] = '000000000000000000000000000000000000000000000000000000000000';
  LineEnd: PChar;

  function ChooseCode: Char; inline;
  var
    R: Double;
    Gene: PGene;
  begin
    R := GenRandom(1);
    Gene := Genes;
    while R >= Gene^.Prob do Gene += 1;
    ChooseCode := Gene^.Code;
  end;

  procedure DoOneLine; inline;
  var P: PChar;
  begin
    P := @Line[1];
    while P <= LineEnd do begin
      P^ := ChooseCode();
      P += 1;
    end;
    WriteLn(IO^, Line);
  end;

begin
  SumProb := 0.0;
  Gene := Genes;
  GenesEnd := Genes + GenesLen;
  while Gene < GenesEnd do begin
    with Gene^ do begin
      SumProb += Prob;
      Prob := SumProb;
    end;
    Gene += 1;
  end;
  LineEnd := @Line[Width];
  while N > Width do begin
    DoOneLine();
    N -= Width;
  end;
  Line[0] := Char(N);
  LineEnd := @Line[N];
  DoOneLine();
end;

begin
  IO := @Output;
  SetTextBuf(IO^, TextBuf);
  if ParamCount = 1 then Val(ParamStr(1), N);
  WriteLn(IO^, '>ONE Homo sapiens alu');
  FastaRepeat(N * 2);
  WriteLn(IO^, '>TWO IUB ambiguity codes');
  FastaRandom(N * 3, @IUB[0], 15);
  WriteLn(IO^, '>THREE Homo sapiens frequency');
  FastaRandom(N * 5, @HomoSapiens[0], 4);
end.
