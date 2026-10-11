

with GNAT.Spitbol.Patterns;    use GNAT.Spitbol.Patterns;

package body Preprocessing is

   task body Removal is

      Sequence : U.String_Access;
      Start,
        Stop   : aliased Natural := 0;
      Last     : Natural         := 0;
      Tail     : Natural         := 0;
      --  Tail is also the value for Removal.Done.Last

      function Transfer return Boolean is
         --  puts good substrings in the resulting sequence
      begin
         if Start > Last then
            Sequence (Tail + 1 ..
                      Tail + 1 + (Start - Last) - 1) :=
              Input_Text (Last + 1 .. Start);
            Tail := Tail + (Start - Last);
         end if;
         Last := Stop;
         return Stop >= Input_Text'Length;
      end Transfer;

      Unwanted : constant Pattern :=
        (Setcur (Start'Access)
           & (('>' & Break (Separator)) or Separator)
           & Setcur (Stop'Access)
           & (+Transfer'Unrestricted_Access));

   begin
      accept Run (Clean : U.String_Access) do
         Sequence := Clean;
      end Run;

      Match (Input_Text.all, Pat => Unwanted);

      accept Done (Last : out Natural) do
         Last := Tail;
      end Done;
   end Removal;

end Preprocessing;
