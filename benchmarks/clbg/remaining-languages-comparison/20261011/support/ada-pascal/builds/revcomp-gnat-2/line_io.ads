
package Line_IO is

   --  Stream I/O of lines of text

   pragma Elaborate_Body (Line_IO);

   Separator : constant String := (1 => ASCII.LF);

   procedure Put_Line (Item : String);

   procedure Put (Item : String);

   function Get_Line return String;

   procedure Close;  --  close output

end Line_IO;
