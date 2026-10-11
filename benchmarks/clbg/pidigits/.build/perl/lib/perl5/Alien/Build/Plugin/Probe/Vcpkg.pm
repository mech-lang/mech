package Alien::Build::Plugin::Probe::Vcpkg;

use strict;
use warnings;
use 5.008004;
use Alien::Build::Plugin;

# ABSTRACT: Probe for system libraries using Vcpkg
our $VERSION = '2.88'; # VERSION


has '+name';
has 'lib';
has 'ffi_name';
has 'include';

sub init
{
  my($self, $meta) = @_;

  if(defined $self->include)
  {
    $meta->add_requires('configure' => 'Alien::Build::Plugin::Probe::Vcpkg' => '2.16' );
  }
  elsif(defined $self->ffi_name)
  {
    $meta->add_requires('configure' => 'Alien::Build::Plugin::Probe::Vcpkg' => '2.14' );
  }
  else
  {
    $meta->add_requires('configure' => 'Alien::Build::Plugin::Probe::Vcpkg' => '0' );
  }

  if($meta->prop->{platform}->{compiler_type} eq 'microsoft')
  {
    $meta->register_hook(
      probe => sub {
        my($build) = @_;

        $build->hook_prop->{probe_class} = __PACKAGE__;
        $build->hook_prop->{probe_instance_id} = $self->instance_id;

        eval {
          require Win32::Vcpkg;
          require Win32::Vcpkg::List;
          require Win32::Vcpkg::Package;
          Win32::Vcpkg->VERSION('0.02');
        };
        if(my $error = $@)
        {
          $build->log("unable to load Win32::Vcpkg: $error");
          return 'share';
        }

        my $package;
        if($self->name)
        {
          $package = Win32::Vcpkg::List->new
                                       ->search($self->name, include => $self->include);
        }
        elsif($self->lib)
        {
          $package = eval { Win32::Vcpkg::Package->new( lib => $self->lib, include => $self->include) };
          return 'share' if $@;
        }
        else
        {
          $build->log("you must provode either name or lib property for Probe::Vcpkg");
          return 'share';
        }

        my $version = $package->version;
        $version = 'unknown' unless defined $version;

        $build->install_prop->{plugin_probe_vcpkg}->{$self->instance_id} = {
          version  => $version,
          cflags   => $package->cflags,
          libs     => $package->libs,
        };
        $build->hook_prop->{version} = $version;
        $build->install_prop->{plugin_probe_vcpkg}->{$self->instance_id}->{ffi_name} = $self->ffi_name
          if defined $self->ffi_name;
        return 'system';
      },
    );

    $meta->register_hook(
      gather_system => sub {
        my($build) = @_;

        return if $build->hook_prop->{name} eq 'gather_system'
        &&        ($build->install_prop->{system_probe_instance_id} || '') ne $self->instance_id;

        if(my $c = $build->install_prop->{plugin_probe_vcpkg}->{$self->instance_id})
        {
          $build->runtime_prop->{version} = $c->{version} unless defined $build->runtime_prop->{version};
          $build->runtime_prop->{$_} = $c->{$_} for grep { defined $c->{$_} } qw( cflags libs ffi_name );
        }
      },
    );
  }
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::Plugin::Probe::Vcpkg - Probe for system libraries using Vcpkg

=head1 VERSION

version 2.88

=head1 SYNOPSIS

 use alienfile;
 
 plugin 'Probe::Vcpkg' => 'libffi';

=head1 DESCRIPTION

This plugin probe can be used to find "system" packages using Microsoft's C<Vcpkg> package manager for
Visual C++ builds of Perl.  C<Vcpkg> is a package manager for Visual C++ that includes a number of
open source packages.  Although C<Vcpkg> does also support Linux and macOS, this plugin does not
support finding C<Vcpkg> packages on those platforms.  For more details on C<Vcpkg>, see the project
github page here:

L<https://github.com/microsoft/vcpkg>

Here is the quick start guide for getting L<Alien::Build> to work with C<Vpkg>:

 # install Vcpkg
 C:\> git clone https://github.com/Microsoft/vcpkg.git
 C:\> cd vcpkg
 C:\vcpkg> .\bootstrap-vcpkg.bat
 C:\vcpkg> .\vcpkg integrate install
 
 # update PATH to include the bin directory
 # so that .DLL files can be found by Perl
 C:\vcpkg> path c:\vcpkg\installed\x64-windows\bin;%PATH%
 
 # install the packages that you want
 C:\vcpkg> .\vcpkg install libffi
 
 # install the alien that uses it
 C:\vcpkg> cpanm Alien::FFI

If you are using 32 bit build of Perl, then substitute C<x86-windows> for C<x64-windows>.  If you do
not want to add the C<bin> directory to the C<PATH>, then you can use C<x64-windows-static> instead,
which will provide static libraries.  (As of this writing static libraries for 32 bit Windows are not
available).  The main downside to using C<x64-windows-static> is that Aliens that require dynamic
libraries for FFI will not be installable.

If you do not want to install C<Vcpkg> user wide (the C<integrate install> command above), then you
can use the C<PERL_WIN32_VCPKG_ROOT> environment variable instead:

 # install Vcpkg
 C:\> git clone https://github.com/Microsoft/vcpkg.git
 C:\> cd vcpkg
 C:\vcpkg> .\bootstrap-vcpkg.bat
 C:\vcpkg> set PERL_WIN32_VCPKG_ROOT=c:\vcpkg

=head1 PROPERTIES

=head2 name

Specifies the name of the Vcpkg.  This should not be used with the C<lib> property below, choose only one.

This is the default property, so these two are equivalent:

 plugin 'Probe::Vcpkg' => (name => 'foo');

and

 plugin 'Probe::Vcpkg' => 'foo';

=head2 lib

Specifies the list of libraries that make up the Vcpkg.  This should not be used with the C<name> property
above, choose only one.  Note that using this detection method, the version number of the package will
not be automatically determined (since multiple packages could potentially make up the list of libraries),
so you need to determine the version number another way if you need it.

This must be an array reference.  Do not include the C<.lib> extension.

 plugin 'Probe::Vcpkg' => (lib => ['foo','bar']);

=head2 ffi_name

Specifies an alternate ffi_name for finding dynamic libraries.

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
