package Alien::Build::Plugin::Fetch::Wget;

use strict;
use warnings;
use 5.008004;
use Alien::Build::Plugin;
use File::Temp qw( tempdir );
use Path::Tiny qw( path );
use File::Which qw( which );
use Capture::Tiny qw( capture capture_merged );
use File::chdir;
use List::Util qw( pairmap );

# ABSTRACT: Plugin for fetching files using wget
our $VERSION = '2.88'; # VERSION


sub _wget
{
  my $wget = defined $ENV{WGET} ? which($ENV{WGET}) : which('wget');
  return undef unless defined $wget;
  my $output = capture_merged { system $wget, '--help' };

  # The wget that BusyBox implements does not follow that same interface
  # as GNU wget and may not check ssl certs which is not good.
  return undef if $output =~ /BusyBox/;

  # Wget2 (which some newer Linux distributions ship as /usr/bin/wget in
  # place of GNU Wget) does not follow the same interface as GNU Wget either
  # (in particular its -S output cannot be relied on to include the
  # Content-Type header), so treat it as not found, same as BusyBox.
  return undef if $output =~ /GNU Wget2/;

  return $wget;
}

has wget_command => sub { _wget() };
has ssl => 0;

# when bootstrapping we have to specify this plugin as a prereq
# 1 is the default so that when this plugin is used directly
# you also get the prereq
has bootstrap_ssl => 1;

sub init
{
  my($self, $meta) = @_;

  $meta->add_requires('configure', 'Alien::Build::Plugin::Fetch::Wget' => '1.19')
    if $self->bootstrap_ssl;

  $meta->register_hook(
    fetch => sub {
      my($build, $url, %options) = @_;
      $url ||= $meta->prop->{start_url};

      my($scheme) = $url =~ /^([a-z0-9]+):/i;

      if($scheme eq 'http' || $scheme eq 'https')
      {
        local $CWD = tempdir( CLEANUP => 1 );

        my @headers;
        if(my $headers = $options{http_headers})
        {
          if(ref $headers eq 'ARRAY')
          {
            my @copy = @$headers;
            my %headers;
            while(@copy)
            {
              my $key = shift @copy;
              my $value = shift @copy;
              push @{ $headers{$key} }, $value;
            }
            @headers = pairmap { "--header=$a: @{[ join ', ', @$b ]}" } %headers;
          }
          else
          {
            $build->log("Fetch for $url with http_headers that is not an array reference");
          }
        }

        my($stdout, $stderr) = $self->_execute(
          $build,
          $self->wget_command,
          '-k', '--content-disposition', '-S',
          @headers,
          $url,
        );

        my($path) = path('.')->children;
        die "no file found after wget" unless $path;
        my($type) = $stderr =~ /Content-Type:\s*(.*?)$/m;
        $type =~ s/;.*$// if $type;
        if($type eq 'text/html')
        {
          return {
            type     => 'html',
            base     => $url,
            content  => scalar $path->slurp,
            protocol => $scheme,
          };
        }
        else
        {
          return {
            type     => 'file',
            filename => $path->basename,
            path     => $path->absolute->stringify,
            protocol => $scheme,
          };
        }
      }
      else
      {
        die "scheme $scheme is not supported by the Fetch::Wget plugin";
      }
    },
  ) if $self->wget_command;
}

sub _execute
{
  my($self, $build, @command) = @_;
  $build->log("+ @command");
  my($stdout, $stderr, $err) = capture {
    system @command;
    $?;
  };
  if($err)
  {
    chomp $stderr;
    $stderr = [split /\n/, $stderr]->[-1];
    die "error in wget fetch: $stderr";
  }
  ($stdout, $stderr);
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::Plugin::Fetch::Wget - Plugin for fetching files using wget

=head1 VERSION

version 2.88

=head1 SYNOPSIS

 use alienfile;
 
 share {
   start_url 'https://www.openssl.org/source/';
   plugin 'Fetch::Wget';
 };

=head1 DESCRIPTION

B<WARNING>: This plugin is somewhat experimental at this time.

This plugin provides a fetch based on the C<wget> command.  It works with other fetch
plugins (that is, the first one which succeeds will be used).  Most of the time the best plugin
to use will be L<Alien::Build::Plugin::Download::Negotiate>, but for some SSL bootstrapping
it may be desirable to try C<wget> first.

Protocols supported: C<http>, C<https>

=head1 PROPERTIES

=head2 wget_command

The full path to the C<wget> command.  The default is usually correct.

=head2 ssl

Ignored by this plugin.  Provided for compatibility with some other fetch plugins.

=head1 SEE ALSO

=over 4

=item L<alienfile>

=item L<Alien::Build>

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
