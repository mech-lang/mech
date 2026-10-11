(*
  The Computer Language Benchmarks Game
  https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

  - contributed by Vitaly Trifonof based on a contribution of Ales Katona
  - made multi-threaded by Nitorami, February 2018:
    https://forum.lazarus.freepascal.org/index.php?topic=39935.0
  - additionally modified by Akira1364, March/April 2019
*)

program BinaryTrees;

{$PointerMath On}

uses CMem, {$IFDEF UNIX}CThreads,{$ENDIF} PooledMM, MTProcs;

type
  TMemPool = PooledMM.TNonFreePooledMemManager;

  PDataRec = ^TDataRec;

  TDataRec = record
    Depth: Byte;
    Iterations, Check: Int32;
  end;

  TData = array[0..8] of TDataRec;

  PNode = ^TNode;

  TNode = record
    Left, Right: PNode;
    class function CheckNode(const ANode: PNode): Int32; static; inline;
    class function MakeTree(const Depth: Int32; const MP: TMemPool): PNode; static; inline;
    class procedure DoTrees(Index: PtrInt; UserData: Pointer; Item: TMultiThreadProcItem); static; inline;
  end;

  // Simplified recursive check.
  class function TNode.CheckNode(const ANode: PNode): Int32;
  begin
    with ANode^ do if (Right <> nil) and (Left <> nil) then
      Exit(1 + CheckNode(Right) + CheckNode(Left));
    Result := 1;
  end;

  // Make a tree recursively.
  class function TNode.MakeTree(const Depth: Int32; const MP: TMemPool): PNode;
  begin
    Result := MP.NewItem();
    with Result^ do begin Right := nil; Left := nil; end;
    if Depth > 0 then with Result^ do begin
      Right := MakeTree(Pred(Depth), MP);
      Left := MakeTree(Pred(Depth), MP);
    end;
  end;

const
  MinDepth = 4;
  MaxDepth: Byte = 10;

  // Make multiple trees.
  class procedure TNode.DoTrees(Index: PtrInt; UserData: Pointer; Item: TMultiThreadProcItem);
  var
    I: Int32;
    IPool: TMemPool;
  begin
    with TData(UserData^)[Index] do begin
      Depth := MinDepth + Index * 2;
      Iterations := 1 shl (MaxDepth - Index * 2);
      Check := 0;
      IPool := TMemPool.Create(SizeOf(TNode));
      for I := 1 to Iterations do begin
        Check += CheckNode(MakeTree(Depth, IPool));
        IPool.Clear();
      end;
      IPool.Free();
    end;
  end;

var
  I, HighIndex: Byte;
  P: PDataRec;
  IO: PText;
  Tree: PNode;
  Pool: TMemPool;
  Data: TData;

begin
  IO := @Output;
  if ParamCount = 1 then Val(ParamStr(1), MaxDepth);
  if MaxDepth < MinDepth + 2 then MaxDepth := MinDepth + 2;

  // Create and destroy a tree of depth MaxDepth + 1.
  Pool := TMemPool.Create(SizeOf(TNode));
  WriteLn(IO^, 'stretch tree of depth ', MaxDepth + 1, #9' check: ',
          TNode.CheckNode(TNode.MakeTree(MaxDepth + 1, Pool)));
  Pool.Clear();

  // Create a "long lived" tree of depth MaxDepth.
  Tree := TNode.MakeTree(MaxDepth, Pool);

  // While the tree stays live, create multiple trees. Local data is stored in the "Data" variable.
  HighIndex := (MaxDepth - MinDepth) div 2;
  ProcThreadPool.MaxThreadCount := 8;
  ProcThreadPool.DoParallel(@TNode.DoTrees, 0, HighIndex, @Data);

  // Display the results, using pointer arithmetic only because this is a benchmark so we actually care about
  // about things at the millisecond level.
  P := @Data[0];
  for I := 0 to HighIndex do begin
    WriteLn(IO^, P^.Iterations, #9' trees of depth ', P^.Depth, #9' check: ', P^.Check);
    Inc(P);
  end;

  // Destroy the long lived tree.
  WriteLn(IO^, 'long lived tree of depth ', MaxDepth, #9' check: ', TNode.CheckNode(Tree));
  Pool.Free();
end.
