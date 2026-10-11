
-- The Great Computer Language Shootout
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
-- Contributed by Pascal Obry on 2005/03/21

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
      Px := Px + Bodies (I).Vx * Bodies (I).Mass;
      Py := Py + Bodies (I).Vy * Bodies (I).Mass;
      Pz := Pz + Bodies (I).Vz * Bodies (I).Mass;
   end loop;

   Offset_Momentum (Bodies (Sun), Px, Py, Pz);

   Put (Energy);
   New_Line;

   for K in 1 .. N loop
      Advance (0.01);
   end loop;

   Put (Energy);
   New_Line;
end Nbody;
