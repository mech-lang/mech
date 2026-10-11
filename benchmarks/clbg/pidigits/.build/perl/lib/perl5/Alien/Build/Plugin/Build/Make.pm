package Alien::Build::Plugin::Build::Make;

use strict;
use warnings;
use 5.008004;
use Carp ();
use Capture::Tiny qw( capture );
use Alien::Build::Plugin;

# ABSTRACT: Make plugin for Alien::Build
our $VERSION = '2.88'; # VERSION


has '+make_type' => undef;

sub init
{
  my($self, $meta) = @_;

  $meta->add_requires('configure', 'Alien::Build::Plugin::Build::Make', '0.99');

  my $type = $self->make_type;

  return unless defined $type;

  $type = 'gmake' if $^O eq 'MSWin32' && $type eq 'umake';

  if($type eq 'nmake')
  {
    $meta->interpolator->replace_helper( make => sub { 'nmake' } );
  }

  elsif($type eq 'dmake')
  {
    $meta->interpolator->replace_helper( make => sub { 'dmake' } );
  }

  elsif($type eq 'gmake')
  {
    my $found = 0;
    foreach my $make (qw( gmake make mingw32-make ))
    {
      my($out, $err) = capture { system $make, '--version' };
      if($out =~ /GNU Make/)
      {
        $meta->interpolator->replace_helper( make => sub { $make } );
        $found = 1;
      }
    }
    unless($found)
    {
      $meta->add_requires('share' => 'Alien::gmake' => '0.20');
      $meta->interpolator->replace_helper('make' => sub { require Alien::gmake; Alien::gmake->exe });
    }
  }

  elsif($type eq 'umake')
  {
    # nothing
  }

  else
  {
    Carp::croak("unknown make type = ", $self->make_type);
  }
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::Plugin::Build::Make - Make plugin for Alien::Build

=head1 VERSION

version 2.88

=head1 SYNOPSIS

 use alienfile;
 # For a recipe that requires GNU Make
 plugin 'Build::Make' => 'gmake';

=head1 DESCRIPTION

By default L<Alien::Build> provides a helper for the C<make> that is used by Perl and L<ExtUtils::MakeMaker> itself.
This is handy, because it is the one make that you can mostly guarantee that you will have.  Unfortunately it may be
a C<make> that isn't supported by the library or tool that you are trying to alienize.  This is mostly a problem on
Windows, where the supported C<make>s for years were Microsoft's C<nmake> and Sun's C<dmake>, which many open source
projects do not use.  This plugin will alter the L<alienfile> recipe to use a different C<make>.  It may (as in the
case of C<gmake> / L<Alien::gmake>) automatically download and install an alienized version of that C<make> if it
is not already installed.

This plugin should NOT be used with other plugins that replace the C<make> helper, like
L<Alien::Build::Plugin::Build::CMake>, L<Alien::Build::Plugin::Build::Autoconf>,
L<Alien::Build::Plugin::Build::MSYS>.  This plugin is intended instead for projects that use vanilla makefiles of
a specific type.

This plugin is for now distributed separately from L<Alien::Build>, but the intention is for it to soon become
a core plugin for L<Alien::Build>.

=head1 PROPERTIES

=head2 make_type

The make type needed by the L<alienfile> recipe:

=over 4

=item dmake

Sun's dmake.

=item gmake

GNU Make.

=item nmake

Microsoft's nmake.  It comes with Visual C++.

=item umake

Any UNIX C<make>  Usually either BSD or GNU Make.

=back

=head1 HELPERS

=head2 make

 %{make}

This plugin may change the make helper used by your L<alienfile> recipe.

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
