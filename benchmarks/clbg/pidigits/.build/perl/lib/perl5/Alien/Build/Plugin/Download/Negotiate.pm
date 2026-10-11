package Alien::Build::Plugin::Download::Negotiate;

use strict;
use warnings;
use 5.008004;
use Alien::Build::Plugin;
use Alien::Build::Util qw( _has_ssl );
use Carp ();

# ABSTRACT: Download negotiation plugin
our $VERSION = '2.88'; # VERSION


has '+url' => undef;


has 'filter'  => undef;


has 'version' => undef;


has 'ssl'     => 0;


has 'passive' => 0;

has 'scheme'  => undef;


has 'bootstrap_ssl' => 0;


has 'prefer' => 1;


has 'decoder' => undef;


sub pick
{
  my($self) = @_;
  my($fetch, @decoders) = $self->_pick;
  if($self->decoder)
  {
    @decoders = ref $self->decoder ? @{ $self->decoder } : ($self->decoder);
  }
  ($fetch, @decoders);
}

sub _pick_decoder
{
  my($self) = @_;

  if(eval { require Mojo::DOM58; Mojo::DOM58->VERSION(1.00); 1 })
  { return "Decode::Mojo" }
  elsif(eval { require Mojo::DOM; require Mojolicious; Mojolicious->VERSION('7.00'); 1 })
  { return "Decode::Mojo" }
  elsif(eval { require HTML::LinkExtor; 1; })
  { return "Decode::HTML" }
  else
  { return "Decode::Mojo" }
}

sub _pick
{
  my($self) = @_;

  $self->scheme(
    $self->url !~ m!(ftps?|https?|file):!i
      ? 'file'
      : $self->url =~ m!^([a-z]+):!i
  ) unless defined $self->scheme;

  if($self->scheme eq 'https' || ($self->scheme eq 'http' && $self->ssl))
  {
    if($self->bootstrap_ssl && ! _has_ssl)
    {
      return (['Fetch::CurlCommand','Fetch::Wget'], __PACKAGE__->_pick_decoder);
    }
    elsif(_has_ssl)
    {
      return ('Fetch::HTTPTiny', __PACKAGE__->_pick_decoder);
    }
    #elsif(do { require Alien::Build::Plugin::Fetch::CurlCommand; Alien::Build::Plugin::Fetch::CurlCommand->protocol_ok('https') })
    #{
    #  return ('Fetch::CurlCommand', __PACKAGE__->_pick_decoder);
    #}
    else
    {
      return ('Fetch::HTTPTiny', __PACKAGE__->_pick_decoder);
    }
  }
  elsif($self->scheme eq 'http')
  {
    return ('Fetch::HTTPTiny', __PACKAGE__->_pick_decoder);
  }
  elsif($self->scheme eq 'ftp')
  {
    if($ENV{ftp_proxy} || $ENV{all_proxy})
    {
      return $self->scheme =~ /^ftps?/
        ? ('Fetch::LWP', 'Decode::DirListing', __PACKAGE__->_pick_decoder)
        : ('Fetch::LWP', __PACKAGE__->_pick_decoder);
    }
    else
    {
      return ('Fetch::NetFTP');
    }
  }
  elsif($self->scheme eq 'file')
  {
    return ('Fetch::Local');
  }
  else
  {
    die "do not know how to handle scheme @{[ $self->scheme ]} for @{[ $self->url ]}";
  }
}

sub init
{
  my($self, $meta) = @_;

  unless(defined $self->url)
  {
    if(defined $meta->prop->{start_url})
    {
      $self->url($meta->prop->{start_url});
    }
    else
    {
      Carp::croak "url is a required property unless you use the start_url directive";
    }
  }

  if($self->url =~ /^http.*github.com.*releases$/ && !$meta->has_requires('configure', 'Download::GitHub'))
  {
    Alien::Build->log('!! WARNING !! WARNING !!');
    Alien::Build->log('!! WARNING !! It looks like this alien is using the regular download negotiator');
    Alien::Build->log('plugin on a GitHub release page.  This will typically not work due to changes');
    Alien::Build->log('in the way GitHub release page works now.  The Alien should instead be updated');
    Alien::Build->log('to use the Download::GitHub plugin, which uses the GitHub API to find available');
    Alien::Build->log('releases.  See: https://metacpan.org/pod/Alien::Build::Plugin::Download::GitHub');
    Alien::Build->log('!! WARNING !! WARNING !!');
  }

  $meta->add_requires('share' => 'Alien::Build::Plugin::Download::Negotiate' => '0.61')
    if $self->passive;

  $meta->prop->{plugin_download_negotiate_default_url} = $self->url;

  my($fetch, @decoders) = $self->pick;

  $fetch = [ $fetch ] unless ref $fetch;

  foreach my $fetch (@$fetch)
  {
    my @args;
    push @args, ssl => $self->ssl;
    # For historical reasons, we pass the URL into older fetch plugins, because
    # this used to be the interface.  Using start_url is now preferred!
    push @args, url => $self->url if $fetch =~ /^Fetch::(HTTPTiny|LWP|Local|LocalDir|NetFTP|CurlCommand)$/;
    push @args, passive => $self->passive if $fetch eq 'Fetch::NetFTP';
    push @args, bootstrap_ssl => $self->bootstrap_ssl if $self->bootstrap_ssl;

    $meta->apply_plugin($fetch, @args);
  }

  if($self->version)
  {
    $meta->apply_plugin($_) for @decoders;

    if(defined $self->prefer && ref($self->prefer) eq 'CODE')
    {
      $meta->add_requires('share' => 'Alien::Build::Plugin::Download::Negotiate' => '1.30');
      $meta->register_hook(
        prefer => $self->prefer,
      );
    }
    elsif($self->prefer)
    {
      $meta->apply_plugin('Prefer::SortVersions',
        (defined $self->filter ? (filter => $self->filter) : ()),
        version => $self->version,
      );
    }
    else
    {
      $meta->add_requires('share' => 'Alien::Build::Plugin::Download::Negotiate' => '1.30');
    }
  }
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::Plugin::Download::Negotiate - Download negotiation plugin

=head1 VERSION

version 2.88

=head1 SYNOPSIS

 use alienfile;
 share {
   start_url 'http://ftp.gnu.org/gnu/make';
   plugin 'Download' => (
     filter => qr/^make-.*\.tar\.gz$/,
     version => qr/([0-9\.]+)/,
   );
 };

=head1 DESCRIPTION

This is a negotiator plugin for downloading packages from the internet.  This
plugin picks the best Fetch, Decode and Prefer plugins to do the actual work.
Which plugins are picked depend on the properties you specify, your platform
and environment.  It is usually preferable to use a negotiator plugin rather
than the Fetch, Decode and Prefer plugins directly from your L<alienfile>.

=head1 PROPERTIES

=head2 url

[DEPRECATED] use C<start_url> instead.

The Initial URL for your package.  This may be a directory listing (either in
HTML or ftp listing format) or the final tarball intended to be downloaded.

=head2 filter

This is a regular expression that lets you filter out files that you do not
want to consider downloading.  For example, if the directory listing contained
tarballs and readme files like this:

 foo-1.0.0.tar.gz
 foo-1.0.0.readme

You could specify a filter of C<qr/\.tar\.gz$/> to make sure only tarballs are
considered for download.

=head2 version

Regular expression to parse out the version from a filename.  The regular expression
should store the result in C<$1>.

Note: if you provide a C<version> property, this plugin will assume that you will
be downloading an initial index to select package downloads from.  Depending on
the protocol (and typically this is the case for http and HTML) that may bring in
additional dependencies.  If start_url points to a tarball or other archive directly
(without needing to do through an index selection process), it is recommended that
you not specify this property.

=head2 ssl

If your initial URL does not need SSL, but you know ahead of time that a subsequent
request will need it (for example, if your directory listing is on C<http>, but includes
links to C<https> URLs), then you can set this property to true, and the appropriate
Perl SSL modules will be loaded.

=head2 passive

If using FTP, attempt a passive mode transfer first, before trying an active mode transfer.

=head2 bootstrap_ssl

If set to true, then the download negotiator will avoid using plugins that have a dependency
on L<Net::SSLeay>, or other Perl SSL modules.  The intent for this option is to allow
OpenSSL to be alienized and be a useful optional dependency for L<Net::SSLeay>.

The implementation may improve over time, but as of this writing, this option relies on you
having a working C<curl> or C<wget> with SSL support in your C<PATH>.

=head2 prefer

How to sort candidates for selection.  This should be one of three types of values:

=over 4

=item code reference

This will be used as the prefer hook.

=item true value

Use L<Alien::Build::Plugin::Prefer::SortVersions>.

=item false value

Don't set any preference at all.  A hook must be installed, or another prefer plugin specified.

=back

=head2 decoder

Override the detected decoder.

=head1 METHODS

=head2 pick

 my($fetch, @decoders) = $plugin->pick;

Returns the fetch plugin and any optional decoders that should be used.

=head1 SEE ALSO

L<Alien::Build::Plugin::Prefer::BadVersion>, L<Alien::Build::Plugin::Prefer::GoodVersion>

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
