package Alien::Build::Plugin::PkgConfig::Negotiate;

use strict;
use warnings;
use 5.008004;
use Alien::Build::Plugin;
use Alien::Build::Plugin::PkgConfig::PP;
use Alien::Build::Plugin::PkgConfig::LibPkgConf;
use Alien::Build::Plugin::PkgConfig::CommandLine;
use Alien::Build::Util qw( _perl_config );
use Carp ();

# ABSTRACT: Package configuration negotiation plugin
our $VERSION = '2.88'; # VERSION


has '+pkg_name' => sub {
  Carp::croak "pkg_name is a required property";
};


has atleast_version => undef;


has exact_version => undef;


has max_version => undef;


has minimum_version => undef;


sub pick
{
  my($class) = @_;

  return $ENV{ALIEN_BUILD_PKG_CONFIG} if $ENV{ALIEN_BUILD_PKG_CONFIG};

  if(Alien::Build::Plugin::PkgConfig::LibPkgConf->available)
  {
    return 'PkgConfig::LibPkgConf';
  }

  if(Alien::Build::Plugin::PkgConfig::CommandLine->available)
  {
    # TODO: determine environment or flags necessary for using pkg-config
    # on solaris 64 bit.
    # Some advice on pkg-config and 64 bit Solaris
    # https://docs.oracle.com/cd/E53394_01/html/E61689/gplhi.html
    my $is_solaris64 = (_perl_config('osname') eq 'solaris' && _perl_config('ptrsize') == 8);

    # PkgConfig.pm is more reliable on windows
    my $is_windows = _perl_config('osname') eq 'MSWin32';

    if(!$is_solaris64 && !$is_windows)
    {
      return 'PkgConfig::CommandLine';
    }
  }

  if(Alien::Build::Plugin::PkgConfig::PP->available)
  {
    return 'PkgConfig::PP';
  }
  else
  {
    # this is a fata error.  because we check for a pkg-config implementation
    # at configure time, we expect at least one of these to work.  (and we
    # fallback on installing PkgConfig.pm as a prereq if nothing else is avail).
    # we therefore expect at least one of these to work, if not, then the configuration
    # of the system has shifted from underneath us.
    Carp::croak("Could not find an appropriate pkg-config or pkgconf implementation, please install PkgConfig.pm, PkgConfig::LibPkgConf, pkg-config or pkgconf");
  }
}

sub init
{
  my($self, $meta) = @_;

  my $plugin = $self->pick;
  Alien::Build->log("Using PkgConfig plugin: $plugin");

  if(ref($self->pkg_name) eq 'ARRAY')
  {
    $meta->add_requires('configure', 'Alien::Build::Plugin::PkgConfig::Negotiate' => '0.79');
  }

  if($self->atleast_version || $self->exact_version || $self->max_version)
  {
    $meta->add_requires('configure', 'Alien::Build::Plugin::PkgConfig::Negotiate' => '1.53');
  }

  my @args;
  push @args, pkg_name         => $self->pkg_name;
  push @args, register_prereqs => 0;

  foreach my $method (map { "${_}_version" } qw( minimum atleast exact max ))
  {
    push @args, $method => $self->$method if defined $self->$method;
  }

  $meta->apply_plugin($plugin, @args);

  $self;
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::Plugin::PkgConfig::Negotiate - Package configuration negotiation plugin

=head1 VERSION

version 2.88

=head1 SYNOPSIS

 use alienfile;
 plugin 'PkgConfig' => (
   pkg_name => 'libfoo',
 );

=head1 DESCRIPTION

This plugin provides Probe and Gather steps for pkg-config based packages.  It picks
the best C<PkgConfig> plugin depending your platform and environment.

=head1 PROPERTIES

=head2 pkg_name

The package name.  If this is a list reference then .pc files with all those package
names must be present.  The first name will be the primary and used by default once
installed.  For the subsequent C<.pc> files you can use the
L<Alien::Base alt method|Alien::Base/alt> to retrieve the alternate configurations
once the L<Alien> is installed.

=head2 atleast_version

The minimum required version that is acceptable version as provided by the system.

=head2 exact_version

The exact required version that is acceptable version as provided by the system.

=head2 max_version

The max required version that is acceptable version as provided by the system.

=head2 minimum_version

Alias for C<atleast_version> for backward compatibility.

=head1 METHODS

=head2 pick

 my $name = Alien::Build::Plugin::PkgConfig::Negotiate->pick;

Returns the name of the negotiated plugin.

=head1 ENVIRONMENT

=over 4

=item ALIEN_BUILD_PKG_CONFIG

If set, this plugin will be used instead of the build in logic
which attempts to automatically pick the best plugin.

=back

=head1 SEE ALSO

L<Alien::Build>, L<alienfile>, L<Alien::Build::MM>, L<Alien>

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
