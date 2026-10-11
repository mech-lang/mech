#!/usr/bin/perl

use strict;
use warnings;

use Test::More tests => 1;

use Math::GMP (qw( :constant ));

{
    # TEST
    is ((2 ** 100 . ''), '1267650600228229401496703205376', "Test for :constant");
}

