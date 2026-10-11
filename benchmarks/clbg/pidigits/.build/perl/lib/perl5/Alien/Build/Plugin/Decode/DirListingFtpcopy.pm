package Alien::Build::Plugin::Decode::DirListingFtpcopy;

use strict;
use warnings;
use 5.008004;
use Alien::Build::Plugin;
use File::Basename ();

# ABSTRACT: Plugin to extract links from a directory listing using ftpcopy
our $VERSION = '2.88'; # VERSION


sub init
{
  my($self, $meta) = @_;

  $meta->add_requires('share' => 'File::Listing::Ftpcopy' => 0);
  $meta->add_requires('share' => 'URI' => 0);

  $meta->register_hook( decode => sub {
    my(undef, $res) = @_;

    die "do not know how to decode @{[ $res->{type} ]}"
      unless $res->{type} eq 'dir_listing';

    my $base = URI->new($res->{base});

    return {
      type => 'list',
      list => [
        map {
          my($name) = @$_;
          my $basename = $name;
          $basename =~ s{/$}{};
          my %h = (
            filename => File::Basename::basename($basename),
            url      => URI->new_abs($name, $base)->as_string,
          );
          \%h;
        } File::Listing::Ftpcopy::parse_dir($res->{content})
      ],
    };
  });

  $self;
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::Plugin::Decode::DirListingFtpcopy - Plugin to extract links from a directory listing using ftpcopy

=head1 VERSION

version 2.88

=head1 SYNOPSIS

 use alienfile;
 plugin 'Decode::DirListingFtpcopy';

=head1 DESCRIPTION

Note: in most case you will want to use L<Alien::Build::Plugin::Download::Negotiate>
instead.  It picks the appropriate decode plugin based on your platform and environment.
In some cases you may need to use this plugin directly instead.

This plugin decodes a ftp file listing into a list of candidates for your Prefer plugin.
It is useful when fetching from an FTP server via L<Alien::Build::Plugin::Fetch::LWP>.
It is different from the similarly named L<Alien::Build::Plugin::Decode::DirListingFtpcopy>
in that it uses L<File::Listing::Ftpcopy> instead of L<File::Listing>.  The rationale for
the C<Ftpcopy> version is that it supports a different set of FTP servers, including
OpenVMS.  In most cases, however, you probably want to use the non C<Ftpcopy> version
since it is pure perl.

=head1 SEE ALSO

L<Alien::Build::Plugin::Download::Negotiate>, L<Alien::Build::Plugin::Decode::DirListing>, L<Alien::Build>, L<alienfile>, L<Alien::Build::MM>, L<Alien>

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
