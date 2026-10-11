
package Line_IO is

   procedure Print (Item : String);

   procedure Close_Stdout;

   procedure Open_Stdout;

   End_of_Line : constant String := (1 => ASCII.LF);

   pragma Inline (Print);

end Line_IO;
