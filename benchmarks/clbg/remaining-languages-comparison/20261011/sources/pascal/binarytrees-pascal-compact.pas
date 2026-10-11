{ Every leaf and interior allocates a distinct, equal-size two-pointer node. }
{$mode objfpc}
uses SysUtils;
type
  PNode = ^TNode;
  TNode = record Left,Right: PNode; end;
function Make(Depth: LongInt): PNode;
begin
  New(Result);
  Result^.Left := nil;
  Result^.Right := nil;
  if Depth>0 then begin
    Result^.Left := Make(Depth-1);
    Result^.Right := Make(Depth-1);
  end;
end;
function Check(Node: PNode): LongInt;
begin
  if Node^.Left=nil then Result := 1
  else Result := 1+Check(Node^.Left)+Check(Node^.Right);
end;
procedure FreeTree(Node: PNode);
begin
  if Node^.Left<>nil then begin
    FreeTree(Node^.Left);
    FreeTree(Node^.Right);
  end;
  Dispose(Node);
end;
var
  Depth,MaxDepth,Iterations,I,Sum: LongInt;
  Root,Tree: PNode;
begin
  MaxDepth := StrToInt(ParamStr(1));
  if MaxDepth<6 then MaxDepth := 6;
  Tree := Make(MaxDepth+1);
  WriteLn('stretch tree of depth ',MaxDepth+1,#9' check: ',Check(Tree));
  FreeTree(Tree);
  Root := Make(MaxDepth);
  Depth := 4;
  while Depth<=MaxDepth do begin
    Iterations := 1 shl (MaxDepth-Depth+4);
    Sum := 0;
    for I := 1 to Iterations do begin
      Tree := Make(Depth);
      Inc(Sum,Check(Tree));
      FreeTree(Tree);
    end;
    WriteLn(Iterations,#9' trees of depth ',Depth,#9' check: ',Sum);
    Inc(Depth,2);
  end;
  WriteLn('long lived tree of depth ',MaxDepth,#9' check: ',Check(Root));
  FreeTree(Root);
end.
