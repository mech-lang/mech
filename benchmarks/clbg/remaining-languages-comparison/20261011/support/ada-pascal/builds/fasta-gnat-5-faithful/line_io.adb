

with Ada.Streams.Stream_IO;
with Unchecked_Conversion;

package body Line_IO is

   use Ada.Streams;

   Stdout : Stream_IO.File_Type;

   procedure Print (Item : String) is
      subtype Index is Stream_Element_Offset range
         Stream_Element_Offset(Item'First) .. Stream_Element_Offset(Item'Last);
      subtype XString is String (Item'Range);
      subtype XBytes is Stream_Element_Array (Index);
      function To_Bytes is new Unchecked_Conversion
        (Source => XString,
         Target => XBytes);
   begin
      Stream_IO.Write (Stdout, To_Bytes (Item));
   end Print;

   procedure Close_Stdout is
   begin
      Stream_IO.Close (Stdout);
   end Close_Stdout;

   procedure Open_Stdout is
   begin
      Stream_IO.Open 
        (File => Stdout,
         Mode => Stream_IO.Out_File,
         Name => "/dev/stdout");
   end Open_Stdout;

end Line_IO;
