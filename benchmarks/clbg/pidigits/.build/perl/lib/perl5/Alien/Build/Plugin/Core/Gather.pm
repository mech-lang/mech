package Alien::Build::Plugin::Core::Gather;

use strict;
use warnings;
use 5.008004;
use Alien::Build::Plugin;
use Env qw( @PATH @PKG_CONFIG_PATH );
use Path::Tiny ();
use File::chdir;
use Alien::Build::Util qw( _mirror _destdir_prefix );
use JSON::PP ();

# ABSTRACT: Core gather plugin
our $VERSION = '2.88'; # VERSION


sub init
{
  my($self, $meta) = @_;

  $meta->default_hook(
    $_ => sub {},
  ) for qw( gather_system gather_share );

  $meta->around_hook(
    gather_share => sub {
      my($orig, $build) = @_;

      local $ENV{PATH} = $ENV{PATH};
      local $ENV{PKG_CONFIG_PATH} = $ENV{PKG_CONFIG_PATH};
      unshift @PATH, Path::Tiny->new('bin')->absolute->stringify
        if -d 'bin';

      for my $dir (qw(share lib)) {
          unshift @PKG_CONFIG_PATH, Path::Tiny->new("$dir/pkgconfig")->absolute->stringify
            if -d "$dir/pkgconfig";
      }

      $orig->($build)
    }
  );

  foreach my $type (qw( share ffi ))
  {
    $meta->around_hook(
      "gather_$type" => sub {
        my($orig, $build) = @_;

        if($build->meta_prop->{destdir})
        {
          my $destdir = $ENV{DESTDIR};
          if(-d $destdir)
          {
            my $src = Path::Tiny->new(_destdir_prefix($ENV{DESTDIR}, $build->install_prop->{prefix}));
            my $dst = Path::Tiny->new($build->install_prop->{stage});

            my $res = do {
              local $CWD = "$src";
              $orig->($build);
            };

            $build->log("mirror $src => $dst");

            $dst->mkpath;
            # Please note: _mirror and Alien::Build::Util are ONLY
            # allowed to be used by core plugins.  If you are writing
            # a non-core plugin it may be removed.  That is why it
            # is private.
            _mirror("$src", "$dst", {
              verbose => 1,
              filter => $build->meta_prop->{$type eq 'share' ? 'destdir_filter' : 'destdir_ffi_filter'},
            });

            return $res;
          }
          else
          {
            die "nothing was installed into destdir" if $type eq 'share';
          }
        }
        else
        {
          local $CWD = $build->install_prop->{stage};
          my $ret = $orig->($build);

          # if we are not doing a double staged install we want to substitute the install
          # prefix with the runtime prefix.
          my $old = $build->install_prop->{prefix};
          my $new = $build->runtime_prop->{prefix};

          foreach my $flag (qw( cflags cflags_static libs libs_static ))
          {
            next unless defined $build->runtime_prop->{$flag};
            $build->runtime_prop->{$flag} =~ s{(-I|-L|-LIBPATH:)\Q$old\E}{$1 . $new}eg;
          }

          return $ret;
        }
      }
    );
  }

  $meta->after_hook(
    $_ => sub {
      my($build) = @_;

      die "stage is not defined.  be sure to call set_stage on your Alien::Build instance"
        unless $build->install_prop->{stage};

      my $stage = Path::Tiny->new($build->install_prop->{stage});
      $build->log("mkdir -p $stage/_alien");
      $stage->child('_alien')->mkpath;

      # drop a alien.json file for the runtime properties
      $stage->child('_alien/alien.json')->spew(
        JSON::PP->new->pretty->canonical(1)->ascii->encode($build->runtime_prop)
      );

      # copy the alienfile, if we managed to keep it around.
      if($build->meta->filename                 &&
         -r $build->meta->filename              &&
         $build->meta->filename !~ /\.(pm|pl)$/ &&
         ! -d $build->meta->filename)
      {
        Path::Tiny->new($build->meta->filename)
                  ->copy($stage->child('_alien/alienfile'));
      }

      if($build->install_prop->{patch} && -d $build->install_prop->{patch})
      {
        # Please note: _mirror and Alien::Build::Util are ONLY
        # allowed to be used by core plugins.  If you are writing
        # a non-core plugin it may be removed.  That is why it
        # is private.
        _mirror($build->install_prop->{patch},
                $stage->child('_alien/patch')->stringify);
      }

    },
  ) for qw( gather_share gather_system );
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::Plugin::Core::Gather - Core gather plugin

=head1 VERSION

version 2.88

=head1 SYNOPSIS

 use alienfile;
 # already loaded

=head1 DESCRIPTION

This plugin helps make the gather stage work.

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
