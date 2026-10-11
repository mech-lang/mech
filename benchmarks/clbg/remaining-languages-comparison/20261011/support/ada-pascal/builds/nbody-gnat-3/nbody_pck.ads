
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
