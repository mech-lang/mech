{ The Computer Language Benchmarks Game
  https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

  contributed by Vincent Snijders
}

{$mode objfpc}


program pidigits;

type
  { TBigInt }

  PBigInt = ^TBigInt;

  { TBigInt }

  TBigInt = class
  private
    Digit: pdword;
    FSize: integer;
    Capacity: integer;
    FNextFree: TBigInt; // used to maintain the freelist
    procedure Clear;
    procedure Resize(NewSize: integer);
    function IsNegative: boolean; inline;
    function IsZero: boolean; inline;
    procedure Negate;
  public
    constructor Create(InitialSize: integer);
    destructor Destroy; override;
    function GetDigit(i: integer) : DWord; inline;
  end;

type
  TBigMatrix = array[1..2,1..2] of TBigInt;
  TIntMatrix = array[1..2,1..2] of integer;

var
  BigIntStack: PBigInt;
  BigIntStackLen: integer;
  BigIntTop: integer;
  FirstFreeBigInt: TBigInt;

{ BigInt memory management }

procedure FreeBigInts;
var
  Next: TBigInt;
begin
  while assigned(FirstFreeBigInt) do begin
    Next := FirstFreeBigInt.FNextFree;
    FirstFreeBigInt.Free;
    FirstFreeBigInt := Next;
  end;
end;

function GetBigInt(Size: integer; DoClear: boolean = true) : TBigInt;
var
  Current, Previous: TBigInt;
begin
  if assigned(FirstFreeBigInt) then begin
    Previous := nil;
    Current := FirstFreeBigInt;
    repeat
      if (Current.Capacity>=Size) then begin
        Result:=Current;
        Result.FSize:= Size;
        if DoClear then
          Result.Clear;
        if assigned(previous) then
          Previous.FNextFree := Current.FNextFree
        else
          FirstFreeBigInt := Current.FNextFree;
        exit;
      end;
      Previous := Current;
      Current := Current.FNextFree;
    until Current=nil;
    Result := FirstFreeBigInt;
    Result.Resize(Size);
    FirstFreeBigInt := FirstFreeBigInt.FNextFree;
  end
  else result := TBigInt.Create(Size);
end;

function GetBigInt(bi: TBigInt) : TBigInt; inline;
begin
  result := GetBigInt(bi.FSize, false);
  Move(bi.Digit^, Result.Digit^, bi.FSize*sizeof(dword));
end;

procedure FreeBigInt(bi: TBigInt);
begin
  bi.FNextFree := FirstFreeBigInt;
  FirstFreeBigInt := bi;
end;

{ TBigInt }

operator := (i: integer) : TBigInt; inline;
begin
  Result := GetBigInt(1);
  Result.Digit[0] := dword(i);
end;

constructor TBigInt.Create(InitialSize: integer);
begin
  FSize:= InitialSize;
  Capacity:= 2*FSize;
  GetMem(Digit, Capacity*sizeof(DWord));
  Clear;
end;

destructor TBigInt.Destroy;
begin
  FreeMem(Digit);
  inherited Destroy;
end;

procedure TBigInt.Clear;
begin
  FillChar(Digit[0], FSize*sizeof(DWord), 0);
end;

procedure TBigInt.Resize(NewSize: integer);
begin
  FreeMem(Digit);
  FSize:= NewSize;
  Capacity:= 2*FSize;
  GetMem(Digit, Capacity*sizeof(DWord));
  Clear;
end;

function TBigInt.IsNegative: boolean; inline;
begin
  result := (Digit[FSize-1] and $80000000)>0;
end;

function TBigInt.IsZero:boolean;inline;
begin
  result := (FSize=1) and (Digit^=0);
end;

procedure TBigInt.Negate;
var
  value: int64;
  valueparts : array[0..1] of dword absolute value;
  carry: integer;
  CurDigit: PDWord;
begin
  if IsZero then exit;
  CurDigit:= @Digit[FSize-1];
  repeat
    CurDigit^:= not CurDigit^;
    dec(CurDigit);
  until CurDigit<Digit;
  carry := 1;
  CurDigit:=Digit;
  while (carry>0) do begin
    value := CurDigit^;
    inc(value);
    CurDigit^ := valueparts[0];
    carry := valueparts[1];
    inc(CurDigit);
  end;
end;

function TBigInt.GetDigit(i: integer): DWord; inline;
begin
  if (i<FSize) then
    result := Digit[i]
  else if IsNegative then
    result := $FFFFFFFF
  else
    result := 0;
end;

{ BigInt Calculation }

procedure PushBigInt(bi: TBigInt);
begin
  inc(BigIntTop);
  if (BigIntTop=BigIntStackLen) then
    RunError(1025); // not implemented, too complicated calculation
  BigIntStack[BigIntTop]:=bi;
end;

procedure PushBigIntByValue(bi: TBigInt);
begin
  inc(BigIntTop);
  if (BigIntTop=BigIntStackLen) then
    RunError(1025); // not implemented, too complicated calculation
  BigIntStack[BigIntTop]:= GetBigInt(bi);
end;

function PopBigInt: TBigInt;
begin
  result:=BigIntStack[BigIntTop];
  dec(BigIntTop);
end;

procedure BigIntAdd;
var
  a, b: TBigInt;
  bSignExtend: dword;
  Result: TBigInt;
  carry: integer;
  sum: int64;
  maxsize, minsize, i: integer;
  sumparts : array[0..1] of integer absolute sum;
  aDigit, bDigit, ResultDigit: PDWord;
begin
  if BigIntStack[BigIntTop-1].FSize<BigIntStack[BigIntTop].FSize then begin
    a:= BigIntStack[BigIntTop];
    b:= BigIntStack[BigIntTop-1];
  end
  else begin
    a:= BigIntStack[BigIntTop-1];
    b:= BigIntStack[BigIntTop];
  end;
  if b.IsZero then
    Result := a
  else begin
    maxsize:=a.FSize;
    minsize:=b.FSize;
    Result := GetBigInt(maxsize+1);
    carry := 0;
    aDigit:= a.Digit; bDigit:= b.Digit; ResultDigit:= Result.Digit;
    for i:= 0 to minsize-1 do begin
      sum := int64(aDigit^) + int64(bDigit^) + carry;
      carry := sumparts[1];
      ResultDigit^ := sumparts[0];
      inc(aDigit); inc(bDigit); inc(ResultDigit);
    end;
    if b.IsNegative then
      bSignExtend := $FFFFFFFF
    else
      bSignExtend := 0;
    for i:= minsize to maxsize do begin
      sum := int64(a.GetDigit(i)) + bSignExtend + carry;
      carry := sumparts[1];
      ResultDigit^ := sumparts[0];
      inc(ResultDigit);
    end;
    while (Result.FSize>1) and (Result.Digit[Result.FSize-1]=0)
      and (Result.Digit[Result.FSize-2] and $80000000=0) do
      dec(Result.FSize);
    while (Result.FSize>1) and
       (Result.Digit[Result.FSize-1]=$FFFFFFFF) and
       (Result.Digit[Result.FSize-2] and $80000000>0) do
      dec(Result.FSize);
    FreeBigInt(a);
  end;
  FreeBigInt(b);
  dec(BigIntTop);
  BigIntStack[BigIntTop]:=Result;
end;

procedure BigIntMulInt(int: integer);
type
  TWordPart = record
    w1, w2: word;
  end;
var
  mcarry: dword;
  value: qword;
  valueparts : array[0..1] of dword absolute value;
  BiNeg, IntNeg: boolean;
  i:Integer;
  TopBi, Result: TBigInt;
  TopBiDigit, ResultDigit: PDWord;
begin
  TopBi := BigIntStack[BigIntTop];
  if (int=0) or (TopBi.IsZero) then begin
    TopBi.FSize := 1;
    TopBi.Digit[0]:=0;
  end
  else begin
    BiNeg := TopBi.IsNegative;
    if BiNeg then
      TopBi.Negate;
    IntNeg := int<0;
    if IntNeg then
      int := -int;
    Result := GetBigInt(TopBi.FSize+1, false);
    mcarry := 0;
    TopBiDigit := TopBi.Digit;
    ResultDigit := Result.Digit;
    if (int and $FFFF0000)=0 then
      for i:= 0 to Result.FSize-2 do begin
        {this is what I want to do, but to get to the carry fpc compiles it into
        an expensive qword*qword mulitplication:   }
        {value := qword(TopBiDigit^) * int + mcarry;}
        value := TWordPart(TopBiDigit^).w1 * word(int) +
          qword(TWordPart(TopBiDigit^).w2 * word(int)) shl 16 + mcarry;
        ResultDigit^ := valueparts[0];
        mcarry := valueparts[1];
        inc(TopBiDigit); inc(ResultDigit);
      end
    else
    // this branch is less often taken, so no hand code dword * dword multiplication
      for i:= 0 to Result.FSize-2 do begin
        value := qword(TopBiDigit^) * int + mcarry;
        ResultDigit^ := valueparts[0];
        mcarry := valueparts[1];
        inc(TopBiDigit); inc(ResultDigit);
      end;
    ResultDigit^ := mcarry;
    while (Result.FSize>1) and (Result.Digit[Result.FSize-1]=0) and
      ((Result.Digit[Result.FSize-2] and $80000000)=0) do
      dec(Result.FSize);
    if (BiNeg<>IntNeg) then
      Result.Negate;
    FreeBigInt(TopBi);
    BigIntStack[BigIntTop]:=Result;
  end;
end;

function BigIntDivIntResult: integer;
var
  dividend: TBigInt;
  divisor: TBigInt;
  carry: dword;
  diff: int64;
  diffparts: array[0..1] of dword absolute diff;
  i: integer;
  DividendDigit: PDWord;
  DivisorDigit: PDWord;

  function DividendIsSmallerThanDivisor : boolean; inline;
  var
    i: integer;
  begin
    while (Dividend.FSize>1) and (Dividend.Digit[Dividend.FSize-1]=0)
      and (Dividend.Digit[Dividend.FSize-2] and $80000000=0) do
      dec(Dividend.FSize);
    if dividend.FSize=divisor.FSize then begin
      i := dividend.FSize-1;
      while (i>=0) and (dividend.Digit[i]=divisor.Digit[i]) do
        dec(i);
      Result:= (i>=0) and (dividend.Digit[i]<divisor.Digit[i]);
    end
    else
      Result:=dividend.FSize<divisor.FSize;
  end;

begin
  dividend := BigIntStack[BigIntTop-1];
  divisor := BigIntStack[BigIntTop];
  Result:=0;
  while not DividendIsSmallerThanDivisor do begin
    inc(Result);
    carry := 0;
    DividendDigit := Dividend.Digit;
    DivisorDigit := Divisor.Digit;
    for i:= 0 to divisor.FSize-1 do begin
      diff := int64(dividendDigit^) - (divisorDigit^ + carry);
      carry := diffparts[1] and $1;
      dividendDigit^ := diffparts[0];
      inc(DividendDigit); inc(DivisorDigit);
    end;
    for i:= divisor.FSize to dividend.FSize-1 do begin
      diff := int64(dividendDigit^) - (divisor.GetDigit(i) + carry);
      carry := diffparts[1] and $1;
      dividendDigit^ := diffparts[0];
      dividend.Digit[i] := diffparts[0];
      inc(DividendDigit);
    end;
  end;
  FreeBigInt(dividend);
  FreeBigInt(divisor);
  dec(BigIntTop,2);
end;

procedure Init;
begin
  BigIntStackLen := 8;
  GetMem(BigIntStack, BigIntStackLen * sizeof(TBigInt));
  BigIntTop := -1;
  FirstFreeBigInt := nil;
end;

procedure Finalize;
begin
  Freemem(BigIntStack);
  FreeBigInts;
end;

{ Matrix manipulation }

procedure FreeBigIntMatrix(a: TBigMatrix); inline;
begin
  FreeBigInt(a[1,1]);
  FreeBigInt(a[1,2]);
  FreeBigInt(a[2,1]);
  FreeBigInt(a[2,2]);
end;

function DotProduct(a1,a2: TBigInt; b1,b2: integer; FreeBigInt: boolean) : TBigInt; inline;
begin
  if FreeBigInt then
    PushBigInt(a1)
  else
    PushBigIntByValue(a1);
  BigIntMulInt(b1);
  if FreeBigInt then
    PushBigInt(a2)
  else
    PushBigIntByValue(a2);
  BigIntMulInt(b2);
  BigIntAdd;
  Result:= PopBigInt;
end;

operator * (a: TBigMatrix; b : TIntMatrix) : TBigMatrix;
begin
  result[1,1] := DotProduct(a[1,1],a[1,2], b[1,1], b[2,1], false);
  result[1,2] := DotProduct(a[1,1],a[1,2], b[1,2], b[2,2], true);
  result[2,1] := DotProduct(a[2,1],a[2,2], b[1,1], b[2,1], false);
  result[2,2] := DotProduct(a[2,1],a[2,2], b[1,2], b[2,2], true);
end;

operator * (a: TIntMatrix; b : TBigMatrix) : TBigMatrix;
begin
  result[1,1] := DotProduct(b[1,1],b[2,1],a[1,1],a[1,2], false);
  result[1,2] := DotProduct(b[1,2],b[2,2],a[1,1],a[1,2], false);
  result[2,1] := DotProduct(b[1,1],b[2,1],a[2,1],a[2,2], true);
  result[2,2] := DotProduct(b[1,2],b[2,2],a[2,1],a[2,2], true);
end;

function InitBigMatrix(a,b,c,d: integer): TBigMatrix;
begin
  result[1,1] := a;
  result[1,2] := b;
  result[2,1] := c;
  result[2,2] := d;
end;

function InitIntMatrix(a,b,c,d: integer): TIntMatrix; inline;
begin
  result[1,1] := a;
  result[1,2] := b;
  result[2,1] := c;
  result[2,2] := d;
end;

{ calculating pidigits}

procedure PrintPiDigits(const NumDigits: integer);
var
  n: integer = 0;
  k: integer = 0;
  z: TBigMatrix;
  x,p: TIntMatrix;
  Digit: integer;

  function Extract(x:integer) : integer;
  begin
    PushBigIntByValue(z[1,1]);
    BigIntMulInt(x);
    PushBigIntByValue(z[1,2]);
    BigIntAdd;
    PushBigIntByValue(z[2,1]);
    BigIntMulInt(x);
    PushBigIntByValue(z[2,2]);
    BigIntAdd;
    result := BigIntDivIntResult;
  end;
  
  function GetDigit : integer;
  begin
    result := Extract(3);
  end;

  function IsSafe : boolean;
  begin
    result := Digit = Extract(4);
  end;

  procedure Produce;
  begin
    p[1,2] := -10 * digit;
    z := p * z;
  end;

  procedure Consume;
  begin
    inc(k);
    x[1,1] := k;
    x[1,2] := 4*k+2;
    x[2,2] := 2*k+1;
    z:= z * x;
  end;

begin
  z := InitBigMatrix(1, 0, 0, 1);
  p := InitIntMatrix(10, 0, 0, 1);
  x[2,1] := 0;
  while (n<NumDigits) do begin
    Digit := GetDigit;
    while not IsSafe do begin
      Consume;
      Digit:= GetDigit;
    end;
    Produce;
    write(Digit);
    inc(n);
    if (n mod 10)=0 then
      writeln(#9':', n);
  end;
  FreeBigIntMatrix(z);
end;

var
  n: integer;
  errorcode: integer;

begin
  Init;
  if (ParamCount=1) then begin
    val(ParamStr(1), n, errorcode);
    PrintPiDigits(n);
  end;
  Finalize;
end.
