
--  The Computer Language Benchmarks Game
--  https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
--  Contributed by Brad Moore

with System;

package Apache_Runtime.Pools is

   subtype Pool_Type is System.Address;
   subtype Pool_Access is System.Address;

   function Initialize return Apr_Status;

   function Create
     (New_Pool : Pool_Access;
      Parent : Pool_Type) return Apr_Status;

   procedure Destroy (Pool : Pool_Type);

   function Allocate (Pool : Pool_Type; Size : Apr_Size) return System.Address;

private

   pragma Import (C, Initialize, "apr_initialize");
   pragma Import (C, Destroy, "apr_pool_destroy");
   pragma Import (C, Allocate, "apr_palloc");

end Apache_Runtime.Pools;
