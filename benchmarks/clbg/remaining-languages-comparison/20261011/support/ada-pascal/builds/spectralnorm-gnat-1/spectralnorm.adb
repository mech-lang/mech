
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
