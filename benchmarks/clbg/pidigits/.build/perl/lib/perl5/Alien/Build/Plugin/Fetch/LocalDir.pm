package Alien::Build::Plugin::Fetch::LocalDir;

use strict;
use warnings;
use 5.008004;
use Alien::Build::Plugin;
use File::chdir;
use Path::Tiny ();

# ABSTRACT: Plugin for fetching a local directory
our $VERSION = '2.88'; # VERSION


has root => undef;


has ssl => 0;

sub init
{
  my($self, $meta) = @_;

  my $url = $meta->prop->{start_url} || 'patch';

  $meta->add_requires('configure' => 'Alien::Build::Plugin::Fetch::LocalDir' => '0.72' );

  if($url =~ /^file:/)
  {
    $meta->add_requires('share' => 'URI' => 0 );
    $meta->add_requires('share' => 'URI::file' => 0 );
  }

  {
    my $root = $self->root;
    if(defined $root)
    {
      $root = Path::Tiny->new($root)->absolute->stringify;
    }
    else
    {
      $root = "$CWD";
    }
    $self->root($root);
  }

  $meta->register_hook(
    fetch => sub {
      my($build, $path, %options) = @_;

      $build->log("plugin Fetch::LocalDir does not support http_headers option") if $options{http_headers};

      $path ||= $url;

      if($path =~ /^file:/)
      {
        my $root = URI::file->new($self->root);
        my $url  = URI->new_abs($path, $root);
        $path = $url->path;
        $path =~ s{^/([a-z]:)}{$1}i if $^O eq 'MSWin32';
      }

      $path = Path::Tiny->new($path)->absolute($self->root);

      if(-d $path)
      {
        return {
          type     => 'file',
          filename => $path->basename,
          path     => $path->stringify,
          tmp      => 0,
          protocol => 'file',
        };
      }
      else
      {
        $build->log("path $path is not a directory");
        $build->log("(you specified $url with root @{[ $self->root ]})");
        die "$path is not a directory";
      }
    }
  );
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::Plugin::Fetch::LocalDir - Plugin for fetching a local directory

=head1 VERSION

version 2.88

=head1 SYNOPSIS

 use alienfile;
 share {
   start_url 'patch/libfoo-1.00/';
   plugin 'Fetch::LocalDir';
 };

=head1 DESCRIPTION

Note: in most case you will want to use L<Alien::Build::Plugin::Download::Negotiate>
instead.  It picks the appropriate fetch plugin based on your platform and environment.
In some cases you may need to use this plugin directly instead.

This fetch plugin fetches files from the local file system.  It is mostly useful if you
intend to bundle source with your Alien.  If you are bundling tarballs see
L<Alien::Build::Plugin::Fetch::Local>.

=head1 PROPERTIES

=head2 root

The directory from which the start URL should be relative.  The default is usually reasonable.

=head2 ssl

This property is for compatibility with other fetch plugins, but is not used.

=head1 SEE ALSO

=over 4

=item L<Alien::Build::Plugin::Download::Negotiate>

=item L<Alien::Build::Plugin::Fetch::Local>

=item L<Alien::Build>

=item L<alienfile>

=item L<Alien::Build::MM>

=item L<Alien>

=back

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
