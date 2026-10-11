package Alien::Build::Plugin::Fetch::HTTPTiny;

use strict;
use warnings;
use 5.008004;
use Alien::Build::Plugin;
use File::Basename ();
use Alien::Build::Util qw( _ssl_reqs );
use Carp ();

# ABSTRACT: Plugin for fetching files using HTTP::Tiny
our $VERSION = '2.88'; # VERSION


has '+url' => '';


has ssl => 0;

# ignored for compatability
has bootstrap_ssl => 1;

sub init
{
  my($self, $meta) = @_;

  $meta->add_requires('share' => 'HTTP::Tiny'  => '0.044' );
  $meta->add_requires('share' => 'URI'         => '0'     );
  $meta->add_requires('share' => 'Mozilla::CA' => '0'     );

  $meta->prop->{start_url} ||= $self->url;
  $self->url($meta->prop->{start_url});
  $self->url || Carp::croak('url is a required property');

  if($self->url =~ /^https:/ || $self->ssl)
  {
    my $reqs = _ssl_reqs;
    foreach my $mod (sort keys %$reqs)
    {
      $meta->add_requires('share' => $mod => $reqs->{$mod});
    }
  }

  $meta->register_hook( fetch => sub {
    my($build, $url, %options) = @_;
    $url ||= $self->url;

    $url = URI->new($url) unless ref($url) && $url->isa('URI');

    my %headers;
    if(my $headers = $options{http_headers})
    {
      if(ref $headers eq 'ARRAY')
      {
        my @headers = @$headers;
        while(@headers)
        {
          my $key = shift @headers;
          my $value = shift @headers;
          unless(defined $key && defined $value)
          {
            $build->log("Fetch for $url with http_headers contains undef key or value");
            next;
          }
          push @{ $headers{$key} }, $value;
        }
      }
      else
      {
        $build->log("Fetch for $url with http_headers that is not an array reference");
      }
    }

    my $ua = HTTP::Tiny->new(
      agent      => "Alien-Build/@{[ $Alien::Build::VERSION || 'dev' ]} ",
      verify_SSL => $build->download_rule =~ /encrypt/ ? 1 : 0,
    );
    my $res = $ua->get($url, { headers => \%headers });

    unless($res->{success})
    {
      my $status = $res->{status} || '---';
      my $reason = $res->{reason} || 'unknown';

      $build->log("$status $reason fetching $url");
      if($status == 599)
      {
        $build->log("exception: $_") for split /\n/, $res->{content};

        my($can_ssl, $why_ssl) = HTTP::Tiny->can_ssl;
        if(! $can_ssl)
        {
          if($res->{redirects}) {
            foreach my $redirect (@{ $res->{redirects} })
            {
              if(defined $redirect->{headers}->{location} && $redirect->{headers}->{location} =~ /^https:/)
              {
                $build->log("An attempt at a SSL URL https was made, but your HTTP::Tiny does not appear to be able to use https.");
                $build->log("Please see: https://metacpan.org/pod/Alien::Build::Manual::FAQ#599-Internal-Exception-errors-downloading-packages-from-the-internet");
              }
            }
          }
        }
      }

      die "error fetching $url: $status $reason";
    }

    my($type) = split /;/, $res->{headers}->{'content-type'};
    $type = lc $type;
    my $base            = URI->new($res->{url});
    my $filename        = File::Basename::basename do { my $name = $base->path; $name =~ s{/$}{}; $name };

    # TODO: this doesn't get exercised by t/bin/httpd
    if(my $disposition = $res->{headers}->{"content-disposition"})
    {
      # Note: from memory without quotes does not match the spec,
      # but many servers actually return this sort of value.
      if($disposition =~ /filename="([^"]+)"/ || $disposition =~ /filename=([^\s]+)/)
      {
        $filename = $1;
      }
    }

    if($type eq 'text/html')
    {
      return {
        type     => 'html',
        base     => $base->as_string,
        content  => $res->{content},
        protocol => $url->scheme,
      };
    }
    else
    {
      return {
        type     => 'file',
        filename => $filename || 'downloadedfile',
        content  => $res->{content},
        protocol => $url->scheme,
      };
    }

  });

  $self;
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::Plugin::Fetch::HTTPTiny - Plugin for fetching files using HTTP::Tiny

=head1 VERSION

version 2.88

=head1 SYNOPSIS

 use alienfile;
 share {
   start_url 'http://ftp.gnu.org/gnu/make';
   plugin 'Fetch::HTTPTiny';
 };

=head1 DESCRIPTION

Note: in most case you will want to use L<Alien::Build::Plugin::Download::Negotiate>
instead.  It picks the appropriate fetch plugin based on your platform and environment.
In some cases you may need to use this plugin directly instead.

This fetch plugin fetches files and directory listings via the C<http> and C<https>
protocol using L<HTTP::Tiny>.  If the URL specified uses the C<https> scheme, then
the required SSL modules will automatically be injected as requirements.  If your
initial URL is not C<https>, but you know that it will be needed on a subsequent
request you can use the ssl property below.

=head1 PROPERTIES

=head2 url

The initial URL to fetch.  This may be a directory listing (in HTML) or the final file.

=head2 ssl

If set to true, then the SSL modules required to make an C<https> connection will be
added as prerequisites.

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
