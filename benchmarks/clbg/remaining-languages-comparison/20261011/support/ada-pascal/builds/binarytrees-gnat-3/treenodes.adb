
-- The Computer Language Benchmarks Game
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
-- Contributed by Jim Rogers
-- Modified by Brian Drummond

with Ada.Unchecked_Deallocation;

package body Treenodes is
   function Bottom_Up_Tree(Depth : Integer)
      return Treenode is
   begin
      if Depth > 0 then
         return new Node'(Bottom_Up_Tree(Depth -1),
            Bottom_Up_Tree(Depth -1));
      else
         return new Node'(null, null);
      end if;
   end Bottom_Up_Tree;

   procedure Item_Check (This : in out Treenode; Sum : out Integer) is
      procedure Free is new Ada.Unchecked_Deallocation(Node, Treenode);
      Left_Sum, Right_Sum : Integer;
   begin
      if This.Left = null then
         Sum := 1;
      else
         Item_Check(This.Left, Left_Sum);
         Item_Check(This.Right, Right_Sum);
         Sum :=  1 + Left_Sum + Right_Sum;
      end if;
      Free(This);      
   end Item_Check;

end Treenodes;
