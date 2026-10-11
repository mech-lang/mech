
--  The Computer Language Benchmarks Game
--  https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
--  Based on Ada versions created by
--    Jim Rogers and Brian Drummond as well as
--    C version by Francesco Abbate
--
--  Contributed by Brad Moore

private with Ada.Finalization;
private with Apache_Runtime.Pools;

package Trees is

   type Tree_Node is private;
   function Item_Check (Item : Tree_Node) return Integer;

   type Node_Pool is limited private;

   function Create
     (Pool : Node_Pool;
      Depth : Integer) return Tree_Node;

private

   use Apache_Runtime;

   type Node;
   type Tree_Node is access all Node;

   type Node is record
      Left  : Tree_Node;
      Right : Tree_Node;
   end record;

   type Node_Pool is
     new Ada.Finalization.Limited_Controlled with
      record
         Pool : aliased Pools.Pool_Type;
      end record;

   overriding procedure Initialize (Item : in out Node_Pool);
   overriding procedure Finalize   (Item : in out Node_Pool);

end Trees;
