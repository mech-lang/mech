package Alien::Build::Plugin::Digest::SHAPP;

use strict;
use warnings;
use Alien::Build::Plugin;

# ABSTRACT: Plugin to check SHA digest with Digest::SHA::PurePerl
our $VERSION = '2.88'; # VERSION


sub init
{
  my($self, $meta) = @_;

  $meta->add_requires('configure' => 'Alien::Build'          => "2.57" );
  $meta->add_requires('share'     => 'Digest::SHA::PurePerl' =>    "0" );

  $meta->register_hook( check_digest => sub {
    my($build, $file, $algo, $expected_digest) = @_;

    return 0 unless $algo =~ /^SHA[0-9]+$/;

    my $sha = Digest::SHA::PurePerl->new($algo);
    return 0 unless defined $sha;

    if(defined $file->{content})
    {
      $sha->add($file->{content});
    }
    elsif(defined $file->{path})
    {
      $sha->addfile($file->{path}, "b");
    }
    else
    {
      die "unknown file type";
    }

    my $actual_digest = $sha->hexdigest;

    return 1 if $expected_digest eq $actual_digest;
    die "@{[ $file->{filename} ]} SHA@{[ $sha->algorithm ]} digest does not match: got $actual_digest, expected $expected_digest";

  });
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::Plugin::Digest::SHAPP - Plugin to check SHA digest with Digest::SHA::PurePerl

=head1 VERSION

version 2.88

=head1 SYNOPSIS

 use alienfile;
 plugin 'Digest::SHAPP';

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
