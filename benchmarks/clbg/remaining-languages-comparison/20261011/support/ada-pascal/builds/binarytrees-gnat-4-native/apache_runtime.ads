
--  The Computer Language Benchmarks Game
--  https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
--  Contributed by Brad Moore

with Interfaces.C;
package Apache_Runtime is
   pragma Pure;

   type Apr_Status is new Integer;

   type Apr_Size is new Interfaces.C.size_t;

end Apache_Runtime;
