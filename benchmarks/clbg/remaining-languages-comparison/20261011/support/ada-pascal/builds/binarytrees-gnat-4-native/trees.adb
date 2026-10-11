
--  The Computer Language Benchmarks Game
--  https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
--  Based on Ada versions created by
--    Jim Rogers and Brian Drummond as well as the
--    C version by Francesco Abbate
--
--  Contributed by Brad Moore

with Ada.Unchecked_Conversion;
with Interfaces;
with System;

package body Trees is

   Pools_Status : constant Apache_Runtime.Apr_Status :=
     Apache_Runtime.Pools.Initialize;
   pragma Unreferenced (Pools_Status);

   function New_Node (Pool : Node_Pool) return Tree_Node;

   function Create
     (Pool : Node_Pool;
      Depth : Integer) return Tree_Node
   is
      Result : constant Tree_Node := New_Node (Pool);
   begin
      if Depth > 0 then
         Result.all := (Left => Create (Pool, Depth - 1),
                        Right => Create (Pool, Depth - 1));
      else
         Result.all := (Left | Right => null);
      end if;

      return Result;

   end Create;

   overriding procedure Finalize   (Item : in out Node_Pool) is
   begin
      Pools.Destroy (Item.Pool);
   end Finalize;

   overriding procedure Initialize (Item : in out Node_Pool) is
      Status : constant Apr_Status :=
        Pools.Create
          (New_Pool => Item.Pool'Address,
           Parent   => System.Null_Address);
      pragma Unreferenced (Status);
   begin
      null;
   end Initialize;

   function Item_Check (Item : Tree_Node) return Integer is
   begin
      if Item.Left = null then
         return 1;
      else
         return 1 + Item_Check (Item.Left) + Item_Check (Item.Right);
      end if;
   end Item_Check;

   function New_Node (Pool : Node_Pool) return Tree_Node
   is
      function Node_Convert is new Ada.Unchecked_Conversion
        (Source => System.Address,
         Target => Tree_Node);
   begin
      return Node_Convert
        (Pools.Allocate (Pool => Pool.Pool,
                         Size => Node'Size / Interfaces.Unsigned_8'Size));
   end New_Node;
end Trees;
