-- The Computer Language Benchmarks Game
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
-- Contributed by Pascal Obry on 2005/03/21
-- Modified by Brian Drummond on 2011/03/24
-- Updated by Jonathan Parker and Georg Bauhaus (May 2012)


with Ada.Command_Line; use Ada.Command_Line;
with Ada.Text_IO;      use Ada.Text_IO;
with Nbody_Pck;        use Nbody_Pck;
with Root;

procedure Nbody is

   subtype Real is Root.S_Real;

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
      Advance (0.01);
   end loop;

   Put (Energy);
   New_Line;

end Nbody;

with Ada.Numerics; use Ada.Numerics;
with Root; use Root;

package Nbody_Pck is

   subtype Real is Root.S_Real;

   Solar_Mass    : constant Real := 4.0 * Pi * Pi;
   Days_Per_Year : constant Real := 365.24;

   type Signed is range -2**15 .. 2**15-1; 
   subtype Body_Name is Signed range 0 .. 4;

   Jupiter : constant := 0;
   Saturn  : constant := 1;
   Neptune : constant := 2;
   Uranus  : constant := 3;
   Sun     : constant := 4;

   type Axes is (X, Y, Z);

   procedure Offset_Momentum
     (Planet     : in Body_Name;
      Px, Py, Pz : in Real);

   procedure Add_Momentum
     (Planet     : in     Body_Name;
      Px, Py, Pz : in out Real);

   function Energy return Real;

   procedure Advance (Dt : in Real);

private

   type Solar_System is array (Body_Name, Axes) of Real;
   pragma Convention (Ada, Solar_System);

   Position : Solar_System :=
     (Jupiter => (X  =>  4.84143144246472090e+00,
                  Y  => -1.16032004402742839e+00,
                  Z  => -1.03622044471123109e-01),
      Saturn  => (X  =>  8.34336671824457987e+00,
                  Y  =>  4.12479856412430479e+00,
                  Z  => -4.03523417114321381e-01),
      Uranus  => (X  =>  1.28943695621391310e+01,
                  y  => -1.51111514016986312e+01,
                  Z  => -2.23307578892655734e-01),
      Neptune => (X  =>  1.53796971148509165e+01,
                  Y  => -2.59193146099879641e+01,
                  Z  =>  1.79258772950371181e-01),
      Sun     => (X  =>  0.0,
                  Y  =>  0.0,
                  Z  =>  0.0));

   Velocity : Solar_System :=
     (Jupiter => (X  =>  1.66007664274403694e-03 * Days_Per_Year,
                  Y  =>  7.69901118419740425e-03 * Days_Per_Year,
                  Z  => -6.90460016972063023e-05 * Days_Per_Year),
      Saturn  => (X  => -2.76742510726862411e-03 * Days_Per_Year,
                  Y  =>  4.99852801234917238e-03 * Days_Per_Year,
                  Z  =>  2.30417297573763929e-05 * Days_Per_Year),
      Uranus  => (X  =>  2.96460137564761618e-03 * Days_Per_Year,
                  Y  =>  2.37847173959480950e-03 * Days_Per_Year,
                  Z  => -2.96589568540237556e-05 * Days_Per_Year),
      Neptune => (X  =>  2.68067772490389322e-03 * Days_Per_Year,
                  Y  =>  1.62824170038242295e-03 * Days_Per_Year,
                  Z  => -9.51592254519715870e-05 * Days_Per_Year),
      Sun     => (X  =>  0.0,
                  Y  =>  0.0,
                  Z  =>  0.0));

   type Body_Mass is array(Body_Name) of Real;

   Mass : constant Body_Mass :=
     (Jupiter => 9.54791938424326609e-04 * Solar_Mass,
      Saturn  => 2.85885980666130812e-04 * Solar_Mass,
      Uranus  => 4.36624404335156298e-05 * Solar_Mass,
      Neptune => 5.15138902046611451e-05 * Solar_Mass,
      Sun     => Solar_Mass);

end Nbody_Pck;

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
     (Planet     : in     Body_Name;
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
      for i in Body_Name loop
        E := E + 0.5 * Mass (i) *
          (Velocity (i, X) * Velocity (i, X)
         + Velocity (i, Y) * Velocity (i, Y)
         + Velocity (i, Z) * Velocity (i, Z));

        if i /= Body_Name'Last then
           for j in Body_Name'Succ (i) .. Body_Name'Last loop
              Dx := Position (i, X) - Position (j, X);
              Dy := Position (i, Y) - Position (j, Y);
              Dz := Position (i, Z) - Position (j, Z);
              Distance := Sqrt (Dx * Dx + Dy * Dy + Dz * Dz);
              E := E - (Mass (i) * Mass (j)) / Distance;
           end loop;
        end if;
      end loop;
      return E;
   end Energy;

   procedure Advance (Dt : in Real) is
      Dx, Dy, Dz, Mag, s : Real;
      Mass_i, Mass_j : Real;
   begin
      for i in Body_Name loop
         Mass_i := Mass(i);
         for j in Body_Name loop
            if j > i then
               Dx := Position (i, X) - Position (j, X);
               Dy := Position (i, Y) - Position (j, Y);
               Dz := Position (i, Z) - Position (j, Z);

               Mass_j := Mass(j);
               s      := SSE_Reciprocal_Sqrt (Dx*Dx + Dy*Dy + Dz*Dz);
               Mag    := (dt * s) * (s * s);

               Velocity (i, X) := Velocity (i, X) - Dx * Mass_j * Mag;
               Velocity (j, X) := Velocity (j, X) + Dx * Mass_i * Mag;
               Velocity (i, Y) := Velocity (i, Y) - Dy * Mass_j * Mag;
               Velocity (j, Y) := Velocity (j, Y) + Dy * Mass_i * Mag;
               Velocity (i, Z) := Velocity (i, Z) - Dz * Mass_j * Mag;
               Velocity (j, Z) := Velocity (j, Z) + Dz * Mass_i * Mag;
            end if;
         end loop;
      end loop;

      for i in Body_Name loop
         Position (i, X) := Position (i, X) + Dt * Velocity (i, X);
         Position (i, Y) := Position (i, Y) + Dt * Velocity (i, Y);
         Position (i, Z) := Position (i, Z) + Dt * Velocity (i, Z);
      end loop;
   end Advance;

end Nbody_Pck;

package Root is

   type S_Real is new Long_Float;

   pragma Assert (S_Real'Size = 64 and S_Real'digits > 13);
  
   type SSE_Vector is array (0 .. 1) of S_Real;

   function Sqrt (X : S_Real) return S_Real;

   function Sqrt_of_Reciprocal (X : S_Real) return S_Real;

   function SSE_Reciprocal_Sqrt (X : S_Real) return S_Real;
   -- Returns double precision 1.0 / Sqrt(X), for Long_Float X.

   pragma Inline (SSE_Reciprocal_Sqrt);

end Root;

package body root is

   -- "divpd" and "sqrtpd" are  double precision:

   type m128d is array (0 .. 1) of S_Real;
   for m128d'Alignment use 16;
   pragma Machine_Attribute (m128d, "vector_type");

   function ia32_Div (X, Y : m128d) return m128d;
   pragma Import (Intrinsic, ia32_Div, "__builtin_ia32_divpd");

   function ia32_Sqrt (X : m128d) return m128d;
   pragma Import (Intrinsic, ia32_Sqrt, "__builtin_ia32_sqrtpd");

   function Sqrt (X : S_Real) return S_Real is
   begin return ia32_Sqrt ((X, 1.0))(0);
   end Sqrt;

   function Sqrt_of_Reciprocal (X : S_Real) return S_Real is
      a : constant m128d := ia32_Div ((1.0, 1.0), (X, 1.0));
      b : constant m128d := ia32_Sqrt (a);
   begin
      return b(0);
   end Sqrt_of_Reciprocal;

   -- "rsqrtps" (Reciprocal Sqrt) operates on Float (single precision):

   type m128s is array (0 .. 3) of Float;
   for m128s'Alignment use 16;
   pragma Machine_Attribute (m128s, "vector_type");
   pragma Assert (Float'Digits < 7 and m128s'size = 128);

   function ia32_RSqrt (X : m128s) return m128s;
   pragma Import (Intrinsic, ia32_RSqrt, "__builtin_ia32_rsqrtps");

   function Recip_Sqrt (X : S_Real) return S_Real is
      Z :  constant m128s := ia32_RSqrt ((Float (X), others => 1.0));
      r : S_Real := S_Real (Z(0));
   begin
      for i in 1 .. 2 loop
         r := r * 1.5 - ((0.5 * X) * r) * (r * r);
      end loop;
      return r;
   end Recip_Sqrt;
   pragma Inline (Recip_Sqrt); 

   function SSE_Reciprocal_Sqrt (X : S_Real) return S_Real is
   begin
      if Abs X < 1.0e+30 and Abs X > 1.0e-30 then
         return Recip_Sqrt (X);
      else
         return Sqrt_of_Reciprocal (X);
      end if;
   end SSE_Reciprocal_Sqrt;

   x : constant m128d := (4.0, 6.0);
   y : constant m128d := (2.0, 2.0);
   pragma Assert (ia32_Div(x, y) = m128d'(2.0, 3.0));
   -- Minimal test, but a good idea when using pragma Import.

end root;
