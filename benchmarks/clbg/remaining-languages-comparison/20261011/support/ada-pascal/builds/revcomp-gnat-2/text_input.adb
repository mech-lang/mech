
with Ada.IO_Exceptions;
with Line_IO;

package body Text_Input is

   procedure Read_Section
     (Data_Buffer     : in out String_Access;
      Data_Length     :    out Natural;
      Next_Header     :    out String;
      Header_Length   :    out Natural;
      Max_Line_Length : in     Natural := 1024)
   is
      Ptr : String_Access;
   begin
      Data_Length   := 0;
      Header_Length := 0;

      Fill_Data_Buffer:
      loop
         if Data_Length + Max_Line_Length > Data_Buffer'Length then
            Ptr := Data_Buffer;
            Data_Buffer := new String (1 .. 2 * Data_Buffer'Length);
            Data_Buffer (1 .. Data_Length) := Ptr (1 .. Data_Length);
            Free (Ptr);
         end if;

         Get_Next_Line:
         declare
            Line : constant String := Line_IO.Get_Line;
            Present_Line_Length : constant Natural := Line'Length;
            Strt : Natural;
         begin

            if Present_Line_Length < 1 then
               Header_Length := 0;
               exit Fill_Data_Buffer;
            end if;

            if Present_Line_Length > Max_Line_Length then
               raise Program_Error;
            end if;

            if Line(Line'First) = Section_Marker then
               Strt := Next_Header'First;
               Next_Header(Strt .. Strt + Present_Line_Length - 1) := Line;
               Header_Length := Present_Line_Length;
               exit Fill_Data_Buffer;
            else
               Data_Buffer(Data_Length+1 .. Data_Length+Present_Line_Length):=Line;
               Data_Length := Data_Length + Present_Line_Length;
            end if;

         end Get_Next_Line;

      end loop Fill_Data_Buffer;

   exception
      when Ada.IO_Exceptions.End_Error =>
        return;
   end Read_Section;

   overriding procedure Initialize (Object : in out String_Pointer) is
   begin
      Object.Buffer := new String (1 .. Object.Size);
   end Initialize;

   overriding procedure Finalize (Object : in out String_Pointer) is
   begin
      Free (Object.Buffer);
   end Finalize;

end Text_Input;
