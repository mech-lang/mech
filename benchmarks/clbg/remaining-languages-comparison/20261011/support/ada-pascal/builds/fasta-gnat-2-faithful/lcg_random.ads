

generic

   type Real is digits <>;

package LCG_Random is

   function Gen_Random (Max : in Real) return Real;
   -- Linear congruential random number generator. 
   -- Period = 139_968, with output in [0.0, 1.0) if Max = 1.0.

end LCG_Random;
