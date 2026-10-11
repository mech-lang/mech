package Alien::Build::Plugin::PkgConfig::LibPkgConf;

use strict;
use warnings;
use 5.008004;
use Alien::Build::Plugin;
use Carp ();

# ABSTRACT: Probe system and determine library or tool properties using PkgConfig::LibPkgConf
our $VERSION = '2.88'; # VERSION


has '+pkg_name' => sub {
  Carp::croak "pkg_name is a required property";
};


has atleast_version => undef;


has exact_version => undef;


has max_version => undef;


has minimum_version => undef;

# private for now, used by negotiator
has register_prereqs => 1;


use constant _min_version => '0.04';

sub available
{
  !!eval { require PkgConfig::LibPkgConf; PkgConfig::LibPkgConf->VERSION(_min_version) };
}

sub init
{
  my($self, $meta) = @_;

  unless(defined $meta->prop->{env}->{PKG_CONFIG})
  {
    # TODO: this doesn't yet find pkgconf in the bin dir of a share
    # install.
    my $command_line =
      File::Which::which('pkgconf')
      ? 'pkgconf'
      : File::Which::which('pkg-config')
        ? 'pkg-config'
        : undef;
    $meta->prop->{env}->{PKG_CONFIG} = $command_line
      if defined $command_line;
  }

  if($self->register_prereqs)
  {
    # Also update in Neotiate.pm
    $meta->add_requires('configure' => 'PkgConfig::LibPkgConf::Client' => _min_version);

    if(defined $self->minimum_version || defined $self->atleast_version || defined $self->exact_version || defined $self->max_version)
    {
      $meta->add_requires('configure' => 'PkgConfig::LibPkgConf::Util' => _min_version);
    }
  }

  my($pkg_name, @alt_names) = (ref $self->pkg_name) ? (@{ $self->pkg_name }) : ($self->pkg_name);

  $meta->register_hook(
    probe => sub {
      my($build) = @_;
      $build->runtime_prop->{legacy}->{name} ||= $pkg_name;

      $build->hook_prop->{probe_class} = __PACKAGE__;
      $build->hook_prop->{probe_instance_id} = $self->instance_id;

      require PkgConfig::LibPkgConf::Client;
      my $client = PkgConfig::LibPkgConf::Client->new;
      my $pkg = $client->find($pkg_name);
      die "package $pkg_name not found" unless $pkg;

      $build->hook_prop->{version} = $pkg->version;
      my $atleast_version = $self->atleast_version;
      $atleast_version = $self->minimum_version unless defined $self->atleast_version;
      if($atleast_version)
      {
        require PkgConfig::LibPkgConf::Util;
        if(PkgConfig::LibPkgConf::Util::compare_version($pkg->version, $atleast_version) < 0)
        {
          die "package $pkg_name is version @{[ $pkg->version ]}, but at least $atleast_version is required.";
        }
      }

      if($self->exact_version)
      {
        require PkgConfig::LibPkgConf::Util;
        if(PkgConfig::LibPkgConf::Util::compare_version($pkg->version, $self->exact_version) != 0)
        {
          die "package $pkg_name is version @{[ $pkg->version ]}, but exactly @{[ $self->exact_version ]} is required.";
        }
      }

      if($self->max_version)
      {
        require PkgConfig::LibPkgConf::Util;
        if(PkgConfig::LibPkgConf::Util::compare_version($pkg->version, $self->max_version) > 0)
        {
          die "package $pkg_name is version @{[ $pkg->version ]}, but max @{[ $self->max_version ]} is required.";
        }
      }

      foreach my $alt (@alt_names)
      {
        my $pkg = $client->find($alt);
        die "package $alt not found" unless $pkg;
      }

      'system';
    },
  );

  $meta->register_hook(
    $_ => sub {
      my($build) = @_;

      return if $build->hook_prop->{name} eq 'gather_system'
      &&        ($build->install_prop->{system_probe_instance_id} || '') ne $self->instance_id;

      require PkgConfig::LibPkgConf::Client;
      my $client = PkgConfig::LibPkgConf::Client->new;

      foreach my $name ($pkg_name, @alt_names)
      {
        my $pkg = $client->find($name);
        die "reload of package $name failed" unless defined $pkg;

        my %prop;
        $prop{version}        = $pkg->version;
        $prop{cflags}         = $pkg->cflags;
        $prop{libs}           = $pkg->libs;
        $prop{cflags_static}  = $pkg->cflags_static;
        $prop{libs_static}    = $pkg->libs_static;
        $build->runtime_prop->{alt}->{$name} = \%prop;
      }

      foreach my $key (keys %{ $build->runtime_prop->{alt}->{$pkg_name} })
      {
        $build->runtime_prop->{$key} = $build->runtime_prop->{alt}->{$pkg_name}->{$key};
      }

      if(keys %{ $build->runtime_prop->{alt} } == 1)
      {
        delete $build->runtime_prop->{alt};
      }
    },
  ) for qw( gather_system gather_share );

  $self;
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::Plugin::PkgConfig::LibPkgConf - Probe system and determine library or tool properties using PkgConfig::LibPkgConf

=head1 VERSION

version 2.88

=head1 SYNOPSIS

 use alienfile;
 plugin 'PkgConfig::LibPkgConf' => (
   pkg_name => 'libfoo',
 );

=head1 DESCRIPTION

Note: in most case you will want to use L<Alien::Build::Plugin::PkgConfig::Negotiate>
instead.  It picks the appropriate fetch plugin based on your platform and environment.
In some cases you may need to use this plugin directly instead.

This plugin provides Probe and Gather steps for pkg-config based packages.  It uses
L<PkgConfig::LibPkgConf> to accomplish this task.

This plugin is part of the Alien::Build core For Now, but may be removed in a future
date.  While It Seemed Like A Good Idea at the time, it may not be appropriate to keep
it in core.  If it is spun off it will get its own distribution some time in the future.

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

=head2 available

 my $bool = Alien::Build::Plugin::PkgConfig::LibPkgConf->available;

Returns true if the necessary prereqs for this plugin are I<already> installed.

=head1 SEE ALSO

L<Alien::Build::Plugin::PkgConfig::Negotiate>, L<Alien::Build>, L<alienfile>, L<Alien::Build::MM>, L<Alien>

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
