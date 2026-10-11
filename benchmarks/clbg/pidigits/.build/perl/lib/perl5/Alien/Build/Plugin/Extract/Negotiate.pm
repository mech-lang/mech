package Alien::Build::Plugin::Extract::Negotiate;

use strict;
use warnings;
use 5.008004;
use Alien::Build::Plugin;
use Alien::Build::Plugin::Extract::ArchiveTar;
use Alien::Build::Plugin::Extract::ArchiveZip;
use Alien::Build::Plugin::Extract::CommandLine;
use Alien::Build::Plugin::Extract::Directory;

# ABSTRACT: Extraction negotiation plugin
our $VERSION = '2.88'; # VERSION


has '+format' => 'tar';

sub init
{
  my($self, $meta) = @_;

  my $format = $self->format;
  $format = 'tar.gz'  if $format eq 'tgz';
  $format = 'tar.bz2' if $format eq 'tbz';
  $format = 'tar.xz'  if $format eq 'txz';

  my $plugin = $self->pick($format);
  $meta->apply_plugin($plugin, format => $format);
  $self;
}


sub pick
{
  my(undef, $format) = @_;

  if($format =~ /^tar(\.(gz|bz2))?$/)
  {
    if(Alien::Build::Plugin::Extract::ArchiveTar->available($format))
    {
      return 'Extract::ArchiveTar';
    }
    else
    {
      return 'Extract::CommandLine';
    }
  }
  elsif($format eq 'zip')
  {
    # Archive::Zip is not that reliable.  But if it is already installed it is probably working
    if(Alien::Build::Plugin::Extract::ArchiveZip->available($format))
    {
      return 'Extract::ArchiveZip';
    }

    # If it isn't available, then use the command-line unzip.  Alien::unzip will be used
    # as necessary in environments where it isn't already installed.
    else
    {
      return 'Extract::CommandLine';
    }
  }
  elsif($format eq 'tar.xz')
  {
    # The windows version of tar.exe (which is based on BSD tar) will try to use external
    # program xz to decompress tar.xz files if it is available.  The default windows
    # install does not have this in the PATH, but if it IS in the PATH then it often
    # or always can hang, so the pure Perl Archive::Tar is more reliable on that platform,
    # but will require a newer version of Archive::Tar and IO::Uncompress::UnXz
    if(Alien::Build::Plugin::Extract::ArchiveTar->available('tar.xz') || $^O eq 'MSWin32')
    {
      return 'Extract::ArchiveTar';
    }
    else
    {
      return 'Extract::CommandLine';
    }
  }
  elsif($format eq 'tar.Z')
  {
    return 'Extract::CommandLine';
  }
  elsif($format eq 'd')
  {
    return 'Extract::Directory';
  }
  elsif($format eq 'f')
  {
    return 'Extract::File';
  }
  else
  {
    die "do not know how to handle format: $format";
  }
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::Plugin::Extract::Negotiate - Extraction negotiation plugin

=head1 VERSION

version 2.88

=head1 SYNOPSIS

 use alienfile;
 plugin 'Extract' => (
   format => 'tar.gz',
 );

=head1 DESCRIPTION

This is a negotiator plugin for extracting packages downloaded from the internet.
This plugin picks the best Extract plugin to do the actual work.  Which plugins are
picked depend on the properties you specify, your platform and environment.  It is
usually preferable to use a negotiator plugin rather than using a specific Extract
Plugin from your L<alienfile>.

=head1 PROPERTIES

=head2 format

The expected format for the download.  Possible values include:
C<tar>, C<tar.gz>, C<tar.bz2>, C<tar.xz>, C<zip>, C<d>.

=head1 METHODS

=head2 pick

 my $name = Alien::Build::Plugin::Extract::Negotiate->pick($format);

Returns the name of the best plugin for the given format.

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
