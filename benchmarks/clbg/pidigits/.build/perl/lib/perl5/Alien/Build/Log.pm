package Alien::Build::Log;

use strict;
use warnings;
use 5.008004;
use Carp ();

# ABSTRACT: Alien::Build logging
our $VERSION = '2.88'; # VERSION


my $log_class;
my $self;

sub new
{
  my($class) = @_;
  Carp::croak("Cannot instantiate base class") if $class eq 'Alien::Build::Log';
  return bless {}, $class;
}


sub default
{
  $self || do {
    my $class = $log_class || $ENV{ALIEN_BUILD_LOG} || 'Alien::Build::Log::Default';
    unless(eval { $class->can('new') })
    {
      my $pm = "$class.pm";
      $pm =~ s/::/\//g;
      require $pm;
    }
    $class->new;
  }
}


sub set_log_class
{
  my(undef, $class) = @_;
  return if defined $class && ($class eq ($log_class || ''));
  $log_class = $class;
  undef $self;
}


sub log
{
  Carp::croak("AB Log base class");
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::Log - Alien::Build logging

=head1 VERSION

version 2.88

=head1 SYNOPSIS

Create your custom log class:

 package Alien::Build::Log::MyLog;
 
 use parent qw( Alien::Build::Log );
 
 sub log
 {
   my(undef, %opt)  = @_;
   my($package, $filename, $line) = @{ $opt{caller} };
   my $message = $opt{message};
 
   ...;
 }

override log class:

 % env ALIEN_BUILD_LOG=Alien::Build::Log::MyLog cpanm Alien::libfoo

=head1 DESCRIPTION

=head1 CONSTRUCTORS

=head2 new

 my $log = Alien::Build::Log->new;

Create an instance of the log class.

=head2 default

 my $log = Alien::Build::Log->default;

Return singleton instance of log class used by L<Alien::Build>.

=head1 METHODS

=head2 set_log_class

 Alien::Build::Log->set_log_class($class);

Set the default log class used by L<Alien::Build>.  This method will also reset the
default instance used by L<Alien::Build>.  If not specified, L<Alien::Build::Log::Default>
will be used.

=head2 log

 $log->log(%options);

Overridable method which does the actual work of the log class.  Options:

=over 4

=item caller

Array references containing the package, file and line number of where the
log was called.

=item message

The message to log.

=back

=head1 ENVIRONMENT

=over 4

=item ALIEN_BUILD_LOG

The default log class used by L<Alien::Build>.

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
