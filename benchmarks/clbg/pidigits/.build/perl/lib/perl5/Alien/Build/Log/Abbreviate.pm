package Alien::Build::Log::Abbreviate;

use strict;
use warnings;
use 5.008004;
use Term::ANSIColor ();
use Path::Tiny qw( path );
use File::chdir;
use parent qw( Alien::Build::Log );

# ABSTRACT: Log class for Alien::Build which is less verbose
our $VERSION = '2.88'; # VERSION


sub _colored
{
  my($code, @out) = @_;
  -t STDOUT ? Term::ANSIColor::colored($code, @out) : @out;
}

my $root = path("$CWD");

sub log
{
  my(undef, %args) = @_;
  my($message) = $args{message};
  my ($package, $filename, $line) = @{ $args{caller} };

  my $source = $package;
  $source =~ s/^Alien::Build::Auto::[^:]+::Alienfile/alienfile/;

  my $expected = $package;
  $expected .= '.pm' unless $package eq 'alienfile';
  $expected =~ s/::/\//g;
  if($filename !~ /\Q$expected\E$/)
  {
    $source = path($filename)->relative($root);
  }
  else
  {
    $source =~ s/^Alien::Build::Plugin/ABP/;
    $source =~ s/^Alien::Build/AB/;
  }

  print _colored([ "bold on_black"          ], '[');
  print _colored([ "bright_green on_black"  ], $source);
  print _colored([ "on_black"               ], ' ');
  print _colored([ "bright_yellow on_black" ], $line);
  print _colored([ "bold on_black"          ], ']');
  print _colored([ "white on_black"         ], ' ', $message);
  print "\n";
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::Log::Abbreviate - Log class for Alien::Build which is less verbose

=head1 VERSION

version 2.88

=head1 SYNOPSIS

=head1 DESCRIPTION

=head1 METHODS

=head2 log

 $log->log(%opts);

Send single log line to stdout.

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
