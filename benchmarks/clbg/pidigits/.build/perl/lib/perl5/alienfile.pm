package alienfile;

use strict;
use warnings;
use 5.008004;
use Alien::Build;
use Exporter ();
use Path::Tiny ();
use Carp ();

sub _path { Path::Tiny::path(@_) }

# ABSTRACT: Specification for defining an external dependency for CPAN
our $VERSION = '2.88'; # VERSION


our @EXPORT = qw( requires on plugin probe configure share sys download fetch decode prefer extract patch patch_ffi build build_ffi gather gather_ffi meta_prop ffi log test start_url before after digest );


sub requires
{
  my($module, $version) = @_;
  $version ||= 0;
  my $caller = caller;
  my $meta = $caller->meta;
  $meta->add_requires($meta->{phase}, $module, $version);
  ();
}


sub plugin
{
  my($name, @args) = @_;

  my $caller = caller;
  $caller->meta->apply_plugin($name, @args);
  return;
}


sub probe
{
  my($instr) = @_;
  my $caller = caller;
  if(my $phase = $caller->meta->{phase})
  {
    Carp::croak "probe must not be in a $phase block" if $phase ne 'any';
  }
  $caller->meta->register_hook(probe => $instr);
  return;
}


sub _phase
{
  my($code, $phase) = @_;
  my $caller = caller(1);
  my $meta = $caller->meta;
  local $meta->{phase} = $phase;
  $code->();
  return;
}

sub configure (&)
{
  _phase($_[0], 'configure');
}


sub sys (&)
{
  _phase($_[0], 'system');
}



sub share (&)
{
  _phase($_[0], 'share');
}


sub _in_phase
{
  my($phase) = @_;
  my $caller = caller(1);
  my(undef, undef, undef, $sub) = caller(1);
  my $meta = $caller->meta;
  $sub =~ s/^.*:://;
  Carp::croak "$sub must be in a $phase block"
    unless $meta->{phase} eq $phase;
}

sub start_url
{
  my($url) = @_;
  _in_phase 'share';
  my $caller = caller;
  my $meta = $caller->meta;
  $meta->prop->{start_url} = $url;
  $meta->add_requires('configure' => 'Alien::Build' => '1.19');
  return;
}


sub digest
{
  my($algo, $digest) = @_;

  my $caller = caller;
  $caller->meta->apply_plugin('Digest', [$algo, $digest]);
  return;
}


sub download
{
  my($instr) = @_;
  _in_phase 'share';
  my $caller = caller;
  $caller->meta->register_hook(download => $instr);
  return;
}


sub fetch
{
  my($instr) = @_;
  _in_phase 'share';
  my $caller = caller;
  $caller->meta->register_hook(fetch => $instr);
  return;
}


sub decode
{
  my($instr) = @_;
  _in_phase 'share';
  my $caller = caller;
  $caller->meta->register_hook(decode => $instr);
  return;
}


sub prefer
{
  my($instr) = @_;
  _in_phase 'share';
  my $caller = caller;
  $caller->meta->register_hook(prefer => $instr);
  return;
}


sub extract
{
  my($instr) = @_;
  _in_phase 'share';
  my $caller = caller;
  $caller->meta->register_hook(extract => $instr);
  return;
}


sub patch
{
  my($instr) = @_;
  _in_phase 'share';
  my $caller = caller;
  my $suffix = $caller->meta->{build_suffix};
  $caller->meta->register_hook("patch$suffix" => $instr);
  return;
}


sub patch_ffi
{
  my($instr) = @_;
  Carp::carp("patch_ffi is deprecated, use ffi { patch ... } } instead");
  _in_phase 'share';
  my $caller = caller;
  $caller->meta->register_hook(patch_ffi => $instr);
  return;
}


sub build
{
  my($instr) = @_;
  _in_phase 'share';
  my $caller = caller;
  my $suffix = $caller->meta->{build_suffix};
  $caller->meta->register_hook("build$suffix" => $instr);
  return;
}


sub build_ffi
{
  my($instr) = @_;
  Carp::carp("build_ffi is deprecated, use ffi { build ... } } instead");
  _in_phase 'share';
  my $caller = caller;
  $caller->meta->register_hook(build_ffi => $instr);
  return;
}


sub gather
{
  my($instr) = @_;
  my $caller = caller;
  my $meta = $caller->meta;
  my $phase = $meta->{phase};
  Carp::croak "gather is not allowed in configure block"
    if $phase eq 'configure';
  my $suffix = $caller->meta->{build_suffix};
  if($suffix eq '_ffi')
  {
    $meta->register_hook(gather_ffi => $instr)
  }
  else
  {
    $meta->register_hook(gather_system => $instr) if $phase =~ /^(any|system)$/;
    $meta->register_hook(gather_share => $instr)  if $phase =~ /^(any|share)$/;
  }
  return;
}


sub gather_ffi
{
  my($instr) = @_;
  Carp::carp("gather_ffi is deprecated, use ffi { gather ... } } instead");
  _in_phase 'share';
  my $caller = caller;
  $caller->meta->register_hook(gather_ffi => $instr);
  return;
}


sub ffi (&)
{
  my($code) = @_;
  _in_phase 'share';
  my $caller = caller;
  local $caller->meta->{build_suffix} = '_ffi';
  $code->();
  return;
}


sub meta_prop
{
  my $caller = caller;
  my $meta = $caller->meta;
  $meta->prop;
}


sub log
{
  unshift @_, 'Alien::Build';
  goto &Alien::Build::log;
}


sub test
{
  my($instr) = @_;
  my $caller = caller;
  my $meta = $caller->meta;
  my $phase = $meta->{phase};
  Carp::croak "test is not allowed in $phase block"
    if $phase eq 'any' || $phase eq 'configure';

  $meta->add_requires('configure' => 'Alien::Build' => '1.14');

  if($phase eq 'share')
  {
    my $suffix = $caller->meta->{build_suffix} || '_share';
    $meta->register_hook(
      "test$suffix" => $instr,
    );
  }
  elsif($phase eq 'system')
  {
    $meta->register_hook(
      "test_system" => $instr,
    );
  }
  else
  {
    die "unknown phase: $phase";
  }
}


my %modifiers = (
  probe    => { any   => 'probe'    },
  download => { share => 'download' },
  fetch    => { share => 'fetch'    },
  decode   => { share => 'fetch'    },
  prefer   => { share => 'prefer'   },
  extract  => { share => 'extract'  },
  patch    => { share => 'patch$'   },
  build    => { share => 'build$'   },
  test     => { share => 'test$'    },
  # Note: below special case gather_ffi for the ffi block :P
  gather   => { share => 'gather_share', system => 'gather_system', any => 'gather_share,gather_system' },
);

sub _add_modifier
{
  my($type, $stage, $sub) = @_;

  my $method = "${type}_hook";

  Carp::croak "No such stage $stage" unless defined $modifiers{$stage};
  Carp::croak "$type $stage argument must be a code reference" unless defined $sub && ref($sub) eq 'CODE';

  my $caller = caller;
  my $meta = $caller->meta;
  Carp::croak "$type $stage is not allowed in sys block" unless defined $modifiers{$stage}->{$meta->{phase}};

  $meta->add_requires('configure' => 'Alien::Build' => '1.40');

  my $suffix = $meta->{build_suffix};
  if($suffix eq '_ffi' && $stage eq 'gather')
  {
    $meta->$method('gather_ffi' => $sub);
  }

  foreach my $hook (
    map { split /,/, $_ }                        # split on , for when multiple hooks must be attached (gather in any)
    map { my $x = $_ ; $x =~ s/\$/$suffix/; $x } # substitute $ at the end for a suffix (_ffi) if any
    $modifiers{$stage}->{$meta->{phase}})        # get the list of modifiers
  {
    $meta->$method($hook => $sub);
  }

  return;
}

sub before
{
  my($stage, $sub) = @_;
  @_ = ('before', @_);
  goto &alienfile::_add_modifier;
}


sub after
{
  my($stage, $sub) = @_;
  @_ = ('after', @_);
  goto &alienfile::_add_modifier;
}

sub import
{
  strict->import;
  warnings->import;
  goto &Exporter::import;
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

alienfile - Specification for defining an external dependency for CPAN

=head1 VERSION

version 2.88

=head1 SYNOPSIS

Do-it-yourself approach:

 use alienfile;
 
 probe [ 'pkg-config --exists libarchive' ];
 
 share {
 
   start_url 'http://libarchive.org/downloads/libarchive-3.2.2.tar.gz';
 
   # the first one which succeeds will be used
   download [ 'wget %{.meta.start_url}' ];
   download [ 'curl -o %{.meta.start_url}' ];
 
   extract [ 'tar xf %{.install.download}' ];
 
   build [
     # Note: will not work on Windows, better to use Build::Autoconf plugin
     # if you need windows support
     './configure --prefix=%{.install.prefix} --disable-shared',
     '%{make}',
     '%{make} install',
   ];
 }
 
 gather [
   [ 'pkg-config', '--modversion', 'libarchive', \'%{.runtime.version}' ],
   [ 'pkg-config', '--cflags',     'libarchive', \'%{.runtime.cflags}'  ],
   [ 'pkg-config', '--libs',       'libarchive', \'%{.runtime.libs}'    ],
 ];

With plugins (better):

 use alienfile;
 
 plugin 'PkgConfig' => 'libarchive';
 
 share {
   start_url 'http://libarchive.org/downloads/';
   plugin Download => (
     filter => qr/^libarchive-.*\.tar\.gz$/,
     version => qr/([0-9\.]+)/,
   );
   plugin Extract => 'tar.gz';
   plugin 'Build::Autoconf';
   plugin 'Gather::IsolateDynamic';
   build [
     '%{configure}',
     '%{make}',
     '%{make} install',
   ];
 };

=head1 DESCRIPTION

An alienfile is a recipe used by L<Alien::Build> to, probe for system libraries or download from the internet, and build source
for those libraries.  This document acts as reference for the alienfile system, but if you are starting out writing your own Alien
you should read L<Alien::Build::Manual::AlienAuthor>, which will teach you how to write your own complete Alien using alienfile +
L<Alien::Build> + L<ExtUtils::MakeMaker>.  Special attention should be taken to the section "a note about dynamic vs. static
libraries".

=head1 DIRECTIVES

=head2 requires

"any" requirement (either share or system):

 requires $module;
 requires $module => $version;

configure time requirement:

 configure {
   requires $module;
   requires $module => $version;
 };

system requirement:

 sys {
   requires $module;
   requires $module => $version;
 };

share requirement:

 share {
   requires $module;
   requires $module => $version;
 };

specifies a requirement.  L<Alien::Build> takes advantage of dynamic requirements, so only
modules that are needed for the specific type of install need to be loaded.  Here are the
different types of requirements:

=over

=item configure

Configure requirements should already be installed before the alienfile is loaded.

=item any

"Any" requirements are those that are needed either for the probe stage, or in either the
system or share installs.

=item share

Share requirements are those modules needed when downloading and building from source.

=item system

System requirements are those modules needed when the system provides the library or tool.

=back

=head2 plugin

 plugin $name => (%args);
 plugin $name => $arg;

Load the given plugin.  If you prefix the plugin name with an C<=> sign,
then it will be assumed to be a fully qualified path name.  Otherwise the
plugin will be assumed to live in the C<Alien::Build::Plugin> namespace.
If there is an appropriate negotiate plugin, that one will be loaded.
Examples:

 # Loads Alien::Build::Plugin::Fetch::Negotiate
 # which will pick the best Alien::Build::Plugin::Fetch
 # plugin based on the URL, and system configuration
 plugin 'Fetch' => 'http://ftp.gnu.org/gnu/gcc';
 
 # loads the plugin with the badly named class!
 plugin '=Badly::Named::Plugin::Not::In::Alien::Build::Namespace';
 
 # explicitly loads Alien::Build::Plugin::Prefer::SortVersions
 plugin 'Prefer::SortVersions' => (
   filter => qr/^gcc-.*\.tar\.gz$/,
   version => qr/([0-9\.]+)/,
 );

=head2 probe

 probe \&code;
 probe \@commandlist;

Instructions for the probe stage.  May be either a code reference, or a command list.
Multiple probes and probe plugins can be given.  These will be used in sequence,
stopping at the first that detects a system installation.  L<Alien::Build> will use
a share install if no system installation is detected by the probes.

=head2 configure

 configure {
   ...
 };

Configure block.  The only directive allowed in a configure block is
requires.

=head2 sys

 sys {
   ...
 };

System block.  Allowed directives are: requires and gather.

=head2 share

 share {
   ...
 };

System block.  Allowed directives are: download, fetch, decode, prefer, extract, build, gather.

=head2 start_url

 share {
   start_url $url;
 };

Set the start URL for download.  This should be the URL to an index page, or the actual tarball of the source.

=head2 digest

[experimental]

 share {
   digest $algorithm, $digest;
 };

Check fetched and downloaded files against the given algorithm and
digest.  Typically you will want to use SHA256 as the algorithm.

=head2 download

 share {
   download \&code;
   download \@commandlist;
 };

Instructions for the download stage.  May be either a
code reference, or a command list.

=head2 fetch

 share {
   fetch \&code;
   fetch \@commandlist;
 };

Instructions for the fetch stage.  May be either a
code reference, or a command list.

=head2 decode

 share {
   decode \&code;
   decode \@commandlist;
 };

Instructions for the decode stage.  May be either a
code reference, or a command list.

=head2 prefer

 share {
   prefer \&code;
   prefer \@commandlist;
 };

Instructions for the prefer stage.  May be either a
code reference, or a command list.

=head2 extract

 share {
   extract \&code;
   extract \@commandlist;
 };

Instructions for the extract stage.  May be either a
code reference, or a command list.

=head2 patch

 share {
   patch \&code;
   patch \@commandlist;
 };

Instructions for the patch stage.  May be either a
code reference, or a command list.

=head2 patch_ffi

 share {
   patch_ffi \&code;
   patch_ffi \@commandlist;
 };

[DEPRECATED]

Instructions for the patch_ffi stage.  May be either a
code reference, or a command list.

=head2 build

 share {
   build \&code;
   build \@commandlist;
 };

Instructions for the build stage.  May be either a
code reference, or a command list.

=head2 build_ffi

 share {
   build \&code;
   build \@commandlist;
 };

[DEPRECATED]

Instructions for the build FFI stage.  Builds shared libraries instead of static.
This is optional, and is only necessary if a fresh and separate build needs to be
done for FFI.

=head2 gather

 gather \&code;
 gather \@commandlist;
 
 share {
   gather \&code;
   gather \@commandlist;
 };
 
 sys {
   gather \&code;
   gather \@commandlist;
 };

Instructions for the gather stage.  May be either a code reference, or a command list.
In the root block of the alienfile it will trigger in both share and system build.
In the share or sys block it will only trigger in the corresponding build.

=head2 gather_ffi

 share {
   gather_ffi \&code;
   gather_ffi \@commandlist;
 }

[DEPRECATED]

Gather specific to C<build_ffi>.  Not usually necessary.

=head2 ffi

 share {
   ffi {
     patch \&code;
     patch \@commandlist;
     build \&code;
     build \@commandlist;
     gather \&code;
     gather \@commandlist;
   }
 }

Specify patch, build or gather stages related to FFI.

=head2 meta_prop

 my $hash = meta_prop;

Get the meta_prop hash reference.

=head2 meta

 my $meta = meta;

Returns the meta object for your L<alienfile>.  For methods that can be used on the
meta object, see L<Alien::Build/"META METHODS">.

=head2 log

 log($message);

Prints the given log to stdout.

=head2 test

 share {
   test \&code;
   test \@commandlist;
 };
 sys {
   test \&code;
   test \@commandlist;
 };

Run the tests

=head2 before

 before $stage => \&code;

Execute the given code before the given stage.  Stage should be one of
C<probe>, C<download>, C<fetch>, C<decode>, C<prefer>, C<extract>,
C<patch>, C<build>, C<test>, and C<gather>.

The before directive is only legal in the same blocks as the stage would
normally be legal in.  For example, you can't do this:

 use alienfile;
 
 sys {
   before 'build' => sub {
     ...
   };
 };

Because a C<build> wouldn't be legal inside a C<sys> block.

=head2 after

 after $stage => \&code;

Execute the given code after the given stage.  Stage should be one of
C<probe>, C<download>, C<fetch>, C<decode>, C<prefer>, C<extract>,
C<patch>, C<build>, C<test>, and C<gather>.

The after directive is only legal in the same blocks as the stage would
normally be legal in.  For example, you can't do this:

 use alienfile;
 
 sys {
   after 'build' => sub {
     ...
   };
 };

Because a C<build> wouldn't be legal inside a C<sys> block.

=head1 SEE ALSO

=over 4

=item L<Alien>

=item L<Alien::Build>

=item L<Alien::Build::MM>

=item L<Alien::Base>

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
