
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
