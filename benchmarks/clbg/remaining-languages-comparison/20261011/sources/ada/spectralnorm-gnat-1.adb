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

-----------------------------------------
-- The Great Computer Language Shootout
--
-- Contributed by Jim Rogers
-----------------------------------------
package body Spectral_Utils is

   function Eval_A (I, J : Integer) return Long_Float is
   begin
      return 1.0 / Long_Float((I + J) * (I + J + 1) /2 + I + 1);
   end Eval_A;
   
   procedure Eval_A_Times (U : Matrix; Au : out Matrix) is
   begin
      for I in Au'range loop
         Au(I) := 0.0;
         for J in U'range loop
            Au(I) := Au(I) + Eval_A(I, J) * U(J);
         end loop;
      end loop;
   end Eval_A_Times;
   
   procedure Eval_At_Times (U : Matrix; Au : out Matrix) is
   begin
      for I in Au'range loop
         Au(I) := 0.0;
         for J in U'range loop
            Au(I) := Au(I) + Eval_A(J, I) * U(J);
         end loop;
      end loop;
   end Eval_At_Times;
   
   procedure Eval_Ata_Times_U (U : Matrix; Atau : out Matrix) is
      V : Matrix(0..U'Length - 1);
   begin
      Eval_A_Times(U, V);
      Eval_At_Times(V, Atau);
   end Eval_Ata_Times_U;
   
end Spectral_Utils;

-----------------------------------------------------------
-- The Great Computer Language Shootout
--
-- Contributed by Jim Rogers
-----------------------------------------------------------

with Ada.Text_Io;
with Ada.Numerics.Generic_Elementary_Functions ;
with Ada.Command_Line; use Ada.Command_Line;
with Spectral_Utils; use Spectral_Utils;

procedure SpectralNorm is
   package Long_Io is new Ada.Text_Io.Float_Io(Long_Float);
   package Long_Funcs is new Ada.Numerics.Generic_Elementary_Functions(Long_Float);
   use Long_Funcs;
   N : Natural := 100;
   Vbv, vv : Long_Float := 0.0;
begin
   if Argument_Count = 1 then
      N := Natural'Value(Argument(1));
   end if;
   
   declare
      U : Matrix(0..N-1) := (Others => 1.0);
      V : Matrix(0..N-1) := (Others => 0.0);
   begin
      for I in 1..10 loop
         Eval_Ata_Times_U(U, V);
         Eval_Ata_Times_U(V, U);
      end loop;
      for I in V'range loop
         Vbv := Vbv + U(I) * V(I);
         Vv := Vv + V(I)*V(I);
      end loop;
   end;
   Long_Io.Put(Item => Sqrt(Vbv/Vv), Fore => 1, Aft => 9, Exp => 0);
   Ada.Text_Io.New_Line;
end SpectralNorm;
