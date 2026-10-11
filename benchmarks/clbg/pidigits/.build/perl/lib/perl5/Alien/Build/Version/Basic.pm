package Alien::Build::Version::Basic;

use strict;
use warnings;
use 5.008004;
use Carp ();
use Exporter qw( import );
use overload
  '<=>'    => sub { shift->cmp(@_) },
  'cmp'    => sub { shift->cmp(@_) },
  '""'     => sub { shift->as_string },
  bool     => sub { 1 },
  fallback => 1;

our @EXPORT_OK = qw( version );

# ABSTRACT: Very basic version object for Alien::Build
our $VERSION = '2.88'; # VERSION


sub new
{
  my($class, $value) = @_;
  $value =~ s/\.$//;  # trim trailing dot
  Carp::croak("invalud version: $value")
    unless $value =~ /^[0-9]+(\.[0-9]+)*$/;
  bless \$value, $class;
}


sub version ($)
{
  my($value) = @_;
  __PACKAGE__->new($value);
}


sub as_string
{
  my($self) = @_;
  "@{[ $$self ]}";
}


sub cmp
{
  my @x = split /\./, ${$_[0]};
  my @y = split /\./, ${ref($_[1]) ? $_[1] : version($_[1])};

  while(@x or @y)
  {
    my $x = (shift @x) || 0;
    my $y = (shift @y) || 0;
    return $x <=> $y if $x <=> $y;
  }

  0;
}


1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::Version::Basic - Very basic version object for Alien::Build

=head1 VERSION

version 2.88

=head1 SYNOPSIS

OO interface:

 use Alien::Build::Version::Basic;
 
 my $version = Alien::Build::Version::Basic->new('1.2.3');
 if($version > '1.2.2')  # true
 {
   ...
 }

Function interface:

 use Alien::Build::Version::Basic qw( version );
 
 if(version('1.2.3') > version('1.2.2')) # true
 {
   ...
 }
 
 my @sorted = sort map { version($_) } qw( 2.1 1.2.3 1.2.2 );
 # will come out in the order 1.2.2, 1.2.3, 2.1

=head1 DESCRIPTION

This module provides a very basic class for comparing versions.
This is already a crowded space on CPAN.  Parts of L<Alien::Build>
already use L<Sort::Versions>, which is fine for sorting versions.
Sometimes you need to compare to see if versions match exact I<values>,
and the best candidates (such as L<Sort::Versions> on CPAN compare
C<1.2.3.0> and C<1.2.3> as being different.  This class compares
those two as the same.

This class is also quite limited, in that it only works with version
schemes using a doted version numbers or real numbers with a fixed
number of digits.  Versions with: dashes, letters, hex digits, or
anything else are not supported.

This class overloads both C<E<lt>=E<gt>> and C<cmp> to compare the version in
the way that you would expect for version numbers.  This way you can
compare versions like numbers, or sort them using sort.

 if(version($v1) > version($v2))
 {
   ...
 }
 
 my @sorted = sort map { version($_) } @unsorted;

it also overloads C<""> to stringify as whatever string value you
passed to the constructor.

=head1 CONSTRUCTOR

=head2 new

 my $version = Alien::Build::Version::Basic->new($value);

This is the long form of the constructor, if you don't want to import
anything into your namespace.

=head2 version

 my $version = version($value);

This is the short form of the constructor, if you are sane.  It is
NOT exported by default so you will have to explicitly import it.

=head1 METHODS

=head2 as_string

 my $string = $version->as_string;
 my $string = "$version";

Returns the string representation of the version object.

=head2 cmp

 my $bool = $version->cmp($other);
 my $bool = $version <=> $other;
 my $bool = $version cmp $other;

Returns C<-1>, C<0> or C<1> just like the regular C<E<lt>=E<gt>> and C<cmp>
operators.  Although C<$version> must be a version object, C<$other> may
be either a version object, or a string that could be used to create a
valid version object.

=head1 SEE ALSO

=over 4

=item L<Sort::Versions>

Good, especially if you have to support rpm style versions (like C<1.2.3-2-b>)
or don't care if trailing zeros (C<1.2.3> vs C<1.2.3.0>) are treated as
different values.

=item L<version>

Problematic for historical reasons.

=back

=head1 AUTHOR

Author: Graham Ollis E<lt>plicease@cpan.orgE<gt>

Contributors:

Diab Jerius (DJERIUS)

Roy Storey (KIWIROY)

Ilya Pavlov

David Mertens (run4flat)

Mark Nunberg (mordy, mnunberg)

Christian Walde (Mithaldu)

Brian Wightman (MidLifeXis)

Zaki Mughal (zmughal)

mohawk (mohawk2, ETJ)

Vikas N Kumar (vikasnkumar)

Flavio Poletti (polettix)

Salvador Fandiño (salva)

Gianni Ceccarelli (dakkar)

Pavel Shaydo (zwon, trinitum)

Kang-min Liu (劉康民, gugod)

Nicholas Shipp (nshp)

Juan Julián Merelo Guervós (JJ)

Joel Berger (JBERGER)

Petr Písař (ppisar)

Lance Wicks (LANCEW)

Ahmad Fatoum (a3f, ATHREEF)

José Joaquín Atria (JJATRIA)

Duke Leto (LETO)

Shoichi Kaji (SKAJI)

Shawn Laffan (SLAFFAN)

Paul Evans (leonerd, PEVANS)

Håkon Hægland (hakonhagland, HAKONH)

nick nauwelaerts (INPHOBIA)

Florian Weimer

Marcel Telka (mtelka)

=head1 COPYRIGHT AND LICENSE

This software is copyright (c) 2011-2022 by Graham Ollis.

This is free software; you can redistribute it and/or modify it under
the same terms as the Perl 5 programming language system itself.

=cut
