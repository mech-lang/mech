#!/usr/bin/perl

use strict;
use warnings;

# See:
#
# https://rt.cpan.org/Ticket/Display.html?id=27521
#
# Thanks to Sisyphus for the report.

use Test::More tests => 1;

use Math::GMP;

{
    my $should_be_1;

    eval {
        $should_be_1 = Math::GMP->new(1.5);
    };

    my $E = $@;

    # TEST
    like ($E , qr/string representing an integer/);
}

