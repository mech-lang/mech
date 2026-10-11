package Alien::Build::Plugin::Core::Setup;

use strict;
use warnings;
use 5.008004;
use Alien::Build::Plugin;
use Config;
use File::Which qw( which );

# ABSTRACT: Core setup plugin
our $VERSION = '2.88'; # VERSION


sub init
{
  my($self, $meta) = @_;
  $meta->prop->{platform} ||= {};
  $self->_platform($meta->prop->{platform});
}

sub _platform
{
  my(undef, $hash) = @_;

  if($^O eq 'MSWin32' && $Config{ccname} eq 'cl')
  {
    $hash->{compiler_type} = 'microsoft';
  }
  else
  {
    $hash->{compiler_type} = 'unix';
  }

  if($^O eq 'MSWin32')
  {
    $hash->{system_type} = 'windows-unknown';

    if(defined &Win32::BuildNumber)
    {
      $hash->{system_type} = 'windows-activestate';
    }
    elsif($Config{myuname} =~ /strawberry-perl/)
    {
      $hash->{system_type} = 'windows-strawberry';
    }
    elsif($hash->{compiler_type} eq 'microsoft')
    {
      $hash->{system_type} = 'windows-microsoft';
    }
    else
    {
      my $uname_exe = which('uname');
      if($uname_exe)
      {
        my $uname = `$uname_exe`;
        if($uname =~ /^(MINGW)(32|64)_NT/)
        {
          $hash->{system_type} = 'windows-' . lc $1;
        }
      }
    }
  }
  elsif($^O =~ /^(VMS)$/)
  {
    # others probably belong in here...
    $hash->{system_type} = lc $^O;
  }
  else
  {
    $hash->{system_type} = 'unix';
  }

  $hash->{cpu}{count} =
    exists $ENV{ALIEN_CPU_COUNT} && $ENV{ALIEN_CPU_COUNT} > 0
    ? $ENV{ALIEN_CPU_COUNT}
    : _cpu_count();

  $hash->{cpu}{arch} = _cpu_arch(\%Config);
}

# Retrieve number of available CPU cores. Adopted from
# <https://metacpan.org/release/MARIOROY/MCE-1.879/source/lib/MCE/Util.pm#L49>
# which is in turn adopted from Test::Smoke::Util with improvements.
sub _cpu_count {
  local $ENV{PATH} = $ENV{PATH};
  if( $^O ne 'MSWin32' ) {
    $ENV{PATH} = "/usr/sbin:/sbin:/usr/bin:/bin:$ENV{PATH}";
  }
  $ENV{PATH} =~ /(.*)/; $ENV{PATH} = $1;   ## Remove tainted'ness

  my $ncpu = 1;

  OS_CHECK: {
    local $_ = lc $^O;

    /linux/ && do {
      my ( $count, $fh );
      if ( open $fh, '<', '/proc/stat' ) {
        $count = grep { /^cpu\d/ } <$fh>;
        close $fh;
      }
      $ncpu = $count if $count;
      last OS_CHECK;
    };

    /bsd|darwin|dragonfly/ && do {
      chomp( my @output = `sysctl -n hw.ncpu 2>/dev/null` );
      $ncpu = $output[0] if @output;
      last OS_CHECK;
    };

    /aix/ && do {
      my @output = `lparstat -i 2>/dev/null | grep "^Online Virtual CPUs"`;
      if ( @output ) {
        $output[0] =~ /(\d+)\n$/;
        $ncpu = $1 if $1;
      }
      if ( !$ncpu ) {
        @output = `pmcycles -m 2>/dev/null`;
        if ( @output ) {
          $ncpu = scalar @output;
        } else {
          @output = `lsdev -Cc processor -S Available 2>/dev/null`;
          $ncpu = scalar @output if @output;
        }
      }
      last OS_CHECK;
    };

    /gnu/ && do {
      chomp( my @output = `nproc 2>/dev/null` );
      $ncpu = $output[0] if @output;
      last OS_CHECK;
    };

    /haiku/ && do {
      my @output = `sysinfo -cpu 2>/dev/null | grep "^CPU #"`;
      $ncpu = scalar @output if @output;
      last OS_CHECK;
    };

    /hp-?ux/ && do {
      my $count = grep { /^processor/ } `ioscan -fkC processor 2>/dev/null`;
      $ncpu = $count if $count;
      last OS_CHECK;
    };

    /irix/ && do {
      my @out = grep { /\s+processors?$/i } `hinv -c processor 2>/dev/null`;
      $ncpu = (split ' ', $out[0])[0] if @out;
      last OS_CHECK;
    };

    /osf|solaris|sunos|svr5|sco/ && do {
      if (-x '/usr/sbin/psrinfo') {
        my $count = grep { /on-?line/ } `psrinfo 2>/dev/null`;
        $ncpu = $count if $count;
      }
      else {
        my @output = grep { /^NumCPU = \d+/ } `uname -X 2>/dev/null`;
        $ncpu = (split ' ', $output[0])[2] if @output;
      }
      last OS_CHECK;
    };

    /mswin|mingw|msys|cygwin/ && do {
      if (exists $ENV{NUMBER_OF_PROCESSORS}) {
        $ncpu = $ENV{NUMBER_OF_PROCESSORS};
      }
      last OS_CHECK;
    };

    warn "CPU count: unknown operating system";
  }

  $ncpu = 1 if (!$ncpu || $ncpu < 1);

  $ncpu;
}

sub _cpu_arch {
  my ($my_config) = @_;

  my $arch = {};

  my %Config = %$my_config;

  die "Config missing archname" unless exists $Config{archname};
  die "Config missing ptrsize"  unless exists $Config{ptrsize};

  if( $Config{archname} =~ m/
      \b x64    \b # MSWin32-x64
    | \b x86_64 \b # x86_64-linux
    | \b amd64  \b # amd64-freebsd
    /ix) {
    $arch = { name => 'x86_64' };
  } elsif( $Config{archname} =~ m/
      \b x86  \b   # MSWin32-x86
    | \b i386 \b   # freebsd-i386
    | \b i486 \b   # i486-linux
    | \b i686 \b   # i686-cygwin
    | \b i86pc \b      # i86pc-solaris, i86pc-solaris-thread-multi-64
    /ix ) {
    $arch = { name => 'x86' };
  } elsif( $Config{archname} =~ m/
      \b darwin \b
    /ix ) {
    chomp( my $hw_machine = `sysctl -n hw.machine 2>/dev/null` );
    HW_MACHINE:
    for($hw_machine) {
      $_ eq 'arm64' && do {
        $arch = { name => 'aarch64' };
        last HW_MACHINE;
      };
      $_ eq 'x86_64' && do {
        $arch = { name => $Config{ptrsize} == 8 ? 'x86_64' : 'x86' };
        last HW_MACHINE;
      };
      $_ eq 'i386' && do {
        $arch = { name => 'x86' };
        last HW_MACHINE;
      };
      $_ eq 'Power Macintosh' && do {
        $arch = { name => $Config{ptrsize} == 8 ? 'ppc64' : 'ppc' };
        last HW_MACHINE;
      };

      warn "Architecture detection: unknown macOS arch hw.machine = $_, ptrsize = $Config{ptrsize}";
      $arch = { name => 'unknown' };
    }
  } elsif( $Config{archname} =~ /
      \b aarch64 \b
    | \b arm64 \b    # arm64-freebsd (FreeBSD can have either aarch64 or arm64)
    /ix ) {
    $arch = { name => 'aarch64' };   # ARM64
  } elsif( $Config{archname} =~ m/
      \b arm-linux-gnueabi \b
    /ix ) {
    # 32-bit ARM soft-float
    $arch = { name => 'armel' };
  } elsif( $Config{archname} =~ m/
      \b arm-linux-gnueabihf \b
    /ix ) {
    # 32-bit ARM hard-float
    $arch = { name => 'armhf' };
  }

  unless(exists $arch->{name}) {
    warn "Architecture detection: Unknown archname '$Config{archname}'.";
    $arch->{name} = 'unknown';
  }

  return $arch;
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::Plugin::Core::Setup - Core setup plugin

=head1 VERSION

version 2.88

=head1 SYNOPSIS

 use alienfile;
 # already loaded

=head1 DESCRIPTION

This plugin does some core setup for you.

=head1 SEE ALSO

L<Alien::Build>, L<Alien::Base::ModuleBuild>

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
