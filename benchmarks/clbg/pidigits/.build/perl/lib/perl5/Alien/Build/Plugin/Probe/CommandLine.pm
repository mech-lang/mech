package Alien::Build::Plugin::Probe::CommandLine;

use strict;
use warnings;
use 5.008004;
use Alien::Build::Plugin;
use Carp ();
use Capture::Tiny qw( capture );
use File::Which ();
use Alien::Util qw( version_cmp );

# ABSTRACT: Probe for tools or commands already available
our $VERSION = '2.88'; # VERSION


has '+command' => sub { Carp::croak "@{[ __PACKAGE__ ]} requires command property" };


has 'args'       => [];


has 'secondary' => 0;


has 'match'     => undef;


has 'match_stderr' => undef;


has 'version'   => undef;


has 'version_stderr' => undef;


has 'atleast_version' => undef;


sub init
{
  my($self, $meta) = @_;

  my $check = sub {
    my($build) = @_;

    unless(File::Which::which($self->command))
    {
      die 'Command not found ' . $self->command;
    }

    if(defined $self->match || defined $self->match_stderr || defined $self->version || defined $self->version_stderr)
    {
      my($out,$err,$ret) = capture {
        system( $self->command, @{ $self->args } );
      };
      die 'Command did not return a true value' if $ret;
      die 'Command output did not match' if defined $self->match && $out !~ $self->match;
      die 'Command standard error did not match' if defined $self->match_stderr && $err !~ $self->match_stderr;
      if (defined $self->version or defined $self->version_stderr)
      {
        my $found_version = '0';
        if(defined $self->version)
        {
          if($out =~ $self->version)
          {
            $found_version = $1;
            $build->runtime_prop->{version} = $found_version;
          }
        }
        if(defined $self->version_stderr)
        {
          if($err =~ $self->version_stderr)
          {
            $found_version = $1;
            $build->hook_prop->{version} = $found_version;
            $build->runtime_prop->{version} = $found_version;
          }
        }
        if (my $atleast_version = $self->atleast_version)
        {
          if(version_cmp ($found_version, $self->atleast_version) < 0)
          {
            #  reset the versions
            $build->runtime_prop->{version} = undef;
            $build->hook_prop->{version} = undef;
            die "CommandLine probe found version $found_version, but at least $atleast_version is required.";
          }
        }
      }
    }

    $build->runtime_prop->{command} = $self->command;
    'system';
  };

  if($self->secondary)
  {
    $meta->around_hook(
      probe => sub {
        my $orig = shift;
        my $build = shift;
        my $type = $orig->($build, @_);
        return $type unless $type eq 'system';
        $check->($build);
      },
    );
  }
  else
  {
    $meta->register_hook(
      probe => $check,
    );
  }
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::Plugin::Probe::CommandLine - Probe for tools or commands already available

=head1 VERSION

version 2.88

=head1 SYNOPSIS

 use alienfile;
 plugin 'Probe::CommandLine' => (
   command => 'gzip',
   args    => [ '--version' ],
   match   => qr/gzip/,
   version => qr/gzip ([0-9\.]+)/,
 );

=head1 DESCRIPTION

This plugin probes for the existence of the given command line program.

=head1 PROPERTIES

=head2 command

The name of the command.

=head2 args

The arguments to pass to the command.

=head2 secondary

If you are using another probe plugin (such as L<Alien::Build::Plugin::Probe::CBuilder> or
L<Alien::Build::Plugin::PkgConfig::Negotiate>) to detect the existence of a library, but
also need a program to exist, then you should set secondary to a true value.  For example
when you need both:

 use alienfile;
 # requires both liblzma library and xz program
 plugin 'PkgConfig' => 'liblzma';
 plugin 'Probe::CommandLine' => (
   command   => 'xz',
   secondary => 1,
 );

When you don't:

 use alienfile;
 plugin 'Probe::CommandLine' => (
   command   => 'gzip',
   secondary => 0, # default
 );

=head2 match

Regular expression for which the program output should match.

=head2 match_stderr

Regular expression for which the program standard error should match.

=head2 version

Regular expression to parse out the version from the program output.
The regular expression should store the version number in C<$1>.

=head2 version_stderr

Regular expression to parse out the version from the program standard error.
The regular expression should store the version number in C<$1>.

=head2 atleast_version

The minimum required version as provided by the system.

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
