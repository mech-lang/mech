package Alien::Build::Plugin::Build::MSYS;

use strict;
use warnings;
use 5.008004;
use Alien::Build::Plugin;
use File::Which ();
use Env qw( @PATH );

# ABSTRACT: MSYS plugin for Alien::Build
our $VERSION = '2.88'; # VERSION


has msys_version   => '0.07';

sub init
{
  my($self, $meta) = @_;

  if($self->msys_version ne '0.07')
  {
    $meta->add_requires('configure' => 'Alien::Build::Plugin::Build::MSYS' => '0.84');
  }

  if(_win_and_needs_msys($meta))
  {
    $meta->add_requires('share' => 'Alien::MSYS' => $self->msys_version);

    $meta->around_hook(
      $_ => sub {
        my $orig = shift;
        my $build = shift;

        local $ENV{PATH} = $ENV{PATH};
        unshift @PATH, Alien::MSYS::msys_path();

        $orig->($build, @_);
      },
    ) for qw( build build_ffi test_share test_ffi );
  }


  if($^O eq 'MSWin32')
  {
    # Most likely if we are trying to build something unix-y and
    # we are using MSYS, then we want to use the make that comes
    # with MSYS.
    $meta->interpolator->replace_helper(
      make => sub { 'make' },
    );

  }

  $self;
}

sub _win_and_needs_msys
{
  my($meta) = @_;
  # check to see if we are running on windows.
  # if we are running on windows, check to see if
  # it is MSYS2, then we can just use that.  Otherwise
  # we are probably on Strawberry, or (less likely)
  # VC Perl, in which case we will still need Alien::MSYS
  return 0 unless $^O eq 'MSWin32';
  return 0 if $meta->prop->{platform}->{system_type} eq 'windows-mingw';
  return 1;
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::Plugin::Build::MSYS - MSYS plugin for Alien::Build

=head1 VERSION

version 2.88

=head1 SYNOPSIS

 use alienfile;
 plugin 'Build::MSYS';

=head1 DESCRIPTION

This plugin sets up the MSYS environment for your build on Windows.  It does
not do anything on non-windows platforms.  MSYS provides the essential tools
for building software that is normally expected in a UNIX or POSIX environment.
This like C<sh>, C<awk> and C<make>.  To provide MSYS, this plugin uses
L<Alien::MSYS>.

=head1 PROPERTIES

=head2 msys_version

The version of L<Alien::MSYS> required if it is deemed necessary.  If L<Alien::MSYS>
isn't needed (if running under Unix, or MSYS2, for example) this will do nothing.

=head1 HELPERS

=head2 make

 %{make}

On windows the default C<%{make}> helper is replace with the make that comes with
L<Alien::MSYS>.  This is almost certainly what you want, as most unix style make
projects will not build with C<nmake> or C<dmake> typically used by Perl on Windows.

=head1 SEE ALSO

L<Alien::Build::Plugin::Build::Autoconf>, L<Alien::Build::Plugin>, L<Alien::Build>, L<Alien::Base>, L<Alien>

L<http://www.mingw.org/wiki/MSYS>

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
