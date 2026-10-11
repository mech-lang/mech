

with Ada.Unchecked_Deallocation;
with Ada.Finalization;

package Text_Input is

   -- Use Stream_IO to Read data from Standard_Input

   type String_Access is access String;
   type String_Pointer (Size : Positive) is new Ada.Finalization.Limited_Controlled with
       record
          Buffer : String_Access;
       end record;

   overriding procedure Initialize (Object : in out String_Pointer);
   overriding procedure Finalize (Object : in out String_Pointer);

   procedure Read_Section
     (Data_Buffer     : in out String_Access;
      Data_Length     :    out Natural;
      Next_Header     :    out String;
      Header_Length   :    out Natural;
      Max_Line_Length : in     Natural := 1024);

   Section_Marker : constant Character := '>';

   -- Read_Section reads until EOF or Section_Marker is found at start
   -- of a line. Can accept any line of length <= Max_Line_Length.

   procedure Free is new Ada.Unchecked_Deallocation (String, String_Access);

end Text_Input;
