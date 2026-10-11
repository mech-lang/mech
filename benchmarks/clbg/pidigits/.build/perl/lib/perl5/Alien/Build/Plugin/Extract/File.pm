package Alien::Build::Plugin::Extract::File;

use strict;
use warnings;
use 5.008004;
use Alien::Build::Plugin;
use Alien::Build::Util qw( _mirror );
use Path::Tiny ();

# ABSTRACT: Plugin to extract a downloaded file to a build directory
our $VERSION = '2.88'; # VERSION


has '+format' => 'f';


sub handles
{
  my(undef, $ext) = @_;
  $ext eq 'f' ? 1 : ();
}


sub available
{
  my(undef, $ext) = @_;
  __PACKAGE__->handles($ext);
}

sub init
{
  my($self, $meta) = @_;

  $meta->register_hook(
    extract => sub {
      my($build, $src) = @_;

      die "not a file: $src" unless -f $src;

      $src = Path::Tiny->new($src)->absolute->parent;;

      my $dst = Path::Tiny->new('.')->absolute;
      # Please note: _mirror and Alien::Build::Util are ONLY
      # allowed to be used by core plugins.  If you are writing
      # a non-core plugin it may be removed.  That is why it
      # is private.

      $build->log("extracting $src => $dst");
      _mirror $src => $dst, { verbose => 1 };
    }
  );
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::Plugin::Extract::File - Plugin to extract a downloaded file to a build directory

=head1 VERSION

version 2.88

=head1 SYNOPSIS

 use alienfile;
 plugin 'Extract::File';

=head1 DESCRIPTION

Some Download or Fetch plugins may produce a single file (usually an executable)
instead of an archive file.  This plugin is used to mirror the file from
the Download step into a fresh directory in the Extract step.

=head1 PROPERTIES

=head2 format

Should always set to C<f> (for file).

=head1 METHODS

=head2 handles

 Alien::Build::Plugin::Extract::File->handles($ext);
 $plugin->handles($ext);

Returns true if the plugin is able to handle the archive of the
given format.  Only returns true for C<f> (for file).

=head2 available

 Alien::Build::Plugin::Extract::File->available($ext);
 $plugin->available($ext);

Returns true if the plugin can extract the given format with
what is already installed.

=head1 SEE ALSO

L<Alien::Build::Plugin::Extract::Negotiate>, L<Alien::Build::Plugin::Extract::File>, L<Alien::Build>, L<alienfile>, L<Alien::Build::MM>, L<Alien>

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
