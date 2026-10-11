
package body LCG_Random is

   pragma Assert (Real'Digits > 5);

   type Random_State is mod 2**32;

   State : Random_State := 42;

   type Signed is range
      -2**(Random_State'Size-1) .. 2**(Random_State'Size-1) - 1;

   function Gen_Random (Max : in Real) return Real is
      IM : constant := 139_968;
      IA : constant :=   3_877;
      IC : constant :=  29_573;
   begin
      State := (State * IA + IC) mod IM;
      return (Max * Real (Signed (State))) / Real (IM);
   end Gen_Random;

end LCG_Random;
