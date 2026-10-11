(*
  The Computer Language Benchmarks Game
  https://salsa.debian.org/benchmarksgame-team/benchmarksgame/

  - contributed by Vitaly Trifonof based on a contribution of Ales Katona
  - made multi-threaded by Nitorami, February 2018:
    https://forum.lazarus.freepascal.org/index.php?topic=39935.0
  - additionally modified by Akira1364, March/April 2019
*)

program BinaryTrees;

uses CMem, {$IFDEF UNIX}CThreads,{$ENDIF} PooledMM, MTProcs;

type
  TMemPool = PooledMM.TNonFreePooledMemManager;

  TData = array of record
    Depth, Iterations, Check: Int32;
  end;

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
  MaxDepth: PtrUInt = 10;

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
  I: PtrUInt;
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
  WriteLn(IO^, 'stretch tree of depth ', Succ(MaxDepth), #9' check: ',
          TNode.CheckNode(TNode.MakeTree(Succ(MaxDepth), Pool)));
  Pool.Clear();

  // Create a "long lived" tree of depth MaxDepth.
  Tree := TNode.MakeTree(MaxDepth, Pool);

  // While the tree stays live, create multiple trees. Local data is stored in the "Data" variable.
  SetLength(Data, (MaxDepth - MinDepth) div 2 + 1);
  ProcThreadPool.MaxThreadCount := 8;
  ProcThreadPool.DoParallel(@TNode.DoTrees, 0, High(Data), @Data);

  // Display the results.
  for I := 0 to High(Data) do with Data[I] do
    WriteLn(IO^, Iterations, #9' trees of depth ', Depth, #9' check: ', Check);

  // Destroy the long lived tree.
  WriteLn(IO^, 'long lived tree of depth ', MaxDepth, #9' check: ', TNode.CheckNode(Tree));
  Pool.Free();
end.
