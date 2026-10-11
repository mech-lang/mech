package Alien::Build::Log::Default;

use strict;
use warnings;
use 5.008004;
use parent qw( Alien::Build::Log );

# ABSTRACT: Default Alien::Build log class
our $VERSION = '2.88'; # VERSION


sub log
{
  my(undef, %args) = @_;
  my($message) = $args{message};
  my ($package, $filename, $line) = @{ $args{caller} };
  print "$package> $message\n";
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::Log::Default - Default Alien::Build log class

=head1 VERSION

version 2.88

=head1 SYNOPSIS

 Alien::Build->log("message1");
 $build->log("message2");

=head1 DESCRIPTION

This is the default log class for L<Alien::Build>.  It does
the sensible thing of sending the message to stdout, along
with the class that made the log call.  For more details
about logging with L<Alien::Build>, see L<Alien::Build::Log>.

=head1 METHODS

=head2 log

 $log->log(%opts);

Send single log line to stdout.

=head1 SEE ALSO

=over 4

=item L<Alien::Build>

=item L<Alien::Build::Log>

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
