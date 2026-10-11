-- The Computer Language Benchmarks Game
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
-- Contributed by Jim Rogers
-- Modified by Jonathan Parker (Oct 2009)

pragma Restrictions (No_Abort_Statements);
pragma Restrictions (Max_Asynchronous_Select_Nesting => 0);

with Ada.Text_Io;
with Ada.Numerics.Generic_Elementary_Functions;
with Ada.Command_Line; use Ada.Command_Line;
with Spectral_Utils;

procedure SpectralNorm is

   type Real is digits 15;

   No_of_Cores_to_Use : constant := 4;

   package Real_IO is new Ada.Text_Io.Float_Io(Real);
   package Real_Funcs is new Ada.Numerics.Generic_Elementary_Functions(Real);
   use Real_Funcs;

   N : Natural := 100;
   Vbv, Vv : Real := 0.0;
begin
   if Argument_Count = 1 then
      N := Natural'Value (Argument(1));
   end if;

   declare
      package Spectral_Utilities is new Spectral_Utils
        (Real, No_of_Tasks => No_of_Cores_to_Use, Matrix_Size => N);
      use Spectral_Utilities;

      U : Matrix := (Others => 1.0);
      V : Matrix := (Others => 0.0);
   begin
      for I in 1 .. 10 loop
         Eval_Ata_Times_U(U, V);
         Eval_Ata_Times_U(V, U);
      end loop;
      for I in V'Range loop
         Vbv := Vbv + U(I) * V(I);
         Vv  := Vv  + V(I) * V(I);
      end loop;
   end;
   Real_IO.Put(Item => Sqrt(Vbv/Vv), Fore => 1, Aft => 9, Exp => 0);
   Ada.Text_Io.New_Line;
end SpectralNorm;
