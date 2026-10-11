
generic

   type Real is digits <>;
   No_Of_Tasks : Positive;
   Matrix_Size : Positive;

package Spectral_Utils is

   type Matrix is array(Natural range 0 .. Matrix_Size-1) of Real;

   --  Evaluate matrix A at indices I, J.
 
   function Eval_A(I, J : Natural) return Real;

   --  Get   A_transpose_A_times_U = A_transpose * A * U. 
 
   procedure Eval_Ata_Times_U
     (U                     : in Matrix; 
      A_transpose_A_times_U : out Matrix);

   --  Get   AU = A * U.  Calculate only AU(Start .. Finish).
 
   procedure Eval_A_Times 
     (U      : in  Matrix; 
      Start  : in  Natural; 
      Finish : in  Natural; 
      AU     : out Matrix);

   --  Get   AU = A_transpose * U.   Calculate only AU(Start .. Finish).
 
   procedure Eval_At_Times
     (U      : in  Matrix; 
      Start  : in  Natural; 
      Finish : in  Natural; 
      AU     : out Matrix);

   pragma Inline (Eval_A_Times, Eval_At_Times);
   pragma Inline (Eval_A, Eval_Ata_Times_U);

end Spectral_Utils;
