package Alien::Build::Plugin::Extract::ArchiveZip;

use strict;
use warnings;
use 5.008004;
use Alien::Build::Plugin;

# ABSTRACT: Plugin to extract a tarball using Archive::Zip
our $VERSION = '2.88'; # VERSION


has '+format' => 'zip';


sub handles
{
  my($class, $ext) = @_;

  return 1 if $ext eq 'zip';

  return 0;
}


sub available
{
  my(undef, $ext) = @_;

  !! ( $ext eq 'zip' && eval { require Archive::Zip; 1} );
}

sub init
{
  my($self, $meta) = @_;

  $meta->add_requires('share' => 'Archive::Zip' => 0);

  $meta->register_hook(
    extract => sub {
      my($build, $src) = @_;
      my $zip = Archive::Zip->new;
      $zip->read($src);
      $zip->extractTree;
    }
  );
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::Plugin::Extract::ArchiveZip - Plugin to extract a tarball using Archive::Zip

=head1 VERSION

version 2.88

=head1 SYNOPSIS

 use alienfile;
 plugin 'Extract::ArchiveZip' => (
   format => 'zip',
 );

=head1 DESCRIPTION

Note: in most case you will want to use L<Alien::Build::Plugin::Extract::Negotiate>
instead.  It picks the appropriate Extract plugin based on your platform and environment.
In some cases you may need to use this plugin directly instead.

B<Note>: Seriously do NOT use this plugin! L<Archive::Zip> is pretty unreliable and
breaks all-the-time.  If you use the negotiator plugin mentioned above, then it will
prefer installing L<Alien::unzip>, which is much more reliable than L<Archive::Zip>.

This plugin extracts from an archive in zip format using L<Archive::Zip>.

=head2 format

Gives a hint as to the expected format.  This should always be C<zip>.

=head1 METHODS

=head2 handles

 Alien::Build::Plugin::Extract::ArchiveZip->handles($ext);
 $plugin->handles($ext);

Returns true if the plugin is able to handle the archive of the
given format.

=head2 available

 Alien::Build::Plugin::Extract::ArchiveZip->available($ext);

Returns true if the plugin has what it needs right now to extract from the given format

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
