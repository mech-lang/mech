-- The Computer Language Benchmarks Game
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
-- Contributed by Pascal Obry on 2005/03/21
-- Modified by Brian Drummond on 2011/03/24

with Ada.Command_Line; use Ada.Command_Line;
with Ada.Text_IO;      use Ada.Text_IO;
with Nbody_Pck;        use Nbody_Pck;

procedure Nbody is

   package RIO is new Float_Io (Real);

   procedure Put
     (Item : Real; Fore : Field := 0; Aft : Field := 9;
      Exp  : Field := 0) renames RIO.Put;

   N : constant Integer := Integer'Value (Argument (1));

   Px, Py, Pz : Real := 0.0;

begin
   for I in Body_Name'Range loop
      Add_Momentum (I, Px, Py, Pz);
   end loop;

   Offset_Momentum (Sun, Px, Py, Pz);

   Put (Energy);
   New_Line;

   for K in 1 .. N loop
      Advance(0.01);
   end loop;

   Put (Energy);
   New_Line;
end Nbody;
