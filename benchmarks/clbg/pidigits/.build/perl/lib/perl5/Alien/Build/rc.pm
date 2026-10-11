package Alien::Build::rc;

use strict;
use warnings;
use 5.008004;

# ABSTRACT: Alien::Build local config
our $VERSION = '2.88'; # VERSION


sub logx ($)
{
  unshift @_, 'Alien::Build';
  goto &Alien::Build::log;
}


sub preload_plugin
{
  my(@args) = @_;
  push @Alien::Build::rc::PRELOAD, sub {
    shift->apply_plugin(@args);
  };
}


sub postload_plugin
{
  my(@args) = @_;
  push @Alien::Build::rc::POSTLOAD, sub {
    shift->apply_plugin(@args);
  };
}


sub preload ($)
{
  push @Alien::Build::rc::PRELOAD, $_[0];
}


sub postload ($)
{
  push @Alien::Build::rc::POSTLOAD, $_[0];
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::rc - Alien::Build local config

=head1 VERSION

version 2.88

=head1 SYNOPSIS

in your C<~/.alienbuild/rc.pl>:

 preload 'Foo::Bar';
 postload 'Baz::Frooble';

=head1 DESCRIPTION

L<Alien::Build> will load your C<~/.alienbuild/rc.pl> file, if it exists
before running the L<alienfile> recipe.  This allows you to alter the
behavior of L<Alien::Build> based L<Alien>s if you have local configuration
requirements.  For example you can prompt before downloading remote content
or fetch from a local mirror.

=head1 FUNCTIONS

=head2 logx

 log $message;

Send a message to the L<Alien::Build> log.

=head2 preload_plugin

 preload_plugin $plugin, @args;

Preload the given plugin, with arguments.

=head2 postload_plugin

 postload_plugin $plugin, @args;

Postload the given plugin, with arguments.

=head2 preload

[deprecated]

 preload $plugin;

Preload the given plugin.

=head2 postload

[deprecated]

 postload $plugin;

Postload the given plugin.

=head1 SEE ALSO

=over 4

=item L<Alien::Build::Plugin::Fetch::Cache>

=item L<Alien::Build::Plugin::Fetch::Prompt>

=item L<Alien::Build::Plugin::Fetch::Rewrite>

=item L<Alien::Build::Plugin::Probe::Override>

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
