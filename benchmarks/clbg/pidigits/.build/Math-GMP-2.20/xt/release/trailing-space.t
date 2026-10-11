#!perl

use strict;
use warnings;

use Test::More;

eval "use Test::TrailingSpace";
if ($@)
{
   plan skip_all => "Test::TrailingSpace required for trailing space test.";
}
else
{
   plan tests => 1;
}

# TODO: add .pod, .PL, the README/Changes/TODO/etc. documents and possibly
# some other stuff.
my $finder = Test::TrailingSpace->new(
   {
       root => '.',
       filename_regex => qr#(?:\.(?:t|pm|pl|xs|c|h|txt|pod|PL)|README|Changes|TODO|LICENSE)\z#,
   },
);

# TEST
$finder->no_trailing_space(
   "No trailing space was found."
);
