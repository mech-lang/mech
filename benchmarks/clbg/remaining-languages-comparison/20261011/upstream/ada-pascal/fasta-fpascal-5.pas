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
    Prob: Double;
    Code: Char;
  end;
  PGene = ^TGene;

const
  ALU =
    'GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGG' + 'GAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAGA' +
    'CCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAAT' + 'ACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCCA' +
    'GCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGG' + 'AGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC' +
    'AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA';

  SourceALU = ALU + 'GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGA';

  Codes = 'acgtBDHKMNRSVWY';

  IUB: array[0..14] of Double = (
    0.27, 0.12, 0.12, 0.27, 0.02, 0.02, 0.02, 0.02,
    0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02
  );

  HomoSap: array[0..3] of Double = (
    0.3029549426680, 0.1979883004921, 0.1975473066391, 0.3015094502008
  );

  ALULength = 287;

  Width = 60;

var
  Genes: array[0..14] of TGene = (
    (Prob: 0.0; Code: #0), (Prob: 0.0; Code: #0), (Prob: 0.0; Code: #0), (Prob: 0.0; Code: #0),
    (Prob: 0.0; Code: #0), (Prob: 0.0; Code: #0), (Prob: 0.0; Code: #0), (Prob: 0.0; Code: #0),
    (Prob: 0.0; Code: #0), (Prob: 0.0; Code: #0), (Prob: 0.0; Code: #0), (Prob: 0.0; Code: #0),
    (Prob: 0.0; Code: #0), (Prob: 0.0; Code: #0), (Prob: 0.0; Code: #0)
  );

  Line: String[60] = '000000000000000000000000000000000000000000000000000000000000';

  IO: PText;
  TextBuf: array[0..65535] of Byte;
  N: PtrInt = 1000;

procedure FastaRepeat(Reps: PtrInt);
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

procedure InitGenes(const Probs: PDouble; const ProbsLen: PtrInt);
var
  I: PtrInt;
  SumProb: Double;
  P: PGene;
begin
  SumProb := 0.0;
  P := @Genes[0];
  for I := 0 to Pred(ProbsLen) do begin
    SumProb += (Probs + I)^;
    with P^ do begin
      Prob := SumProb;
      Code := Codes[Succ(I)];
    end;
    Inc(P);
  end;
end;

procedure FastaRandom(N: PtrInt; const Probs: PDouble; const ProbsLen: PtrInt);

  function ChooseCode: Char; inline;
  var
    R: Double;
    Gene: PGene;
  begin
    R := GenRandom(1);
    Gene := @Genes[0];
    while R >= Gene^.Prob do Inc(Gene);
    ChooseCode := Gene^.Code;
  end;

  procedure DoOneLine(const Size: PtrInt); inline;
  var P, PLimit: PChar;
  begin
    P := @Line[1];
    PLimit := @Line[Size];
    while P <= PLimit do begin
      P^ := ChooseCode();
      Inc(P);
    end;
    WriteLn(IO^, Line);
  end;

begin
  InitGenes(Probs, ProbsLen);
  while N > Width do begin
    DoOneLine(Width);
    N -= Width;
  end;
  Line[0] := Char(N);
  DoOneLine(N);
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
  FastaRandom(N * 5, @HomoSap[0], 4);
end.
