package Alien::Build::Plugin::Extract::CommandLine;

use strict;
use warnings;
use 5.008004;
use Alien::Build::Plugin;
use Path::Tiny ();
use File::Which ();
use File::chdir;
use File::Temp qw( tempdir );
use Capture::Tiny qw( capture_merged );

# ABSTRACT: Plugin to extract an archive using command line tools
our $VERSION = '2.88'; # VERSION


has '+format' => 'tar';


sub gzip_cmd
{
  _which('gzip') ? 'gzip' : undef;
}


sub _which { scalar File::Which::which(@_) }

sub bzip2_cmd
{
  _which('bzip2') ? 'bzip2' : undef;
}


sub xz_cmd
{
  _which('xz') ? 'xz' : undef;
}


{
  my $bsd_tar;

  # Note: GNU tar can be iffy to very bad on windows, where absolute
  # paths get confused with remote tars.  We used to assume that 'tar.exe'
  # is borked on Windows, but recent versions of Windows 10 come bundled
  # with bsdtar (libarchive) named 'tar.exe', and we should definitely
  # prefer that to ptar.
  sub _windows_tar_is_bsdtar
  {
    return 1 if $^O ne 'MSWin32';
    return $bsd_tar if defined $bsd_tar;
    my($out) = capture_merged {
      system 'tar', '--version';
    };
    return $bsd_tar = $out =~ /bsdtar/ ? 1 : 0
  }
}

sub tar_cmd
{
  _which('bsdtar')
    ? 'bsdtar'
    # Slowlaris /usr/bin/tar doesn't seem to like pax global header
    # but seems to have gtar in the path by default, which is okay with it
    : $^O eq 'solaris' && _which('gtar')
      ? 'gtar'
      # See note above for Windows logic.
      : _which('tar') && _windows_tar_is_bsdtar()
        ? 'tar'
        : _which('ptar')
          ? 'ptar'
          : undef;
};


sub unzip_cmd
{
  if($^O eq 'MSWin32' && _which('tar') && _windows_tar_is_bsdtar())
  {
    (_which('tar'), 'xf');
  }
  else
  {
    _which('unzip') ? 'unzip' : undef;
  }
}

sub _run
{
  my(undef, $build, @cmd) = @_;
  $build->log("+ @cmd");
  system @cmd;
  die "execute failed" if $?;
}

sub _cp
{
  my(undef, $build, $from, $to) = @_;
  require File::Copy;
  $build->log("copy $from => $to");
  File::Copy::cp($from, $to) || die "unable to copy: $!";
}

sub _mv
{
  my(undef, $build, $from, $to) = @_;
  $build->log("move $from => $to");
  rename($from, $to) || die "unable to rename: $!";
}

sub _dcon
{
  my($self, $src) = @_;

  my $name;
  my $cmd;

  if($src =~ /\.(gz|tgz|Z|taz)$/)
  {
    $self->gzip_cmd(_which('gzip')) unless defined $self->gzip_cmd;
    if($src =~ /\.(gz|tgz)$/)
    {
      $cmd = $self->gzip_cmd unless $self->_tar_can('tar.gz');
    }
    elsif($src =~ /\.(Z|taz)$/)
    {
      $cmd = $self->gzip_cmd unless $self->_tar_can('tar.Z');
    }
  }
  elsif($src =~ /\.(bz2|tbz)$/)
  {
    $self->bzip2_cmd(_which('bzip2')) unless defined $self->bzip2_cmd;
    $cmd = $self->bzip2_cmd unless $self->_tar_can('tar.bz2');
  }
  elsif($src =~ /\.(xz|txz)$/)
  {
    $self->xz_cmd(_which('xz')) unless defined $self->xz_cmd;
    $cmd = $self->xz_cmd unless $self->_tar_can('tar.xz');
  }

  if($cmd && $src =~ /\.(gz|bz2|xz|Z)$/)
  {
    $name = $src;
    $name =~ s/\.(gz|bz2|xz|Z)$//g;
  }
  elsif($cmd && $src =~ /\.(tgz|tbz|txz|taz)$/)
  {
    $name = $src;
    $name =~ s/\.(tgz|tbz|txz|taz)$/.tar/;
  }

  ($name,$cmd);
}


sub handles
{
  my($class, $ext) = @_;

  my $self = ref $class
  ? $class
  : __PACKAGE__->new;

  $ext = 'tar.Z'   if $ext eq 'taz';
  $ext = 'tar.gz'  if $ext eq 'tgz';
  $ext = 'tar.bz2' if $ext eq 'tbz';
  $ext = 'tar.xz'  if $ext eq 'txz';

  return 1 if $ext eq 'tar.gz'  && $self->_tar_can('tar.gz');
  return 1 if $ext eq 'tar.Z'   && $self->_tar_can('tar.Z');
  return 1 if $ext eq 'tar.bz2' && $self->_tar_can('tar.bz2');
  return 1 if $ext eq 'tar.xz'  && $self->_tar_can('tar.xz');

  return 0 if $ext =~ s/\.(gz|Z)$// && (!$self->gzip_cmd);
  return 0 if $ext =~ s/\.bz2$//    && (!$self->bzip2_cmd);
  return 0 if $ext =~ s/\.xz$//     && (!$self->xz_cmd);

  return 1 if $ext eq 'tar' && $self->_tar_can('tar');
  return 1 if $ext eq 'zip' && $self->_tar_can('zip');

  return 0;
}


sub available
{
  my(undef, $ext) = @_;

  # this is actually the same as handles
  __PACKAGE__->handles($ext);
}

sub init
{
  my($self, $meta) = @_;

  if($self->format eq 'tar.xz' && !$self->handles('tar.xz'))
  {
    $meta->add_requires('share' => 'Alien::xz' => '0.06');
  }
  elsif($self->format eq 'tar.bz2' && !$self->handles('tar.bz2'))
  {
    $meta->add_requires('share' => 'Alien::Libbz2' => '0.22');
  }
  elsif($self->format =~ /^tar\.(gz|Z)$/ && !$self->handles($self->format))
  {
    $meta->add_requires('share' => 'Alien::gzip' => '0.03');
  }
  elsif($self->format eq 'zip' && !$self->handles('zip'))
  {
    $meta->add_requires('share' => 'Alien::unzip' => '0');
  }

  $meta->register_hook(
    extract => sub {
      my($build, $src) = @_;

      my($dcon_name, $dcon_cmd) = _dcon($self, $src);

      if($dcon_name)
      {
        unless($dcon_cmd)
        {
          die "unable to decompress $src";
        }
        # if we have already decompressed, then keep it.
        unless(-f $dcon_name)
        {
          # we don't use pipes, because that may not work on Windows.
          # keep the original archive, in case another extract
          # plugin needs it.  keep the decompressed archive
          # in case WE need it again.
          my $src_tmp = Path::Tiny::path($src)
            ->parent
            ->child('x'.Path::Tiny::path($src)->basename);
          my $dcon_tmp = Path::Tiny::path($dcon_name)
            ->parent
            ->child('x'.Path::Tiny::path($dcon_name)->basename);
          $self->_cp($build, $src, $src_tmp);
          $self->_run($build, $dcon_cmd, "-d", $src_tmp);
          $self->_mv($build, $dcon_tmp, $dcon_name);
        }
        $src = $dcon_name;
      }

      if($src =~ /\.zip$/i)
      {
        $self->_run($build, $self->unzip_cmd, $src);
      }
      elsif($src =~ /\.tar/ || $src =~ /(\.tgz|\.tbz|\.txz|\.taz)$/i)
      {
        $self->_run($build, $self->tar_cmd, '-xf', $src);
      }
      else
      {
        die "not sure of archive type from extension";
      }
    }
  );
}

my %tars;

sub _tar_can
{
  my($self, $ext) = @_;

  unless(%tars)
  {
    my $name = '';
    local $_; # to avoid dynamically scoped read-only $_ from upper scopes
    while(my $line = <DATA>)
    {
      if($line =~ /^\[ (.*) \]$/)
      {
        $name = $1;
      }
      else
      {
        $tars{$name} .= $line;
      }
    }

    foreach my $key (keys %tars)
    {
      $tars{$key} = unpack "u", $tars{$key};
    }
  }

  my $name = "xx.$ext";

  return 0 unless $tars{$name};

  local $CWD = tempdir( CLEANUP => 1 );

  my $cleanup = sub {
    my $save = $CWD;
    unlink $name;
    unlink 'xx.txt';
    $CWD = '..';
    rmdir $save;
  };

  Path::Tiny->new($name)->spew_raw($tars{$name});

  my @cmd = ($self->tar_cmd, 'xf', $name);
  if($ext eq 'zip')
  {
    @cmd = ($self->unzip_cmd, $name);
  }

  my(undef, $exit) = capture_merged {
    system(@cmd);
    $?;
  };

  if($exit)
  {
    $cleanup->();
    return 0;
  }

  my $content = eval { Path::Tiny->new('xx.txt')->slurp };
  $cleanup->();

  return defined $content && $content eq "xx\n";
}

1;

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::Plugin::Extract::CommandLine - Plugin to extract an archive using command line tools

=head1 VERSION

version 2.88

=head1 SYNOPSIS

 use alienfile;
 plugin 'Extract::CommandLine' => (
   format => 'tar.gz',
 );

=head1 DESCRIPTION

Note: in most case you will want to use L<Alien::Build::Plugin::Extract::Negotiate>
instead.  It picks the appropriate Extract plugin based on your platform and environment.
In some cases you may need to use this plugin directly instead.

This plugin extracts from an archive in various formats using command line tools.

=head1 PROPERTIES

=head2 format

Gives a hint as to the expected format.

=head2 gzip_cmd

The C<gzip> command, if available.  C<undef> if not available.

=head2 bzip2_cmd

The C<bzip2> command, if available.  C<undef> if not available.

=head2 xz_cmd

The C<xz> command, if available.  C<undef> if not available.

=head2 tar_cmd

The C<tar> command, if available.  C<undef> if not available.

=head2 unzip_cmd

The C<unzip> command, if available.  C<undef> if not available.

=head1 METHODS

=head2 handles

 Alien::Build::Plugin::Extract::CommandLine->handles($ext);
 $plugin->handles($ext);

Returns true if the plugin is able to handle the archive of the
given format.

=head2 available

 Alien::Build::Plugin::Extract::CommandLine->available($ext);

Returns true if the plugin is available to extract without
installing anything new.

=head1 SEE ALSO

L<Alien::Build::Plugin::Extract::Negotiate>, L<Alien::Build>, L<alienfile>, L<Alien::Build::MM>, L<Alien>

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

__DATA__

[ xx.tar ]
M>'@N='AT````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M`````````````#`P,#8T-"``,#`P-S8U(``P,#`P,C0@`#`P,#`P,#`P,#`S
M(#$S-#,U,#0S-#(R(#`Q,C<P,P`@,```````````````````````````````
M````````````````````````````````````````````````````````````
M``````````````````````````````````````````!U<W1A<@`P,&]L;&ES
M9P``````````````````````````````````<W1A9F8`````````````````
M```````````````````P,#`P,#`@`#`P,#`P,"``````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M``````````````````````!X>`H`````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
M````````````````````````````````````````````````````````````
7````````````````````````````````


[ xx.tar.Z ]
M'YV0>/"XH(.'#H"#"!,J7,BPH<.'$"-*1`BCH@T:-$``J`CCAHT:&CG"D)%Q
MH\B3,T#$F$%C1@T8+6G(D`$"1@P9-V#,`%!SHL^?0(,*!5!G#ITP<DR^8<,F
MS9PS0Q<:#6/&3-2%)V&$/*GQJM>O8,.*'1I0P=BS:-.J7<NVK=NW<./*G4NW
7KMV[>//JW<NWK]^_@`,+'DRXL.'#0P$`


[ xx.tar.bz2 ]
M0EIH.3%!629365(,+ID``$A[D-$0`8!``7^``!!AI)Y`!```""``=!JGIBC3
M30&CU`]($HHTTR:`>D#0)SI*Z'R%H*J"&3@H]P@J>U$F5BMHOC`$L-"8C!(V
I"`'?*WA:(9*4U)@4)+"(V%.G]#W(_E6B'J8G]D`/Q=R13A0D%(,+ID``


[ xx.tar.gz ]
M'XL("!)'=%P``WAX+G1A<@"KJ-`KJ2AAH"DP,#`P,S%1`-'F9J9@VL`(PH<"
M8P5#8Q-C4P,38Q,C(P4#0R-S`V,&!0/:.@L"2HM+$HN`3LG/R<DL3L>M#J@L
E+0V/.1"/*,#I(0(J*K@&V@FC8!2,@E$P"@8````U:,3F``@`````


[ xx.tar.xz ]
M_3=Z6%H```3FUK1&`@`A`18```!T+^6CX`?_`&!=`#Q@M.AX.4O&N38V648.
M[J6L\\<_[3M*R;CASOTX?B.F\V:^)+G;\YY4"!4MLF9`*\N40G=O+K,J0"NF
M0VU7J%NN(A,R^DM8@/(_YGR5CAO+1CS_YNHE:,1!G%6L1\GT``"[$^?"O*"!
9`P`!?(`0````:OY*7K'$9_L"``````196@``


[ xx.zip ]
M4$L#!`H``````%5V64X:^I"B`P````,````&`!P`>'@N='AT550)``,21W1<
M$D=T7'5X"P`!!/4!```$%````'AX"E!+`0(>`PH``````%5V64X:^I"B`P``
M``,````&`!@```````$```"D@0````!X>"YT>'155`4``Q)'=%QU>`L``03U
>`0``!!0```!02P4&``````$``0!,````0P``````


