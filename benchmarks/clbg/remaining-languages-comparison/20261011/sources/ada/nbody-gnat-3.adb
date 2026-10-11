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

-- The Great Computer Language Shootout
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
-- Contributed by Pascal Obry on 2005/03/21
-- Modified by Brian Drummond on 2011/03/24

with Ada.Numerics; use Ada.Numerics;
with Ada.Numerics.Generic_Elementary_Functions;

package Nbody_Pck is

   type Real is Digits 15;

   package Math is new Ada.Numerics.Generic_Elementary_Functions (Real);

   Solar_Mass    : constant Real := 4.0 * Pi * Pi;
   Days_Per_Year : constant Real := 365.24;

   type Body_Name is (Sun, Jupiter, Saturn, Uranus, Neptune);
   type Axes      is (X, Y, Z);

   procedure Offset_Momentum
     (Planet     : in Body_Name;
      Px, Py, Pz : in Real);

   procedure Add_Momentum
     (Planet     : in Body_Name;
      Px, Py, Pz : in out Real);

   function Energy return Real;

   procedure Advance(Dt : in Real);

private

   Position : array (Body_Name,Axes) of Real :=
              (Jupiter => (X    => 4.84143144246472090e+00,
                           Y    => -1.16032004402742839e+00,
                           Z    => -1.03622044471123109e-01),
               Saturn  => (X    => 8.34336671824457987e+00,
                           Y    => 4.12479856412430479e+00,
                           Z    => -4.03523417114321381e-01),
               Uranus  => (X    => 1.28943695621391310e+01,
                           y    => -1.51111514016986312e+01,
                           Z    => -2.23307578892655734e-01),
               Neptune => (X    => 1.53796971148509165e+01,
                           Y    => -2.59193146099879641e+01,
                           Z    => 1.79258772950371181e-01),
               Sun     => (X    => 0.0,
                           Y    => 0.0,
                           Z    => 0.0));

   Velocity : array (Body_Name,Axes) of Real :=
              (Jupiter => (X    => 1.66007664274403694e-03 * Days_Per_Year,
                           Y    => 7.69901118419740425e-03 * Days_Per_Year,
                           Z    => -6.90460016972063023e-05 * Days_Per_Year),
               Saturn  => (X    => -2.76742510726862411e-03 * Days_Per_Year,
                           Y    => 4.99852801234917238e-03 * Days_Per_Year,
                           Z    => 2.30417297573763929e-05 * Days_Per_Year),
               Uranus  => (X    => 2.96460137564761618e-03 * Days_Per_Year,
                           Y    => 2.37847173959480950e-03 * Days_Per_Year,
                           Z    => -2.96589568540237556e-05 * Days_Per_Year),
               Neptune => (X    => 2.68067772490389322e-03 * Days_Per_Year,
                           Y    => 1.62824170038242295e-03 * Days_Per_Year,
                           Z    => -9.51592254519715870e-05 * Days_Per_Year),
               Sun     => (X    => 0.0,
                           Y    => 0.0,
                           Z    => 0.0));

   Mass: constant array (Body_Name) of Real :=
              (Jupiter => 9.54791938424326609e-04 * Solar_Mass,
               Saturn  => 2.85885980666130812e-04 * Solar_Mass,
               Uranus  => 4.36624404335156298e-05 * Solar_Mass,
               Neptune => 5.15138902046611451e-05 * Solar_Mass,
               Sun     => Solar_Mass);

end Nbody_Pck;

-- The Great Computer Language Shootout
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
-- Contributed by Pascal Obry on 2005/03/21
-- Modified by Brian Drummond on 2011/03/24

package body Nbody_Pck is

   procedure Offset_Momentum
     (Planet     : in Body_Name;
      Px, Py, Pz : in Real) is
   begin
      Velocity (Planet, X) := -Px / Solar_Mass;
      Velocity (Planet, Y) := -Py / Solar_Mass;
      Velocity (Planet, Z) := -Pz / Solar_Mass;
   end Offset_Momentum;

   procedure Add_Momentum
     (Planet     : in Body_Name;
      Px, Py, Pz : in out Real) is
   begin
      Px := Px + Velocity (Planet, X) * Mass (Planet);
      Py := Py + Velocity (Planet, Y) * Mass (Planet);
      Pz := Pz + Velocity (Planet, Z) * Mass (Planet);
   end Add_Momentum;

   function Energy return Real is
      Dx, Dy, Dz, Distance : Real;
      E                    : Real := 0.0;
   begin
      for I in Body_Name loop
        E := E + 0.5 * Mass (I)
              * (Velocity (I, X) * Velocity (I, X)
               + Velocity (I, Y) * Velocity (I, Y)
               + Velocity (I, Z) * Velocity (I, Z));

        if I /= Body_Name'Last then
           for J in Body_Name'Succ (I) .. Body_Name'Last loop
              Dx := Position (I, X) - Position (J, X);
              Dy := Position (I, Y) - Position (J, Y);
              Dz := Position (I, Z) - Position (J, Z);

              Distance := Math.Sqrt (Dx * Dx + Dy * Dy + Dz * Dz);
              E := E - (Mass (I) * Mass (J)) / Distance;
           end loop;
        end if;
      end loop;
      return E;
   end Energy;

   procedure Advance(Dt : in Real) is
      Dx, Dy, Dz, Dist_Sq, Mag : Real;
      Mass_I: Real;
      subtype I_Name is Body_Name range Body_Name'first .. Body_Name'pred(Body_Name'last);

   begin
      for I in I_Name loop
         Mass_I := Mass (I);   
         for J in Body_Name loop
            if J > I then
               Dx := Position (I, X) - Position (J, X);
               Dy := Position (I, Y) - Position (J, Y);
               Dz := Position (I, Z) - Position (J, Z);

               Dist_sq := Dx*Dx + Dy*Dy + Dz*Dz;
               Mag     := Dt / (Dist_sq * Math.Sqrt (Dist_sq));

               Velocity (I, X) := Velocity (I, X) - Dx * Mass (J) * Mag;
               Velocity (I, Y) := Velocity (I, Y) - Dy * Mass (J) * Mag;
               Velocity (I, Z) := Velocity (I, Z) - Dz * Mass (J) * Mag;

               Velocity (J, X) := Velocity (J, X) + Dx * Mass_I * Mag;
               Velocity (J, Y) := Velocity (J, Y) + Dy * Mass_I * Mag;
               Velocity (J, Z) := Velocity (J, Z) + Dz * Mass_I * Mag;

            end if;
         end loop;
      end loop;

      for I in Body_Name loop
         Position (I, X) := Position (I, X) + Dt * Velocity (I, X);
         Position (I, Y) := Position (I, Y) + Dt * Velocity (I, Y);
         Position (I, Z) := Position (I, Z) + Dt * Velocity (I, Z);
      end loop;
   end Advance;

end Nbody_Pck;
