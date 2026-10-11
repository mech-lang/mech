-- The Computer Language Benchmarks Game
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
-- Contributed by Jim Rogers
-- Modified by Brian Drummond
-- *reset*

with Treenodes; use Treenodes;
with Ada.Text_Io; use Ada.Text_Io;
with Ada.Integer_Text_Io; use Ada.Integer_Text_Io;
with Ada.Command_Line; use Ada.Command_Line;
with Ada.Characters.Latin_1; use Ada.Characters.Latin_1;

procedure Binarytrees is
   -- Change "CPUs" to control number of tasks created
   CPUs : constant Positive := 4;
   BlockSize : Positive;
   Min_Depth : constant Positive := 4;
   N : Natural := 1;
   Stretch_Tree : TreeNode;
   Long_Lived_Tree : TreeNode;
   Max_Depth : Positive;
   Stretch_Depth : Positive;
   Iteration : Positive;
   Iterations : Positive;
   Sum : Integer;
   Check : Integer;
   Depth : Natural;

   task type check_this_depth is
      entry Start(Iteration, Size : Positive; To_Depth :in Natural);
      entry Complete(Result : out Integer);
   end check_this_depth;

   task body check_this_depth is
      Check : Integer;
      Sum : Integer;
      Depth : Natural;
      First : Positive;
      Last : Positive;
      Short_Lived_Tree_1 : TreeNode;
      Short_Lived_Tree_2 : TreeNode;

   begin
      loop
         select
            accept Start(Iteration, Size : Positive; To_Depth :in Natural) do
               First := Iteration;
               Last := Iteration + Size - 1;
               Depth := To_Depth;
            end Start;
            Check := 0;
            for I in First .. Last loop
               Short_Lived_Tree_1 := Bottom_Up_Tree(Depth => Depth);
               Item_Check(Short_Lived_Tree_1, Sum);
               Check := Check + Sum;
            end loop;
            accept Complete(Result : out Integer) do
               Result := Check;
            end Complete;
         or
            Terminate;
         end select;
      end loop;
   end check_this_depth;

   subtype Task_Count is positive range 1 .. CPUs;	
   Tasks : array (Task_Count) of check_this_depth;

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

-- Setup tasking parameters for reasonable task granularity
-- Too large and we can't balance CPU loads
-- Too small and we waste time in task switches
-- Not very critical - anything more complex is probably a waste of effort
      
      BlockSize := 2**10;
      if Iterations < BlockSize * CPUs then
         BlockSize := 1;
      end if;
  
-- Check that Iterations is a multiple of Blocksize * CPUs
-- Error out otherwise (dealing with remainder is trivial but tedious)
      Pragma Assert(Iterations mod( BlockSize * CPUs) = 0, 
                            "Iteration count not supported!");

      -- for I in 1..Iterations loop  
      Iteration := 1;   
      while Iteration <= Iterations loop
         for j in Task_Count loop
            Tasks(j).Start(Iteration, Blocksize, Depth);
            Iteration := Iteration + BlockSize;
         end loop;
         for j in Task_Count loop
            Tasks(j).Complete(Sum);
            Check := Check + Sum;
         end loop;
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
end Binarytrees;

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
