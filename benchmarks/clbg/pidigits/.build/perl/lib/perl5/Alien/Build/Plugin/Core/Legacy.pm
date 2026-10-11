package Alien::Build::Plugin::Core::Legacy;

use strict;
use warnings;
use 5.008004;
use Alien::Build::Plugin;

# ABSTRACT: Core Alien::Build plugin to maintain compatibility with legacy Alien::Base
our $VERSION = '2.88'; # VERSION


sub init
{
  my($self, $meta) = @_;

  $meta->after_hook(
    $_ => sub {
      my($build) = @_;

      $build->log("adding legacy hash to config");

      my $runtime = $build->runtime_prop;

      if($runtime->{cflags} && ! defined $runtime->{cflags_static})
      {
        $runtime->{cflags_static} = $runtime->{cflags};
      }

      if($runtime->{libs} && ! defined $runtime->{libs_static})
      {
        $runtime->{libs_static} = $runtime->{libs};
      }

      $runtime->{legacy}->{finished_installing} = 1;
      $runtime->{legacy}->{install_type}        = $runtime->{install_type};
      $runtime->{legacy}->{version}             = $runtime->{version};
      $runtime->{legacy}->{original_prefix}     = $runtime->{prefix};
    }
  ) for qw( gather_system gather_share );
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::Plugin::Core::Legacy - Core Alien::Build plugin to maintain compatibility with legacy Alien::Base

=head1 VERSION

version 2.88

=head1 SYNOPSIS

 use alienfile;
 # already loaded

=head1 DESCRIPTION

This plugin provides some compatibility with the legacy L<Alien::Base::ModuleBuild>
interfaces.

=head1 SEE ALSO

L<Alien::Build>, L<Alien::Base::ModuleBuild>

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
