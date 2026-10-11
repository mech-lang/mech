# The Computer Language Benchmarks Game
# https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
#
# Contributed by David Eccles (gringer)
# substr tweaks by Richard Leach
# fixed by Isaac Gouy

use feature 'say';

local $/ = ">";
while (my $entry = <STDIN>) {
   chomp $entry;

   my $header = substr($entry, 0, index($entry, "\n"), "");
   next unless $header;
   substr($entry, 0, 1, "");

   {
      local $/ = "\n";
      say ">", $header;

      $entry =  reverse $entry;
      $entry =~ tr{wsatugcyrkmbdhvnATUGCYRKMBDHV\n}
              {WSTAACGRYMKVHDBNTAACGRYMKVHDB}d;

    say substr($entry,0,60,"") while ($entry);
   }
}
