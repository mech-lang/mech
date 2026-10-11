package Alien::Build::Plugin::Fetch::CurlCommand;

use strict;
use warnings;
use 5.008004;
use Alien::Build::Plugin;
use File::Which qw( which );
use Path::Tiny qw( path );
use Capture::Tiny qw( capture );
use File::Temp qw( tempdir );
use List::Util 1.33 qw( any pairmap );
use File::chdir;

# ABSTRACT: Plugin for fetching files using curl
our $VERSION = '2.88'; # VERSION


sub curl_command
{
  defined $ENV{CURL} ? scalar which($ENV{CURL}) : scalar which('curl');
}

has ssl => 0;
has _see_headers => 0;
has '+url' => '';

# when bootstrapping we have to specify this plugin as a prereq
# 1 is the default so that when this plugin is used directly
# you also get the prereq
has bootstrap_ssl => 1;


sub protocol_ok
{
  my($class, $protocol) = @_;
  my $curl = $class->curl_command;
  return 0 unless defined $curl;
  my($out, $err, $exit) = capture {
    system $curl, '--version';
  };

  {
    # make sure curl supports the -J option.
    # CentOS 6 for example is recent enough
    # that it does not.  gh#147, gh#148, gh#149
    local $CWD = tempdir( CLEANUP => 1 );
    my $file1 = path('foo/foo.txt');
    $file1->parent->mkpath;
    $file1->spew("hello world\n");
    my $url = 'file://' . $file1->absolute;
    my($out, $err, $exit) = capture {
      system $curl, '-O', '-J', $url;
    };
    my $file2 = $file1->parent->child($file1->basename);
    unlink "$file1";
    unlink "$file2";
    rmdir($file1->parent);
    return 0 if $exit;
  }

  foreach my $line (split /\n/, $out)
  {
    if($line =~ /^Protocols:\s*(.*)\s*$/)
    {
      my %proto = map { $_ => 1 } split /\s+/, $1;
      return $proto{$protocol} if $proto{$protocol};
    }
  }
  return 0;
}

sub init
{
  my($self, $meta) = @_;

  $meta->prop->{start_url} ||= $self->url;
  $self->url($meta->prop->{start_url});
  $self->url || Carp::croak('url is a required property');

  $meta->add_requires('configure', 'Alien::Build::Plugin::Fetch::CurlCommand' => '1.19')
    if $self->bootstrap_ssl;

  $meta->register_hook(
    fetch => sub {
      my($build, $url, %options) = @_;
      $url ||= $self->url;

      my($scheme) = $url =~ /^([a-z0-9]+):/i;

      if($scheme =~ /^https?$/)
      {
        local $CWD = tempdir( CLEANUP => 1 );

        my @writeout = (
          "ab-filename     :%{filename_effective}",
          "ab-content_type :%{content_type}",
          "ab-url          :%{url_effective}",
        );

        $build->log("writeout: $_\\n") for @writeout;
        path('writeout')->spew(join("\\n", @writeout));

        my @headers;
        if(my $headers = $options{http_headers})
        {
          if(ref $headers eq 'ARRAY')
          {
            @headers = pairmap { -H => "$a: $b" } @$headers;
          }
          else
          {
            $build->log("Fetch for $url with http_headers that is not an array reference");
          }
        }

        my @command = (
          $self->curl_command,
          '-L', '-f', '-O', '-J',
          -w => '@writeout',
          @headers,
        );

        push @command, -D => 'head' if $self->_see_headers;

        push @command, $url;

        my($stdout, $stderr) = $self->_execute($build, @command);

        my %h = map { /^ab-(.*?)\s*:(.*)$/ ? ($1 => $2) : () } split /\n/, $stdout;

        if(-e 'head')
        {
          $build->log(" ~ $_ => $h{$_}") for sort keys %h;
          $build->log(" header: $_") for path('headers')->lines;
        }

        my($type) = split /;/, $h{content_type};

        if($type eq 'text/html')
        {
          return {
            type     => 'html',
            base     => $h{url},
            content  => scalar path($h{filename})->slurp,
            protocol => $scheme,
          };
        }
        else
        {
          return {
            type     => 'file',
            filename => $h{filename},
            path     => path($h{filename})->absolute->stringify,
            protocol => $scheme,
          };
        }
      }
#      elsif($scheme eq 'ftp')
#      {
#        if($url =~ m{/$})
#        {
#          my($stdout, $stderr) = $self->_execute($build, $self->curl_command, -l => $url);
#          chomp $stdout;
#          return {
#            type => 'list',
#            list => [
#              map { { filename => $_, url => "$url$_" } } sort split /\n/, $stdout,
#            ],
#          };
#        }
#
#        my $first_error;
#
#        {
#          local $CWD = tempdir( CLEANUP => 1 );
#
#          my($filename) = $url =~ m{/([^/]+)$};
#          $filename = 'unknown' if (! defined $filename) || ($filename eq '');
#          my($stdout, $stderr) = eval { $self->_execute($build, $self->curl_command, -o => $filename, $url) };
#          $first_error = $@;
#          if($first_error eq '')
#          {
#            return {
#              type     => 'file',
#              filename => $filename,
#              path     => path($filename)->absolute->stringify,
#            };
#          }
#        }
#
#        {
#          my($stdout, $stderr) = eval { $self->_execute($build, $self->curl_command, -l => "$url/") };
#          if($@ eq '')
#          {
#            chomp $stdout;
#            return {
#              type => 'list',
#              list => [
#                map { { filename => $_, url => "$url/$_" } } sort split /\n/, $stdout,
#              ],
#            };
#          };
#        }
#
#        $first_error ||= 'unknown error';
#        die $first_error;
#
#      }
      else
      {
        die "scheme $scheme is not supported by the Fetch::CurlCommand plugin";
      }

    },
  ) if $self->curl_command;

  $self;
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
    $build->log($_) for split /\n/, $stderr;
    if($stderr =~ /Remote filename has no length/ && !!(any { /^-O$/ } @command))
    {
      my @new_command = map {
        /^-O$/ ? ( -o => 'index.html' ) : /^-J$/ ? () : ($_)
      } @command;
      return $self->_execute($build, @new_command);
    }
    die "error in curl fetch";
  }
  ($stdout, $stderr);
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::Plugin::Fetch::CurlCommand - Plugin for fetching files using curl

=head1 VERSION

version 2.88

=head1 SYNOPSIS

 use alienfile;
 
 share {
   start_url 'https://www.openssl.org/source/';
   plugin 'Fetch::CurlCommand';
 };

=head1 DESCRIPTION

This plugin provides a fetch based on the C<curl> command.  It works with other fetch
plugins (that is, the first one which succeeds will be used).  Most of the time the best plugin
to use will be L<Alien::Build::Plugin::Download::Negotiate>, but for some SSL bootstrapping
it may be desirable to try C<curl> first.

Protocols supported: C<http>, C<https>

C<https> support requires that curl was built with SSL support.

=head1 PROPERTIES

=head2 curl_command

The full path to the C<curl> command.  The default is usually correct.

=head2 ssl

Ignored by this plugin.  Provided for compatibility with some other fetch plugins.

=head1 METHODS

=head2 protocol_ok

 my $bool = $plugin->protocol_ok($protocol);
 my $bool = Alien::Build::Plugin::Fetch::CurlCommand->protocol_ok($protocol);

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
