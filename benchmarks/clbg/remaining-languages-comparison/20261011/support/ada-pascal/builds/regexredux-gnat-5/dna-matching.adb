
with Ada.Unchecked_Conversion, Ada.Synchronous_Task_Control;
package body DNA.Matching is

   task type Service (Sequence : U.String_Access) is
      --  matches one pattern concurrently

      entry Match_Variant (Variant : Variant_Index);
      entry Get (Number : out Natural);
   end Service;

   package Sem renames Ada.Synchronous_Task_Control;

   Ready         : Sem.Suspension_Object;
   Done          : Boolean                          := False with Volatile;
   No_Of_Matches : array (Variant_Index) of Integer := (others => -1);

   function Get (Variant : Variant_Index) return Natural is
   begin
      if not Done then
         Sem.Suspend_Until_True (Ready);
         Done := True;
      end if;
      return No_Of_Matches (Variant);
   end Get;

   procedure Count_Matches (Seq : U.String_Access; Limit : Positive) is
      subtype P is U.String_Access (1 .. Limit);
      function To_P is new Ada.Unchecked_Conversion (U.String_Access, P);
      Worker : array (Variant_Index) of Service (Sequence => To_P (Seq));
   begin
      --  assign tasks
      for Variant in Variant_Index loop
         Worker (Variant).Match_Variant (Variant);
      end loop;
      for Variant in Variant_Index loop
         Worker (Variant).Get (Number => No_Of_Matches (Variant));
      end loop;
      Sem.Set_True (Ready);
   end Count_Matches;

   task body Service is
      Count : Natural;

      function Inc_Count return Boolean is
         --  another occurrence of a pattern
      begin
         Count := Count + 1;
         return False;
      end Inc_Count;

      Variant : Variant_Index;
   begin  --  Service
      accept Match_Variant (Variant : Variant_Index) do
         Service.Variant := Variant;
      end Match_Variant;

      Count := 0;
      Match (Subject => Sequence.all,
             Pat => (Variant_Patterns (Variant)
                       & (+Inc_Count'Unrestricted_Access)));

      accept Get (Number : out Natural) do
         Number := Count;
      end Get;
   end Service;

end DNA.Matching;
