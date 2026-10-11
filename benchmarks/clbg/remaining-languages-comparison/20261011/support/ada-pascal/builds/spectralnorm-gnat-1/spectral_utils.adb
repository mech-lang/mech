
-----------------------------------------
-- The Great Computer Language Shootout
--
-- Contributed by Jim Rogers
-----------------------------------------
package body Spectral_Utils is

   function Eval_A (I, J : Integer) return Long_Float is
   begin
      return 1.0 / Long_Float((I + J) * (I + J + 1) /2 + I + 1);
   end Eval_A;
   
   procedure Eval_A_Times (U : Matrix; Au : out Matrix) is
   begin
      for I in Au'range loop
         Au(I) := 0.0;
         for J in U'range loop
            Au(I) := Au(I) + Eval_A(I, J) * U(J);
         end loop;
      end loop;
   end Eval_A_Times;
   
   procedure Eval_At_Times (U : Matrix; Au : out Matrix) is
   begin
      for I in Au'range loop
         Au(I) := 0.0;
         for J in U'range loop
            Au(I) := Au(I) + Eval_A(J, I) * U(J);
         end loop;
      end loop;
   end Eval_At_Times;
   
   procedure Eval_Ata_Times_U (U : Matrix; Atau : out Matrix) is
      V : Matrix(0..U'Length - 1);
   begin
      Eval_A_Times(U, V);
      Eval_At_Times(V, Atau);
   end Eval_Ata_Times_U;
   
end Spectral_Utils;
