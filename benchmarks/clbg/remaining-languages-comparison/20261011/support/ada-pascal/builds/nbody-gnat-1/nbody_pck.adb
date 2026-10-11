
-- The Great Computer Language Shootout
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
-- Contributed by Pascal Obry on 2005/03/21

package body Nbody_Pck is

   procedure Offset_Momentum
     (Planet     : in out Data;
      Px, Py, Pz : in     Real) is
   begin
      Planet.Vx := -Px / Solar_Mass;
      Planet.Vy := -Py / Solar_Mass;
      Planet.Vz := -Pz / Solar_Mass;
   end Offset_Momentum;

   function Energy return Real is
      Dx, Dy, Dz, Distance : Real;
      E                    : Real := 0.0;
   begin
      for I in Bodies'Range loop
        E := E + 0.5 * Bodies (I).Mass
          * (Bodies (I).Vx * Bodies (I).Vx
               + Bodies (I).Vy * Bodies (I).Vy
               + Bodies (I).Vz * Bodies (I).Vz);

        if I /= Body_Name'Last then
           for J in Body_Name'Succ (I) .. Body_Name'Last loop
              Dx := Bodies (I).X - Bodies (J).X;
              Dy := Bodies (I).Y - Bodies (J).Y;
              Dz := Bodies (I).Z - Bodies (J).Z;

              Distance := Math.Sqrt (Dx * Dx + Dy * Dy + Dz * Dz);
              E := E - (Bodies (I).Mass * Bodies (J).Mass) / Distance;
           end loop;
        end if;
      end loop;
      return E;
   end Energy;

   procedure Advance (Dt : in Real) is
      Dx, Dy, Dz, Distance, Mag : Real;
   begin
      for I in Body_Name'Range loop
         if I /= Body_Name'Last then
            for J in Body_Name'Succ (I) .. Body_Name'Last loop
               Dx := Bodies (I).X - Bodies (J).X;
               Dy := Bodies (I).Y - Bodies (J).Y;
               Dz := Bodies (I).Z - Bodies (J).Z;

               Distance := Math.Sqrt (Dx * Dx + Dy * Dy + Dz * Dz);
               Mag := Dt / (Distance ** 3);

               Bodies (I).Vx := Bodies (I).Vx - Dx * Bodies (J).Mass * Mag;
               Bodies (I).Vy := Bodies (I).Vy - Dy * Bodies (J).Mass * Mag;
               Bodies (I).Vz := Bodies (I).Vz - Dz * Bodies (J).Mass * Mag;

               Bodies (J).Vx := Bodies (J).Vx + Dx * Bodies (I).Mass * Mag;
               Bodies (J).Vy := Bodies (J).Vy + Dy * Bodies (I).Mass * Mag;
               Bodies (J).Vz := Bodies (J).Vz + Dz * Bodies (I).Mass * Mag;
            end loop;
         end if;
      end loop;

      for I in Body_Name'Range loop
         Bodies (I).X := Bodies (I).X + Dt * Bodies (I).Vx;
         Bodies (I).Y := Bodies (I).Y + Dt * Bodies (I).Vy;
         Bodies (I).Z := Bodies (I).Z + Dt * Bodies (I).Vz;
      end loop;
   end Advance;

end Nbody_Pck;
