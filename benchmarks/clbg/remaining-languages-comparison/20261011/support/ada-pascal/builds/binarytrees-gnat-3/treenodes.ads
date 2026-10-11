
-- The Computer Language Benchmarks Game
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
-- Contributed by Jim Rogers
-- Modified by Brian Drummond

package Treenodes is
   type Treenode is private;
   function Bottom_Up_Tree(Depth : Integer) return Treenode;
   procedure Item_Check(This : in out Treenode; Sum : out Integer);
private
   type Node;
   type Treenode is access Node;
   type Node is record
      Left  : Treenode := null;
      Right : Treenode := null; 
   end record;
end Treenodes;
