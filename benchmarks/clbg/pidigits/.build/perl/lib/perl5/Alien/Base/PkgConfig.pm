package Alien::Base::PkgConfig;

use strict;
use warnings;
use 5.008004;
use Carp;
use Config;
use Path::Tiny qw( path );
use Capture::Tiny qw( capture_stderr );

# ABSTRACT: Private legacy pkg-config class for Alien::Base
our $VERSION = '2.88'; # VERSION


sub new {
  my $class   = shift;

  # allow creation of an object from a full spec.
  if (ref $_[0] eq 'HASH') {
    return bless $_[0], $class;
  }

  my ($path) = @_;
  croak "Must specify a file" unless defined $path;

  $path = path( $path )->absolute;

  my($name) = $path->basename =~ /^(.*)\.pc$/;

  my $self = {
    package  => $name,
    vars     => { pcfiledir => $path->parent->stringify },
    keywords => {},
  };

  bless $self, $class;

  $self->read($path);

  return $self;
}

sub read {
  my $self = shift;
  my ($path) = @_;

  open my $fh, '<', $path
    or croak "Cannot open .pc file $path: $!";

  while (my $line = <$fh>) {
    if ($line =~ /^([^=:]+?)=([^\n\r]*)/) {
      $self->{vars}{$1} = $2;
    } elsif ($line =~ /^([^=:]+?):\s*([^\n\r]*)/) {
      $self->{keywords}{$1} = $2;
    }
  }
}

# getter/setter for vars
sub var {
  my $self = shift;
  my ($var, $newval) = @_;
  if (defined $newval) {
    $self->{vars}{$var} = $newval;
  }
  return $self->{vars}{$var};
}

# abstract keywords and other vars in terms of "pure" vars
sub make_abstract {
  my $self = shift;
  die "make_abstract needs a key (and possibly a value)" unless @_;
  my ($var, $value) = @_;

  $value = defined $value ? $value : $self->{vars}{$var};

  # convert other vars
  foreach my $key (keys %{ $self->{vars} }) {
    next if $key eq $var; # don't overwrite the current var
    $self->{vars}{$key} =~ s/\Q$value\E/\$\{$var\}/g;
  }

  # convert keywords
  foreach my $key (keys %{ $self->{keywords} }) {
    $self->{keywords}{$key} =~ s/\Q$value\E/\$\{$var\}/g;
  }

}

sub _interpolate_vars {
  my $self = shift;
  my ($string, $override) = @_;

  $override ||= {};

  foreach my $key (keys %$override) {
    carp "Overriden pkg-config variable $key, contains no data"
      unless $override->{$key};
  }

  if (defined $string) {
    1 while $string =~ s/\$\{(.*?)\}/$override->{$1} || $self->{vars}{$1}/e;
  }
  return $string;
}

sub keyword {
  my $self = shift;
  my ($keyword, $override) = @_;

  {
    no warnings 'uninitialized';
    croak "overrides passed to 'keyword' must be a hashref"
      if defined $override and ref $override ne 'HASH';
  }

  return $self->_interpolate_vars( $self->{keywords}{$keyword}, $override );
}

my $pkg_config_command;

sub pkg_config_command {
  unless (defined $pkg_config_command) {
    capture_stderr {

      # For now we prefer PkgConfig.pm over pkg-config on
      # Solaris 64 bit Perls.  We may need to do this on
      # other platforms, in which case this logic should
      # be abstracted so that it can be shared here and
      # in Build.PL

      if (`pkg-config --version` && $? == 0 && !($^O eq 'solaris' && $Config{ptrsize} == 8)) {
        $pkg_config_command = 'pkg-config';
      } else {
        require PkgConfig;
        $pkg_config_command = "$^X $INC{'PkgConfig.pm'}";
      }
    }
  }

  $pkg_config_command;
}

sub TO_JSON
{
  my($self) = @_;
  my %hash = %$self;
  $hash{'__CLASS__'} = ref($self);
  \%hash;
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Base::PkgConfig - Private legacy pkg-config class for Alien::Base

=head1 VERSION

version 2.88

=head1 DESCRIPTION

This class is used internally by L<Alien::Base> and L<Alien::Base::ModuleBuild>
to store information from pkg-config about installed Aliens.  It is not used
internally by the newer L<alienfile> and L<Alien::Build>.  It should never
be used externally, should not be used for code new inside of C<Alien-Build>.

=head1 SEE ALSO

=over

=item L<Alien::Base>

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
