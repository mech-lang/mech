-- The Computer Language Benchmarks Game
-- https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
--
-- Contributed by Jim Rogers
-- Fixed by Pascal Obry on 2005/03/21
-- Modified Experimental version suggested by Pascal Obry

with GNAT.IO;          use GNAT.IO;
with Ada.Command_Line; use Ada.Command_Line;
with Interfaces;       use Interfaces;

procedure Mandelbrot is
   type Real is digits 15;
   Iter                   : constant := 50;
   Limit                  : constant := 4.0;
   Width, Height          : Positive;
   Bit_Num                : Natural    := 0;
   Byte_Acc               : Unsigned_8 := 0;
   Zr, Zi, Cr, Ci, Tr, Ti, Zr2, Zi2 : Real;
begin
   Width := Positive'Value (Argument (1));

   Height := Width;

   Put_Line ("P4");
   Put_Line (Argument (1) & " " & Argument (1));

   for Y in 0 .. Height - 1 loop
      for X in 0 .. Width - 1 loop
         Zr := 0.0;
         Zi := 0.0;
         Cr := 2.0 * Real (X) / Real (Width) - 1.5;
         Ci := 2.0 * Real (Y) / Real (Height) - 1.0;

         for I in 1 .. Iter + 1 loop
            Zr2 := Zr ** 2;
            Zi2 := Zi ** 2;
            Tr  := Zr2 - Zi2 + Cr;
            Ti  := 2.0 * Zr * Zi + Ci;
            Zr  := Tr;
            Zi  := Ti;

            exit when Zr2 + Zi2 > Limit;
         end loop;
         
         if Zr2 + Zi2 > Limit then
            Byte_Acc := Shift_Left (Byte_Acc, 1) or 16#00#;
         else
            Byte_Acc := Shift_Left (Byte_Acc, 1) or 16#01#;
         end if;

         Bit_Num := Bit_Num + 1;

         if Bit_Num = 8 then
            Put (Character'Val (Byte_Acc));
            Byte_Acc := 0;
            Bit_Num  := 0;
         elsif X = Width - 1 then
            Byte_Acc := Shift_Left (Byte_Acc, 8 - (Width mod 8));
            Put (Character'Val (Byte_Acc));
            Byte_Acc := 0;
            Bit_Num  := 0;
         end if;
      end loop;
   end loop;
end Mandelbrot;
