package Alien::Build::Plugin::Digest::Negotiate;

use strict;
use warnings;
use Alien::Build::Plugin;

# ABSTRACT: Plugin negotiator for cryptographic signatures
our $VERSION = '2.88'; # VERSION


has '+sig' => sub { {} };

has check_fetch    => 1;
has check_download => 1;
has allow_listing  => 1;

sub init
{
  my($self, $meta) = @_;
  $meta->add_requires('configure' => 'Alien::Build::Plugin::Digest::Negotiate' => "0" );

  $meta->prop->{check_digest} = 1;

  my $sigs = $meta->prop->{digest} ||= {};

  if(ref($self->sig) eq 'HASH') {

    foreach my $filename (keys %{ $self->sig })
    {
      my $signature = $self->sig->{$filename};
      my($algo) = @$signature;
      die "Unknown digest algorithm $algo" unless $algo =~ /^SHA(1|224|256|384|512|512224|512256)$/; # reportedly what is supported by Digest::SHA
      $sigs->{$filename} = $signature;
    }

  } elsif(ref($self->sig) eq 'ARRAY') {

    my $signature = $self->sig;
    my($algo) = @$signature;
    die "Unknown digest algorithm $algo" unless $algo =~ /^SHA(1|224|256|384|512|512224|512256)$/; # reportedly what is supported by Digest::SHA
    $sigs->{'*'} = $signature;
  }

  # In the future if this negotiator supports algorithms other
  # than SHA, we should probably ajust this to keep track of
  # which ones we actually need when we are looping through them
  # above.  Also technically you could call this plugin without
  # any sigs, and we shouldn't in theory need to apply Digest::SHA,
  # but stuff won't work that way so that is a corner case we
  # are not going to worry about.
  $meta->apply_plugin('Digest::SHA');

  $meta->around_hook(
    fetch => sub {
      my($orig, $build, @rest) = @_;
      my $res = $orig->($build, @rest);
      if($res->{type} eq 'file')
      {
        $build->check_digest($res);
      }
      else
      {
        die "listing fetch not allowed" unless $self->allow_listing;
      }
      $res;
    },
  ) if $self->check_fetch;

  # Note that check_download hook is currently undocumented and
  # may change in the future.
  $meta->register_hook(
    check_download => sub {
      my($build) = @_;
      my $path = $build->install_prop->{download};
      die "Checking cryptographic signatures on download only works for single archive" unless defined $path;
      $build->check_digest($path);
    },
  ) if $self->check_download;
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::Plugin::Digest::Negotiate - Plugin negotiator for cryptographic signatures

=head1 VERSION

version 2.88

=head1 SYNOPSIS

for a single file:

 use alienfile;
 plugin 'Digest' => [ SHA256 => $digest ];

or for multiple files:

 use alienfile;
 plugin 'Digest' => {
   file1 => [ SHA256 => $digest1 ],
   file2 => [ SHA256 => $digest2 ],
 };

=head1 DESCRIPTION

This plugin is experimental.

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
