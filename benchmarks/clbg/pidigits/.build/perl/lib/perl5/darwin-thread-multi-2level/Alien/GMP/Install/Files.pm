package Alien::GMP::Install::Files;
use strict;
use warnings;
require Alien::GMP;
sub Inline { shift; Alien::GMP->Inline(@_) }
1;

=begin Pod::Coverage

  Inline

=cut
