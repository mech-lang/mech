package Alien::Util;

use strict;
use warnings;
use Exporter qw( import );

# ABSTRACT: Alien Utilities used at build and runtime
our $VERSION = '2.88'; # VERSION


our @EXPORT_OK = qw( version_cmp );


# Sort::Versions isn't quite the same algorithm because it differs in
# behaviour with leading zeroes.
#   See also  https://dev.gentoo.org/~mgorny/pkg-config-spec.html#version-comparison
sub version_cmp {
  my @x = (shift =~ m/([0-9]+|[a-z]+)/ig);
  my @y = (shift =~ m/([0-9]+|[a-z]+)/ig);

  while(@x and @y) {
    my $x = shift @x; my $x_isnum = $x =~ m/[0-9]/;
    my $y = shift @y; my $y_isnum = $y =~ m/[0-9]/;

    if($x_isnum and $y_isnum) {
      # Numerical comparison
      return $x <=> $y if $x != $y;
    }
    elsif(!$x_isnum && !$y_isnum) {
      # Alphabetic comparison
      return $x cmp $y if $x ne $y;
    }
    else {
      # Of differing types, the numeric one is newer
      return $x_isnum - $y_isnum;
    }
  }

  # Equal so far; the longer is newer
  return @x <=> @y;
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Util - Alien Utilities used at build and runtime

=head1 VERSION

version 2.88

=head1 SYNOPSIS

 use Alien::Util qw( version_cmp );

=head1 DESCRIPTION

This module contains some functions used by both the L<Alien::Build> build-time and <Alien::Base>
run-time for Alien.

=head2 version_cmp

  $cmp = version_cmp($x, $y)

Comparison method used by L<Alien::Base/atleast_version>, L<Alien::Base/exact_version> and
L<Alien::Base/max_version>. May be useful to implement custom comparisons, or for
subclasses to overload to get different version comparison semantics than the
default rules, for packages that have some other rules than the F<pkg-config>
behaviour.

Should return a number less than, equal to, or greater than zero; similar in
behaviour to the C<< <=> >> and C<cmp> operators.

=head1 SEE ALSO

L<Alien::Base>, L<alienfile>, L<Alien::Build>

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
