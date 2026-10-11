
--  The Computer Language Benchmarks Game
--  https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
--  Contributed by Brad Moore

package body Apache_Runtime.Pools is

   function Create_Ex
     (New_Pool : Pool_Access;
      Parent : Pool_Type;
      Reserved_1, Reserved_2 : System.Address) return Apr_Status;
   pragma Import (C, Create_Ex, "apr_pool_create_ex");

   ------------
   -- Create --
   ------------

   function Create
     (New_Pool : Pool_Access;
      Parent : Pool_Type)
      return Apr_Status
   is
   begin
      return Create_Ex
        (New_Pool,
         Parent,
         System.Null_Address,
         System.Null_Address);
   end Create;

end Apache_Runtime.Pools;
