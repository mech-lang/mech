
with GNAT.OS_Lib;
with Ada.Strings.Unbounded;

package body Block_Input is
   function Read return U.String_Access is
      Buffer : String(1..65536);
      Text : U.Unbounded_String;
      Got : Integer;
   begin
      loop
         Got := GNAT.OS_Lib.Read(GNAT.OS_Lib.Standin,Buffer'Address,Buffer'Length);
         exit when Got=0;
         if Got<0 then raise Constraint_Error; end if;
         U.Append(Text,Buffer(1..Got));
      end loop;
      return new String'(U.To_String(Text));
   end Read;
   procedure Open_Stdin is
   begin
      null;
   end Open_Stdin;
   procedure Close_Stdin is
   begin
      null;
   end Close_Stdin;
end Block_Input;
