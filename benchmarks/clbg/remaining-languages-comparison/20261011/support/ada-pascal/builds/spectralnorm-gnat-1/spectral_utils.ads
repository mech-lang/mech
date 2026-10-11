-----------------------------------------
-- The Computer Language Benchmarks Game
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
-- 
-- Contributed by Jim Rogers
-----------------------------------------
package Spectral_Utils is
   type Matrix is array(Natural range <>) of Long_Float;
   
   function Eval_A(I, J : Integer) return Long_Float;
   procedure Eval_A_Times(U : Matrix; Au : out Matrix);
   procedure Eval_At_Times(U : Matrix; Au : out Matrix);
   procedure Eval_Ata_Times_U(U : Matrix; Atau : out Matrix);
end Spectral_Utils;
