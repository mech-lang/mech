package Alien::Build::Plugin::PkgConfig::MakeStatic;

use strict;
use warnings;
use 5.008004;
use Alien::Build::Plugin;
use Path::Tiny ();

# ABSTRACT: Convert .pc files into static
our $VERSION = '2.88'; # VERSION


has path => undef;

sub _convert
{
  my($self, $build, $path) = @_;

  die "unable to read $path" unless -r $path;
  die "unable to write $path" unless -w $path;

  $build->log("converting $path to static");

  my %h = map {
    my($key, $value) = /^(.*?):(.*?)$/;
    $value =~ s{^\s+}{};
    $value =~ s{\s+$}{};
    ($key => $value);
  } grep /^(?:Libs|Cflags)(?:\.private)?:/, $path->lines;

  $h{Cflags} = '' unless defined $h{Cflags};
  $h{Libs}   = '' unless defined $h{Libs};

  $h{Cflags} .= ' ' . $h{"Cflags.private"} if defined $h{"Cflags.private"};
  $h{Libs}   .= ' ' . $h{"Libs.private"} if defined $h{"Libs.private"};

  $h{"Cflags.private"} = '';
  $h{"Libs.private"}  = '';

  $path->edit_lines(sub {

    if(/^(.*?):/)
    {
      my $key = $1;
      if(defined $h{$key})
      {
        s/^(.*?):.*$/$1: $h{$key} /;
        delete $h{$key};
      }
    }

  });

  $path->append("$_: $h{$_}\n") foreach keys %h;
}

sub _recurse
{
  my($self, $build, $dir) = @_;

  foreach my $child ($dir->children)
  {
    if(-d $child)
    {
      $self->_recurse($build, $child);
    }
    elsif($child->basename =~ /\.pc$/)
    {
      $self->_convert($build, $child);
    }
  }
}

sub init
{
  my($self, $meta) = @_;

  $meta->add_requires('configure' => 'Alien::Build::Plugin::Build::SearchDep' => '0.35');

  $meta->before_hook(
    gather_share => sub {
      my($build) = @_;

      if($self->path)
      {
        $self->_convert($build, Path::Tiny->new($self->path)->absolute);
      }
      else
      {
        $self->_recurse($build, Path::Tiny->new(".")->absolute);
      }

    },
  );
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::Plugin::PkgConfig::MakeStatic - Convert .pc files into static

=head1 VERSION

version 2.88

=head1 SYNOPSIS

 use alienfile;
 
 plugin 'PkgConfig::MakeStatic' => (
   path => 'lib/pkgconfig/foo.pc',
 );

=head1 DESCRIPTION

Convert C<.pc> file to use static linkage by default.  This is an experimental
plugin, so use with caution.

=head1 PROPERTIES

=head2 path

The path to the C<.pc> file.  If not provided, all C<.pc> files in the stage
directory will be converted.

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
