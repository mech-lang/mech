-- The Computer Language Benchmarks Game
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
-- Contributed by Pascal Obry on 2005/03/21

with Ada.Numerics; use Ada.Numerics;
with Ada.Numerics.Generic_Elementary_Functions;

package Nbody_Pck is

   type Real is Digits 15;

   package Math is new Ada.Numerics.Generic_Elementary_Functions (Real);

   Solar_Mass    : constant Real := 4.0 * Pi * Pi;
   Days_Per_Year : constant Real := 365.24;

   type Data is record
      X, Y, Z    : Real;
      Vx, Vy, Vz : Real;
      Mass       : Real;
   end record;

   type Body_Name is (Sun, Jupiter, Saturn, Uranus, Neptune);

   Bodies : array (Body_Name) of Data :=
              (Jupiter => (X    => 4.84143144246472090e+00,
                           Y    => -1.16032004402742839e+00,
                           Z    => -1.03622044471123109e-01,
                           Vx   => 1.66007664274403694e-03 * Days_Per_Year,
                           Vy   => 7.69901118419740425e-03 * Days_Per_Year,
                           Vz   => -6.90460016972063023e-05 * Days_Per_Year,
                           Mass => 9.54791938424326609e-04 * Solar_Mass),

               Saturn  => (X    => 8.34336671824457987e+00,
                           Y    => 4.12479856412430479e+00,
                           Z    => -4.03523417114321381e-01,
                           Vx   => -2.76742510726862411e-03 * Days_Per_Year,
                           Vy   => 4.99852801234917238e-03 * Days_Per_Year,
                           Vz   => 2.30417297573763929e-05 * Days_Per_Year,
                           Mass => 2.85885980666130812e-04 * Solar_Mass),

               Uranus  => (X    => 1.28943695621391310e+01,
                           y    => -1.51111514016986312e+01,
                           Z    => -2.23307578892655734e-01,
                           Vx   => 2.96460137564761618e-03 * Days_Per_Year,
                           Vy   => 2.37847173959480950e-03 * Days_Per_Year,
                           Vz   => -2.96589568540237556e-05 * Days_Per_Year,
                           Mass => 4.36624404335156298e-05 * Solar_Mass),

               Neptune => (X    => 1.53796971148509165e+01,
                           Y    => -2.59193146099879641e+01,
                           Z    => 1.79258772950371181e-01,
                           Vx   => 2.68067772490389322e-03 * Days_Per_Year,
                           Vy   => 1.62824170038242295e-03 * Days_Per_Year,
                           Vz   => -9.51592254519715870e-05 * Days_Per_Year,
                           Mass => 5.15138902046611451e-05 * Solar_Mass),

               Sun     => (X    => 0.0,
                           Y    => 0.0,
                           Z    => 0.0,
                           Vx   => 0.0,
                           Vy   => 0.0,
                           Vz   => 0.0,
                           Mass => Solar_Mass));

   procedure Offset_Momentum
     (Planet     : in out Data;
      Px, Py, Pz : in     Real);

   function Energy return Real;

   procedure Advance (Dt : in Real);

end Nbody_Pck;
