package Alien::Build::Util;

use strict;
use warnings;
use 5.008004;
use Exporter qw( import );
use Path::Tiny qw( path );
use Config;

# ABSTRACT: Private utility functions for Alien::Build
our $VERSION = '2.88'; # VERSION


our @EXPORT_OK = qw( _mirror _dump _destdir_prefix _perl_config _ssl_reqs _has_ssl );

# usage: _mirror $source_directory, $dest_direction, \%options
#
# options:
#  - filter -> regex for files that should match
#  - empty_directory -> if true, create all directories, including empty ones.
#  - verbose -> turn on verbosity

sub _mirror
{
  my($src_root, $dst_root, $opt) = @_;
  ($src_root, $dst_root) = map { path($_) } ($src_root, $dst_root);
  $opt ||= {};

  require Alien::Build;
  require File::Find;
  require File::Copy;

  File::Find::find({
    wanted => sub {
      next unless -e $File::Find::name;
      my $src = path($File::Find::name)->relative($src_root);
      return if $opt->{filter} && "$src" !~ $opt->{filter};
      return if "$src" eq '.';
      my $dst = $dst_root->child("$src");
      $src = $src->absolute($src_root);
      if(-l "$src")
      {
        unless(-d $dst->parent)
        {
          Alien::Build->log("mkdir -p @{[ $dst->parent ]}") if $opt->{verbose};
          $dst->parent->mkpath;
        }
        # TODO: rmtree if a directory?
        if(-e "$dst")
        { unlink "$dst" }
        my $target = readlink "$src";
        Alien::Build->log("ln -s $target $dst") if $opt->{verbose};
        symlink($target, $dst) || die "unable to symlink $target => $dst";
      }
      elsif(-d "$src")
      {
        if($opt->{empty_directory})
        {
          unless(-d $dst)
          {
            Alien::Build->log("mkdir $dst") if $opt->{verbose};
            mkdir($dst) || die "unable to create directory $dst: $!";
          }
        }
      }
      elsif(-f "$src")
      {
        unless(-d $dst->parent)
        {
          Alien::Build->log("mkdir -p @{[ $dst->parent ]}") if $opt->{verbose};
          $dst->parent->mkpath;
        }
        # TODO: rmtree if a directory?
        if(-e "$dst")
        { unlink "$dst" }
        Alien::Build->log("cp $src $dst") if $opt->{verbose};
        File::Copy::cp("$src", "$dst") || die "copy error $src => $dst: $!";
        if($] < 5.012 && -x "$src" && $^O ne 'MSWin32')
        {
          # apparently Perl 5.8 and 5.10 do not preserver perms
          my $mode = [stat "$src"]->[2] & oct(777);
          eval { chmod $mode, "$dst" };
        }
      }
    },
    no_chdir => 1,
  }, "$src_root");

  ();
}

sub _dump
{
  if(eval { require YAML })
  {
    return YAML::Dump(@_);
  }
  else
  {
    require Data::Dumper;
    return Data::Dumper::Dumper(@_);
  }
}

sub _destdir_prefix
{
  my($destdir, $prefix) = @_;
  $prefix =~ s{^/?([a-z]):}{$1}i if $^O eq 'MSWin32';
  path($destdir)->child($prefix)->stringify;
}

sub _perl_config
{
  my($key) = @_;
  $Config{$key};
}

sub _ssl_reqs
{
  return {
    'Net::SSLeay' => '1.49',
    'IO::Socket::SSL' => '1.56',
  };
}

sub _has_ssl
{
  my %reqs = %{ _ssl_reqs() };
  eval {
    require Net::SSLeay;
    die "need Net::SSLeay $reqs{'Net::SSLeay'}" unless Net::SSLeay->VERSION($reqs{'Net::SSLeay'});
    require IO::Socket::SSL;
    die "need IO::Socket::SSL $reqs{'IO::Socket::SSL'}" unless IO::Socket::SSL->VERSION($reqs{'IO::Socket::SSL'});
  };
  $@ eq '';
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::Util - Private utility functions for Alien::Build

=head1 VERSION

version 2.88

=head1 DESCRIPTION

This module contains some private utility functions used internally by
L<Alien::Build>.  It shouldn't be used by any distribution other than
C<Alien-Build>.  That includes L<Alien::Build> plugins that are not
part of the L<Alien::Build> core.

You have been warned.  The functionality within may be removed at
any time!

=head1 SEE ALSO

L<Alien::Build>

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
