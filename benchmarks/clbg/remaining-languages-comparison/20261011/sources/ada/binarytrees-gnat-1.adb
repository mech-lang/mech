-- The Computer Language Benchmarks Game
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
-- Contributed by Jim Rogers
-- *reset*

with Treenodes; use Treenodes;
with Ada.Text_Io; use Ada.Text_Io;
with Ada.Integer_Text_Io; use Ada.Integer_Text_Io;
with Ada.Command_Line; use Ada.Command_Line;
with Ada.Characters.Latin_1; use Ada.Characters.Latin_1;

procedure Binarytrees is
   Min_Depth : constant Positive := 4;
   N : Natural := 1;
   Stretch_Tree : TreeNode;
   Long_Lived_Tree : TreeNode;
   Short_Lived_Tree_1 : TreeNode;
   Short_Lived_Tree_2 : TreeNode;
   Max_Depth : Positive;
   Stretch_Depth : Positive;
   Check : Integer;
   Sum : Integer;
   Depth : Natural;
   Iterations : Positive;
begin
   if Argument_Count > 0 then
      N := Positive'Value(Argument(1));
   end if;
   Max_Depth := Positive'Max(Min_Depth + 2, N);
   Stretch_Depth := Max_Depth + 1;
   Stretch_Tree := Bottom_Up_Tree(Stretch_Depth);
   Item_Check(Stretch_Tree, Check);
   Put("stretch tree of depth ");
   Put(Item => Stretch_Depth, Width => 1);
   Put(Ht & " check: ");
   Put(Item => Check, Width => 1);
   New_Line;
   
   Long_Lived_Tree := Bottom_Up_Tree(Max_Depth);
   
   Depth := Min_Depth;
   while Depth <= Max_Depth loop
      Iterations := 2**(Max_Depth - Depth + Min_Depth);
      Check := 0;
      for I in 1..Iterations loop
         Short_Lived_Tree_1 := Bottom_Up_Tree(Depth => Depth);
         Item_Check(Short_Lived_Tree_1, Sum);
         Check := check + Sum;
      end loop;
      Put(Item => Iterations, Width => 0);
      Put(Ht & " trees of depth ");
      Put(Item => Depth, Width => 0);
      Put(Ht & " check: ");
      Put(Item => Check, Width => 0);
      New_Line;
      Depth := Depth + 2;
   end loop;
   Put("long lived tree of depth ");
   Put(Item => Max_Depth, Width => 0);
   Put(Ht & " check: ");
   Item_Check(Long_Lived_Tree, Check);
   Put(Item => Check, Width => 0);
   New_Line;
end BinaryTrees;
----------------------------------------------------------------
-- BinaryTrees
--
-- Ada 95 (GNAT)
--
-- Contributed by Jim Rogers
----------------------------------------------------------------

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
----------------------------------------------------------------
-- BinaryTrees
--
-- Ada 95 (GNAT)
--
-- Contributed by Jim Rogers
----------------------------------------------------------------

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
