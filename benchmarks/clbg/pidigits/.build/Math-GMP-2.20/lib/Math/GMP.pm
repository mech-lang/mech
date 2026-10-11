package Math::GMP;

# Math::GMP, a Perl module for high-speed arbitrary size integer
# calculations
# Copyright (C) 2000-2008 James H. Turner
# Copyright (C) 2008-2009 Greg Sabino Mullane

# This library is free software; you can redistribute it and/or
# modify it under the terms of the GNU Library General Public
# License as published by the Free Software Foundation; either
# version 2 of the License, or (at your option) any later version.

# This library is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
# Library General Public License for more details.

# You should have received a copy of the GNU Library General Public
# License along with this library; if not, write to the Free
# Foundation, Inc., 59 Temple Place, Suite 330, Boston, MA  02111-1307  USA

# You can contact the author at chip@redhat.com, chipt@cpan.org, or by mail:

# Chip Turner
# Red Hat Inc.
# 2600 Meridian Park Blvd
# Durham, NC 27713

use strict;
use warnings;
use 5.010;
use Carp;
use vars qw(@ISA @EXPORT @EXPORT_OK $AUTOLOAD);

use overload (
	'""'  =>   sub { stringify($_[0]) },
	'0+'  =>   sub { $_[0] >= 0 ? uintify($_[0]) : intify($_[0]) },
	'bool' =>  sub { $_[0] != 0 },

	'<=>' =>   \&op_spaceship,
	'=='  =>   \&op_eq,

	'+'   =>   \&op_add,
	'-'   =>   \&op_sub,

	'&'   =>   \&band,
	'^'   =>   \&bxor,
	'|'   =>   \&bior,

    '<<' =>    \&blshift,
    '>>' =>    \&brshift,

	'%'   =>   \&op_mod,
	'**'  =>   sub { $_[2] ? op_pow($_[1], $_[0]) : op_pow($_[0], $_[1]) },
	'*'   =>   \&op_mul,
	'/'   =>   \&op_div,
);

require Exporter;
require DynaLoader;
require AutoLoader;

@ISA = qw(Exporter DynaLoader);
# Items to export into callers namespace by default. Note: do not export
# names by default without a very good reason. Use EXPORT_OK instead.
# Do not simply export all your public functions/methods/constants.

our $VERSION = '2.20';


bootstrap Math::GMP $VERSION;

use strict;
sub import {
	shift;
	return unless @_;
	die "unknown import: @_" unless @_ == 1 and $_[0] eq ':constant';
	overload::constant integer => sub { Math::GMP->new(shift) };
	return;
}


sub new {
	my $class = shift;
	my $ival = shift || 0;
	my $base = shift;

	$ival =~ s/\A\+//;
	$ival =~ tr/ _//d;
	if ($base) {
		return Math::GMP::new_from_scalar_with_base($ival, $base);
	}
    else {
		if ($ival =~ /[^\d\-xA-Fa-f]/)
        {
            die "Argument to Math::GMP->new is not a string representing an integer";
        }
		return Math::GMP::new_from_scalar($ival);
	}
}

BEGIN
	{
		*DESTROY = \&Math::GMP::destroy;
		*gcd = \&bgcd;
		*lcm = \&blcm;
	}

__END__

=pod

=encoding UTF-8

=head1 NAME

Math::GMP - High speed arbitrary size integer math

=head1 VERSION

version 2.20

=head1 SYNOPSIS

  use Math::GMP;
  my $n = Math::GMP->new('2');

  $n = $n ** (256*1024);
  $n = $n - 1;
  print "n is now $n\n";

=head1 DESCRIPTION

Math::GMP was designed to be a drop-in replacement both for
Math::BigInt and for regular integer arithmetic.  Unlike BigInt,
though, Math::GMP uses the GNU gmp library for all of its
calculations, as opposed to straight Perl functions.  This can result
in speed improvements.

The downside is that this module requires a C compiler to install -- a
small tradeoff in most cases. Also, this module is not 100% compatible
with Math::BigInt.

A Math::GMP object can be used just as a normal numeric scalar would
be -- the module overloads most of the normal arithmetic operators to
provide as seamless an interface as possible. However, if you need a
perfect interface, you can do the following:

  use Math::GMP qw(:constant);

  $n = 2 ** (256 * 1024);
  print "n is $n\n";

This would fail without the ':constant' since Perl would use normal
doubles to compute the 250,000 bit number, and thereby overflow it
into meaninglessness (smaller exponents yield less accurate data due
to floating point rounding).

=begin Removed

sub AUTOLOAD {
	# This AUTOLOAD is used to 'autoload' constants from the constant()
	# XS function.  If a constant is not found then control is passed
	# to the AUTOLOAD in AutoLoader.

	my $constname;
	($constname = $AUTOLOAD) =~ s/.*:://;
	croak '& not defined' if $constname eq 'constant';
	my $val = constant($constname, @_ ? $_[0] : 0);
	if ($! != 0) {
		if ($! =~ /Invalid/) {
			$AutoLoader::AUTOLOAD = $AUTOLOAD;
			goto &AutoLoader::AUTOLOAD;
		} else {
			croak "Your vendor has not defined Math::GMP macro $constname";
		}
	}
	no strict 'refs';			## no critic
	*$AUTOLOAD = sub () { $val };
	goto &$AUTOLOAD;
}

=end Removed

=head1 METHODS

Although the non-overload interface is not complete, the following
functions do exist:

=head2 new

  $x = Math::GMP->new(123);

Creates a new Math::GMP object from the passed string or scalar.

  $x = Math::GMP->new('abcd', 36);

Creates a new Math::GMP object from the first parameter which should
be represented in the base specified by the second parameter.

=head2 bfac

  $x = Math::GMP->new(5);
  my $val = $x->bfac();      # 1*2*3*4*5 = 120
  print $val;

Calculates the factorial of $x and returns the result.

=head2 my $val = $x->band($y, $swap)

  $x = Math::GMP->new(6);
  my $val = $x->band(3, 0);      # 0b110 & 0b11 = 1
  print $val;

Calculates the bit-wise AND of its two arguments and returns the result.
$swap should be provided but is ignored.

=head2 my $ret = $x->bxor($y, $swap);

  $x = Math::GMP->new(6);
  my $val = $x->bxor(3, 0);      # 0b110 ^ 0b11 = 0b101
  print $val;

Calculates the bit-wise XOR of its two arguments and returns the result.

=head2 my $ret = $x->bior($y, $swap);

  $x = Math::GMP->new(6);
  my $val = $x->bior(3);      # 0b110 | 0b11 = 0b111
  print $val;

Calculates the bit-wise OR of its two arguments and returns the result.

=head2 blshift

  $x = Math::GMP->new(0b11);
  my $result = $x->blshift(4, 0);
  # $result = 0b11 << 4 = 0b110000

Calculates the bit-wise left-shift of its two arguments and returns the
result. Second argument is swap.

=head2 brshift

  $x = Math::GMP->new(0b11001);
  my $result = $x->brshift(3, 0);
  # $result = 0b11001 << 3 = 0b11

Calculates the bit-wise right-shift of its two arguments and returns the
result. Second argument is swap.

=head2 bgcd

  my $x = Math::GMP->new(6);
  my $gcd = $x->bgcd(4);
  # 6 / 2 = 3, 4 / 2 = 2 => 2
  print $gcd

Returns the Greatest Common Divisor of the two arguments.

=head2 blcm

  my $x = Math::GMP->new(6);
  my $lcm = $x->blcm(4);      # 6 * 2 = 12, 4 * 3 = 12 => 12
  print $lcm;

Returns the Least Common Multiple of the two arguments.

=head2 bmodinv

  my $x = Math::GMP->new(5);
  my $modinv = $x->bmodinv(7);   # 5 * 3 == 1 (mod 7) => 3
  print $modinv;

Returns the modular inverse of $x (mod $y), if defined. This currently
returns 0 if there is no inverse (but that may change in the future).
Behaviour is undefined when $y is 0.

=head2 broot

  my $x = Math::GMP->new(100);
  my $root = $x->root(3);    # int(100 ** (1/3)) => 4
  print $root;

Returns the integer n'th root of its argument, given a positive integer n.

=head2 brootrem

  my $x = Math::GMP->new(100);
  my($root, $rem) = $x->rootrem(3); # 4 ** 3 + 36 = 100
  print "$x is $rem more than the cube of $root";

Returns the integer n'th root of its argument, and the difference such that
C< $root ** $n + $rem == $x >.

=head2 bsqrt

  my $x = Math::GMP->new(6);
  my $root = $x->bsqrt();      # int(sqrt(6)) => 2
  print $root;

Returns the integer square root of its argument.

=head2 bsqrtrem

  my $x = Math::GMP->new(7);
  my($root, $rem) = $x->sqrtrem(); # 2 ** 2 + 3 = 7
  print "$x is $rem more than the square of $root";

Returns the integer square root of its argument, and the difference such that
C< $root ** 2 + $rem == $x >.

=head2 is_perfect_power

  my $x = Math::GMP->new(100);
  my $is_power = $x->is_perfect_power();
  print "$x is " . ($is_power ? "" : "not ") . "a perfect power";

Returns C<TRUE> if its argument is a power, ie if there exist integers a
and b with b > 1 such that C< $x == $a ** $b >.

=head2 is_perfect_square

  my $x = Math::GMP->new(100);
  my $is_square = $x->is_perfect_square();
  print "$x is " . ($is_square ? "" : "not ") . "a perfect square";

Returns C<TRUE> if its argument is the square of an integer.

=head2 legendre

  $x = Math::GMP->new(6);
  my $ret = $x->legendre(3);

Returns the value of the Legendre symbol ($x/$y). The value is defined only
when $y is an odd prime; when the value is not defined, this currently
returns 0 (but that may change in the future).

=head2 jacobi

  my $x = Math::GMP->new(6);
  my $jacobi_verdict = $x->jacobi(3);

Returns the value of the Jacobi symbol ($x/$y). The value is defined only
when $y is odd; when the value is not defined, this currently returns 0
(but that may change in the future).

=head2 fibonacci

  my $fib = Math::GMP::fibonacci(16);

Calculates the n'th number in the Fibonacci sequence.

=head2 probab_prime

  my $x = Math::GMP->new(7);
  my $is_prime_verdict = $x->probab_prime(10);

Probabilistically determines if the number is a prime. Argument is the number
of checks to perform. Returns 0 if the number is definitely not a prime,
1 if it may be, and 2 if it definitely is a prime.

=head2 $x->add_ui_gmp($n)

Adds to $x and mutates it in-place. $n must be a regular non-GMP, positive,
integer.

=head2 ($quotient, $remainder) = $x->bdiv($y);

  my $x = Math::GMP->new(7);
  my ($quo, $rem) = $x->bdiv(3);

Returns both the division and the modulo of an integer division operation.

=head2 my $ret = $x->div_2exp_gmp($n);

  my $x = Math::GMP->new(200);
  my $ret = $x->div_2exp_gmp(2);

Returns a right-shift of the Math::GMP object by an unsigned regular integer.
Also look at blshift() .

=head2 my $str = $x->get_str_gmp($base)

  my $init_n = 3 * 7 + 2 * 7 * 7 + 6 * 7 * 7 * 7;
  my $x = Math::GMP->new($init_n);
  my $ret = $x->get_str_gmp(7);

  print $ret; # Prints "6230".

Returns a string representation of the number in base $base.

=head2 my $clone = $x->gmp_copy()

Returns a copy of $x that can be modified without affecting the original.

=head2 my $verdict = $x->gmp_tstbit($bit_index);

Returns whether or not bit No. $bit_index is 1 in $x.

=head2 my $remainder = $dividend->mmod_gmp($divisor)

  my $x = Math::GMP->new(2 . ('0' x 200) . 4);
  my $y = Math::GMP->new(5);

  my $ret = $x->mmod_gmp($y);
  # $ret is now Math::GMP of 4.

From the GMP documentation:

Divide dividend and divisor and put the remainder in remainder. The remainder
is always positive, and its value is less than the value of the divisor.

=head2 my $result = $x->mod_2exp_gmp($shift);

  my $x = Math::GMP->new(0b10001011);
  my $ret = $x->mod_2exp_gmp(4);

  # $ret is now Math::GMP of 0b1011

Returns a Math::GMP object containing the lower $shift bits of $x (while not
modifying $x).

=head2 my $left_shifted = $x->mul_2exp_gmp($shift);

  my $x = Math::GMP->new(0b10001011);
  my $ret = $x->mul_2exp_gmp(4);

  # $ret is now Math::GMP of 0b1000_1011_0000

Returns a Math::GMP object containing $x shifted by $shift bits
(where $shift is a plain integer).

=head2 my $ret = $base->powm_gmp($exp, $mod);

    my $base = Math::GMP->new(157);
    my $exp = Math::GMP->new(100);
    my $mod = Math::GMP->new(5013);

    my $ret = $base->powm_gmp($exp, $mod);

    # $ret is now (($base ** $exp) % $mod)

Returns $base raised to the power of $exp modulo $mod.

=head2 my $plain_int_ret = $x->sizeinbase_gmp($plain_int_base);

Returns the size of $x in base $plain_int_base .

=head2 my $int = $x->intify();

Returns the value of the object as an unblessed (and limited-in-precision)
integer.

=head2 _gmp_build_version()

  my $gmp_version = Math::GMP::_gmp_build_version;
  if ($gmp_version ge 6.0.0) {
    print "Math::GMP was built against libgmp-6.0.0 or later";
  }

Class method that returns as a vstring the version of libgmp against which
this module was built.

=head2 _gmp_lib_version()

  my $gmp_version = Math::GMP::_gmp_lib_version;
  if ($gmp_version ge 6.0.0) {
    print "Math::GMP is now running with libgmp-6.0.0 or later";
  }

Class method that returns as a vstring the version of libgmp it is currently
running.

=head2 gcd()

An alias to bgcd() .

=head2 lcm()

An alias to blcm() .

=head2 constant

For internal use. B<Do not use directly>.

=head2 destroy

For internal use. B<Do not use directly>.

=head2 new_from_scalar

For internal use. B<Do not use directly>.

=head2 new_from_scalar_with_base

For internal use. B<Do not use directly>.

=head2 op_add

For internal use. B<Do not use directly>.

=head2 op_div

For internal use. B<Do not use directly>.

=head2 op_eq

For internal use. B<Do not use directly>.

=head2 op_mod

For internal use. B<Do not use directly>.

=head2 op_mul

For internal use. B<Do not use directly>.

=head2 op_pow

For internal use. B<Do not use directly>.

=head2 op_spaceship

For internal use. B<Do not use directly>.

=head2 op_sub

For internal use. B<Do not use directly>.

=head2 stringify

For internal use. B<Do not use directly>.

=head2 uintify

For internal use. B<Do not use directly>.

=head1 BUGS

As of version 1.0, Math::GMP is mostly compatible with the old
Math::BigInt version. It is not a full replacement for the rewritten
Math::BigInt versions, though. See the L<SEE ALSO section|SEE ALSO>
on how to achieve to use Math::GMP and retain full compatibility to
Math::BigInt.

There are some slight incompatibilities, such as output of positive
numbers not being prefixed by a '+' sign.  This is intentional.

There are also some things missing, and not everything might work as
expected.

=head1 VERSION CONTROL

The version control repository of this module is a git repository hosted
on GitHub at: L<https://github.com/turnstep/Math-GMP>. Pull requests are
welcome.

=head1 SEE ALSO

Math::BigInt has a new interface to use a different library than the
default pure Perl implementation. You can use, for instance, Math::GMP
with it:

  use Math::BigInt lib => 'GMP';

If Math::GMP is not installed, it will fall back to its own Perl
implementation.

See L<Math::BigInt> and L<Math::BigInt::GMP> or
L<Math::BigInt::Pari> or L<Math::BigInt::BitVect>.

=head1 AUTHOR

Chip Turner <chip@redhat.com>, based on the old Math::BigInt by Mark Biggar
and Ilya Zakharevich.  Further extensive work provided by Tels
<tels@bloodgate.com>.

=head1 AUTHOR

Shlomi Fish <shlomif@cpan.org>

=head1 COPYRIGHT AND LICENSE

This software is Copyright (c) 2000 by James H. Turner.

This is free software, licensed under:

  The GNU Lesser General Public License, Version 2.1, February 1999

=head1 BUGS

Please report any bugs or feature requests on the bugtracker website
L<https://rt.cpan.org/Public/Dist/Display.html?Name=Math-GMP> or by email
to L<bug-math-gmp@rt.cpan.org|mailto:bug-math-gmp@rt.cpan.org>.

When submitting a bug or request, please include a test-file or a
patch to an existing test-file that illustrates the bug or desired
feature.

=for :stopwords cpan testmatrix url bugtracker rt cpants kwalitee diff irc mailto metadata placeholders metacpan

=head1 SUPPORT

=head2 Perldoc

You can find documentation for this module with the perldoc command.

  perldoc Math::GMP

=head2 Websites

The following websites have more information about this module, and may be of help to you. As always,
in addition to those websites please use your favorite search engine to discover more resources.

=over 4

=item *

MetaCPAN

A modern, open-source CPAN search engine, useful to view POD in HTML format.

L<https://metacpan.org/release/Math-GMP>

=item *

RT: CPAN's Bug Tracker

The RT ( Request Tracker ) website is the default bug/issue tracking system for CPAN.

L<https://rt.cpan.org/Public/Dist/Display.html?Name=Math-GMP>

=item *

CPANTS

The CPANTS is a website that analyzes the Kwalitee ( code metrics ) of a distribution.

L<http://cpants.cpanauthors.org/dist/Math-GMP>

=item *

CPAN Testers

The CPAN Testers is a network of smoke testers who run automated tests on uploaded CPAN distributions.

L<http://www.cpantesters.org/distro/M/Math-GMP>

=item *

CPAN Testers Matrix

The CPAN Testers Matrix is a website that provides a visual overview of the test results for a distribution on various Perls/platforms.

L<http://matrix.cpantesters.org/?dist=Math-GMP>

=item *

CPAN Testers Dependencies

The CPAN Testers Dependencies is a website that shows a chart of the test results of all dependencies for a distribution.

L<http://deps.cpantesters.org/?module=Math::GMP>

=back

=head2 Bugs / Feature Requests

Please report any bugs or feature requests by email to C<bug-math-gmp at rt.cpan.org>, or through
the web interface at L<https://rt.cpan.org/Public/Bug/Report.html?Queue=Math-GMP>. You will be automatically notified of any
progress on the request by the system.

=head2 Source Code

The code is open to the world, and available for you to hack on. Please feel free to browse it and play
with it, or whatever. If you want to contribute patches, please send me a diff or prod me to pull
from your repository :)

L<https://github.com/turnstep/Math-GMP>

  git clone https://github.com/turnstep/Math-GMP.git

=cut
