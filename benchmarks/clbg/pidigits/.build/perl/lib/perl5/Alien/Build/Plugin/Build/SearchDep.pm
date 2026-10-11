package Alien::Build::Plugin::Build::SearchDep;

use strict;
use warnings;
use 5.008004;
use Alien::Build::Plugin;
use Text::ParseWords qw( shellwords );

# ABSTRACT: Add dependencies to library and header search path
our $VERSION = '2.88'; # VERSION


has aliens => {};


has public_I => 0;
has public_l => 0;

sub init
{
  my($self, $meta) = @_;

  $meta->add_requires('configure' => 'Alien::Build::Plugin::Build::SearchDep' => '0.35');
  $meta->add_requires('share'     => 'Env::ShellWords' => 0.01);

  if($self->public_I || $self->public_l)
  {
    $meta->add_requires('configure' => 'Alien::Build::Plugin::Build::SearchDep' => '0.53');
  }

  my @aliens;
  if(ref($self->aliens) eq 'HASH')
  {
    @aliens = keys %{ $self->aliens };
    $meta->add_requires('share' => $_ => $self->aliens->{$_}) for @aliens;
  }
  else
  {
    @aliens = ref $self->aliens ? @{ $self->aliens } : ($self->aliens);
    $meta->add_requires('share' => $_ => 0) for @aliens;
  }

  $meta->around_hook(
    build => sub {
      my($orig, $build) = @_;

      local $ENV{CFLAGS}   = $ENV{CFLAGS};
      local $ENV{CXXFLAGS} = $ENV{CXXFLAGS};
      local $ENV{LDFLAGS}  = $ENV{LDFLAGS};

      tie my @CFLAGS,   'Env::ShellWords', 'CFLAGS';
      tie my @CXXFLAGS, 'Env::ShellWords', 'CXXFLAGS';
      tie my @LDFLAGS,  'Env::ShellWords', 'LDFLAGS';

      my $cflags  = $build->install_prop->{plugin_build_searchdep_cflags}  = [];
      my $ldflags = $build->install_prop->{plugin_build_searchdep_ldflags} = [];
      my $libs    = $build->install_prop->{plugin_build_searchdep_libs}    = [];

      foreach my $other (@aliens)
      {
        my $other_cflags;
        my $other_libs;
        if($other->install_type('share'))
        {
          $other_cflags = $other->cflags_static;
          $other_libs   = $other->libs_static;
        }
        else
        {
          $other_cflags = $other->cflags;
          $other_libs   = $other->libs;
        }
        unshift @$cflags,  grep /^-I/, shellwords($other_cflags);
        unshift @$ldflags, grep /^-L/, shellwords($other_libs);
        unshift @$libs,    grep /^-l/, shellwords($other_libs);
      }

      unshift @CFLAGS, @$cflags;
      unshift @CXXFLAGS, @$cflags;
      unshift @LDFLAGS, @$ldflags;

      $orig->($build);

    },
  );

  $meta->after_hook(
    gather_share => sub {
      my($build) = @_;

      $build->runtime_prop->{libs}        = '' unless defined $build->runtime_prop->{libs};
      $build->runtime_prop->{libs_static} = '' unless defined $build->runtime_prop->{libs_static};

      if($self->public_l)
      {
        $build->runtime_prop->{$_} = join(' ', _space_escape(@{ $build->install_prop->{plugin_build_searchdep_libs} })) . ' ' . $build->runtime_prop->{$_}
          for qw( libs libs_static );
      }

      $build->runtime_prop->{$_} = join(' ', _space_escape(@{ $build->install_prop->{plugin_build_searchdep_ldflags} })) . ' ' . $build->runtime_prop->{$_}
        for qw( libs libs_static );

      if($self->public_I)
      {
        $build->runtime_prop->{cflags}        = '' unless defined $build->runtime_prop->{cflags};
        $build->runtime_prop->{cflags_static} = '' unless defined $build->runtime_prop->{cflags_static};
        $build->runtime_prop->{$_} = join(' ', _space_escape(@{ $build->install_prop->{plugin_build_searchdep_cflags} })) . ' ' . $build->runtime_prop->{$_}
          for qw( cflags cflags_static );
      }
    },
  );
}

sub _space_escape
{
  map {
    my $str = $_;
    $str =~ s{(\s)}{\\$1}g;
    $str;
  } @_;
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::Plugin::Build::SearchDep - Add dependencies to library and header search path

=head1 VERSION

version 2.88

=head1 SYNOPSIS

 use alienfile;
 plugin 'Build::SearchDep' => (
   aliens => [qw( Alien::Foo Alien::Bar )],
 );

=head1 DESCRIPTION

This plugin adds the other aliens as prerequisites, and adds their header and library
search path to C<CFLAGS> and C<LDFLAGS> environment variable, so that tools that use
them (like autoconf) can pick them up.

=head1 PROPERTIES

=head2 aliens

Either a list reference or hash reference of the other aliens.  If a hash reference
then the keys are the class names and the values are the versions of those classes.

=head2 public_I

Include the C<-I> flags when setting the runtime cflags property.

=head2 public_l

Include the C<-l> flags when setting the runtime libs property.

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
