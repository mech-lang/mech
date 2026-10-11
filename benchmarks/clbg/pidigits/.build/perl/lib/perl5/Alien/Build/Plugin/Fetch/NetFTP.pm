package Alien::Build::Plugin::Fetch::NetFTP;

use strict;
use warnings;
use 5.008004;
use Alien::Build::Plugin;
use Carp ();
use File::Temp ();
use Path::Tiny qw( path );

# ABSTRACT: Plugin for fetching files using Net::FTP
our $VERSION = '2.88'; # VERSION


has '+url' => '';


has ssl => 0;


has passive => 0;

sub init
{
  my($self, $meta) = @_;

  $meta->prop->{start_url} ||= $self->url;
  $self->url($meta->prop->{start_url});
  $self->url || Carp::croak('url is a required property');

  $meta->add_requires('share' => 'Net::FTP' => 0 );
  $meta->add_requires('share' => 'URI' => 0 );
  $meta->add_requires('share' => 'Alien::Build::Plugin::Fetch::NetFTP' => '0.61')
    if $self->passive;

  $meta->register_hook( fetch => sub {
    my($build, $url, %options) = @_;
    $url ||= $self->url;

    $build->log("plugin Fetch::NetFTP does not support http_headers option") if $options{http_headers};

    $url = URI->new($url);

    die "Fetch::NetFTP does not support @{[ $url->scheme ]}"
      unless $url->scheme eq 'ftp';

    $build->log("trying passive mode FTP first") if $self->passive;
    my $ftp = _ftp_connect($url, $self->passive);

    my $path = $url->path;

    unless($path =~ m!/$!)
    {
      my(@parts) = split /\//, $path;
      my $filename = pop @parts;
      my $dir      = join '/', @parts;

      my $path = eval {
        $ftp->cwd($dir) or die;
        my $tdir = File::Temp::tempdir( CLEANUP => 1);
        my $path = path("$tdir/$filename")->stringify;

        unless(eval { $ftp->get($filename, $path) }) # NAT problem? try to use passive mode
        {
          $ftp->quit;

          $build->log("switching to @{[ $self->passive ? 'active' : 'passive' ]} mode");
          $ftp = _ftp_connect($url, !$self->passive);

          $ftp->cwd($dir) or die;

          $ftp->get($filename, $path) or die;
        }

        $path;
      };

      if(defined $path)
      {
        return {
          type     => 'file',
          filename => $filename,
          path     => $path,
          protocol => 'ftp',
        };
      }

      $path .= "/";
    }

    $ftp->quit;
    $ftp = _ftp_connect($url, $self->passive);
    $ftp->cwd($path) or die "unable to fetch $url as either a directory or file";

    my $list = eval { $ftp->ls };
    unless(defined $list) # NAT problem? try to use passive mode
    {
      $ftp->quit;

      $build->log("switching to @{[ $self->passive ? 'active' : 'passive' ]} mode");
      $ftp = _ftp_connect($url, !$self->passive);

      $ftp->cwd($path) or die "unable to fetch $url as either a directory or file";

      $list = $ftp->ls;

      die "cannot list directory $path on $url" unless defined $list;
    }

    die "no files found at $url" unless @$list;

    $path .= '/' unless $path =~ /\/$/;

    return {
      type     => 'list',
      protocol => 'ftp',
      list     => [
        map {
          my $filename = $_;
          my $furl = $url->clone;
          $furl->path($path . $filename);
          my %h = (
            filename => $filename,
            url      => $furl->as_string,
          );
          \%h;
        } sort @$list,
      ],
    };

  });

  $self;
}

sub _ftp_connect {
  my $url = shift;
  my $is_passive = shift || 0;

  my $ftp = Net::FTP->new(
    $url->host, Port =>$url->port, Passive =>$is_passive,
  ) or die "error fetching $url: $@";

  $ftp->login($url->user, $url->password)
    or die "error on login $url: @{[ $ftp->message ]}";

  $ftp->binary;

  $ftp;
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::Plugin::Fetch::NetFTP - Plugin for fetching files using Net::FTP

=head1 VERSION

version 2.88

=head1 SYNOPSIS

 use alienfile;
 share {
   start_url 'ftp://ftp.gnu.org/gnu/make';
   plugin 'Fetch::NetFTP';
 };

=head1 DESCRIPTION

Note: in most case you will want to use L<Alien::Build::Plugin::Download::Negotiate>
instead.  It picks the appropriate fetch plugin based on your platform and environment.
In some cases you may need to use this plugin directly instead.

This fetch plugin fetches files and directory listings via the C<ftp>, protocol using
L<Net::FTP>.

=head1 PROPERTIES

=head2 url

The initial URL to fetch.  This may be a directory or the final file.

=head2 ssl

This property is for compatibility with other fetch plugins, but is not used.

=head2 passive

If set to true, try passive mode FIRST.  By default it will try an active mode, then
passive mode.

=head1 SEE ALSO

L<Alien::Build::Plugin::Download::Negotiate>, L<Alien::Build>, L<alienfile>, L<Alien::Build::MM>, L<Alien>

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
