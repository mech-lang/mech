package Alien::Build::Plugin::Build::CMake;

use strict;
use warnings;
use 5.008004;
use Config;
use Alien::Build::Plugin;
use Capture::Tiny qw( capture );

# ABSTRACT: CMake plugin for Alien::Build
our $VERSION = '2.88'; # VERSION


sub cmake_generator
{
  if($^O eq 'MSWin32')
  {
    return 'MinGW Makefiles' if is_dmake();

    {
      my($out, $err) = capture { system $Config{make}, '/?' };
      return 'NMake Makefiles' if $out =~ /NMAKE/;
    }

    {
      my($out, $err) = capture { system $Config{make}, '--version' };
      return 'MinGW Makefiles' if $out =~ /GNU Make/;
    }

    die 'make not detected';
  }
  else
  {
    return 'Unix Makefiles';
  }
}

sub init
{
  my($self, $meta) = @_;

  $meta->prop->{destdir} = $^O eq 'MSWin32' ? 0 : 1;

  $meta->add_requires('configure' => 'Alien::Build::Plugin::Build::CMake' => '0.99');
  $meta->add_requires('share'     => 'Alien::cmake3' => '0.02');

  if(is_dmake())
  {
    # even on at least some older versions of strawberry that do not
    # use it, come with gmake in the PATH.  So to save us the effort
    # of having to install Alien::gmake lets just use that version
    # if we can find it!
    my $found_gnu_make = 0;

    foreach my $exe (qw( gmake make mingw32-make ))
    {
      my($out, $err) = capture { system $exe, '--version' };
      if($out =~ /GNU Make/)
      {
        $meta->interpolator->replace_helper('make' => sub { $exe });
        $found_gnu_make = 1;
        last;
      }
    }

    if(!$found_gnu_make)
    {
      $meta->add_requires('share' => 'Alien::gmake' => '0.20');
      $meta->interpolator->replace_helper('make' => sub { require Alien::gmake; Alien::gmake->exe });
    }
  }

  $meta->interpolator->replace_helper('cmake' => sub { require Alien::cmake3; Alien::cmake3->exe });
  $meta->interpolator->add_helper('cmake_generator' => \&cmake_generator);

  my @args = (
    -G => '%{cmake_generator}',
    '-DCMAKE_POSITION_INDEPENDENT_CODE:BOOL=true',
    '-DCMAKE_INSTALL_PREFIX:PATH=%{.install.prefix}',
    '-DCMAKE_INSTALL_LIBDIR:PATH=lib',
    '-DCMAKE_MAKE_PROGRAM:PATH=%{make}',
  );

  $meta->prop->{plugin_build_cmake}->{args} = \@args;

  $meta->default_hook(
    build => [
      ['%{cmake}', @args, '%{.install.extract}' ],
      ['%{make}' ],
      ['%{make}', 'install' ],
    ],
  );

  # TODO: handle destdir on windows ??
}

my $is_dmake;

sub is_dmake
{
  unless(defined $is_dmake)
  {
    if($^O eq 'MSWin32')
    {
      my($out, $err) = capture { system $Config{make}, '-V' };
      $is_dmake = $out =~ /dmake/ ? 1 : 0;
    }
    else
    {
      $is_dmake = 0;
    }
  }

  $is_dmake;
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::Plugin::Build::CMake - CMake plugin for Alien::Build

=head1 VERSION

version 2.88

=head1 SYNOPSIS

 use alienfile;
 
 share {
   plugin 'Build::CMake';
   build [
     # this is the default build step, if you do not specify one.
     [ '%{cmake}',
         @{ meta->prop->{plugin_build_cmake}->{args} },
         # ... put extra cmake args here ...
         '%{.install.extract}'
     ],
     '%{make}',
     '%{make} install',
   ];
 };

=head1 DESCRIPTION

This plugin helps build alienized projects that use C<cmake>.
The intention is to make this a core L<Alien::Build> plugin if/when
it becomes stable enough.

This plugin provides a meta property C<plugin_build_cmake.args> which may change over time
but for the moment includes:

 -G %{cmake_generator}                          \
 -DCMAKE_POSITION_INDEPENDENT_CODE:BOOL=true    \
 -DCMAKE_INSTALL_PREFIX:PATH=%{.install.prefix} \
 -DCMAKE_INSTALL_LIBDIR:PATH=lib                \
 -DCMAKE_MAKE_PROGRAM:PATH=%{make}

This plugin supports out-of-source builds via the meta property C<out_of_source>.

=head1 METHODS

=head2 cmake_generator

Returns the C<cmake> generator according to your Perl's C<make>.

=head2 is_dmake

Returns true if your Perls C<make> appears to be C<dmake>.

=head1 HELPERS

=head2 cmake

This plugin replaces the default C<cmake> helper with the one that comes from L<Alien::cmake3>.

=head2 cmake_generator

This is the appropriate C<cmake> generator to use based on the make used by your Perl.  This is
frequently C<Unix Makefiles>.  One place where it may be different is if your Windows Perl uses
C<nmake>, which comes with Visual C++.

=head2 make

This plugin I<may> replace the default C<make> helper if the default C<make> is not supported by
C<cmake>.  This is most often an issue with older versions of Strawberry Perl which used C<dmake>.
On Perls that use C<dmake>, this plugin will search for GNU Make in the PATH, and if it can't be
found will fallback on using L<Alien::gmake>.

=head1 SEE ALSO

=over 4

=item L<Alien::Build>

=item L<Alien::Build::Plugin::Build::Autoconf>

=item L<alienfile>

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
