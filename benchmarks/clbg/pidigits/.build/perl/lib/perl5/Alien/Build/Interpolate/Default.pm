package Alien::Build::Interpolate::Default;

use strict;
use warnings;
use 5.008004;
use parent qw( Alien::Build::Interpolate );
use File::chdir;
use File::Which qw( which );
use Capture::Tiny qw( capture );

# ABSTRACT: Default interpolator for Alien::Build
our $VERSION = '2.88'; # VERSION

sub _config
{
  $Config::Config{$_[0]};
}


sub new
{
  my($class) = @_;
  my $self = $class->SUPER::new(@_);


  $self->add_helper( ar => sub { _config 'ar' }, 'Config' );


  $self->add_helper( bison => undef, sub {
    my $helper = shift;
    if(which 'bison')
    {
      $helper->code(sub { 'bison' });
      return  ();
    }
    else
    {
      return 'Alien::bison' => '0.17';
    }
  });


  $self->add_helper( bzip2 => undef, sub {
    my $helper = shift;
    if(which 'bzip2')
    {
      $helper->code( sub { 'bzip2' });
      return ();
    }
    else
    {
      return 'Alien::Libbz2' => '0.04';
    }
  });


  $self->add_helper( cc => sub { _config 'cc' }, 'Config' );


  $self->add_helper( cmake => sub { 'cmake' }, sub {
    if(which 'cmake')
    {
      return ();
    }
    else
    {
      return 'Alien::CMake' => '0.07';
    }
  });


  $self->add_helper( cp => sub { _config 'cp' }, 'Config' );


  $self->add_helper( devnull => sub { $^O eq 'MSWin32' ? 'NUL' : '/dev/null' });


  $self->add_helper( flex => undef, sub {
    my $helper = shift;
    if(which 'flex')
    {
      $helper->code(sub { 'flex' });
      return  ();
    }
    else
    {
      return 'Alien::flex' => '0.08';
    }
  });


  $self->add_helper( gmake => undef, 'Alien::gmake' => '0.11' );


  $self->add_helper( install => sub { 'install' });


  $self->add_helper( ld => sub { _config 'ld' }, 'Config' );


  $self->add_helper( m4 => undef, 'Alien::m4' => '0.08' );


  if($^O eq 'MSWin32')
  {
    # TL;DR: dmake is bad, and shouldn't be used to build anything but older
    # versions of Windows Perl that don't support gmake.
    my $perl_make = _config 'make';
    my $my_make;
    $self->add_helper( make => sub {
      return $my_make if defined $my_make;

      if( $perl_make ne 'dmake' && which $perl_make )
      {
        # assume if it is called nmake or gmake that it really is what it
        # says it is.
        if( $perl_make eq 'nmake' || $perl_make eq 'gmake' )
        {
          return $my_make = $perl_make;
        }

        my $out = capture { system $perl_make, '--version' };
        if( $out =~ /GNU make/i || $out =~ /Microsoft \(R\) Program Maintenance/ )
        {
          return $my_make = $perl_make;
        }

      }

      # if we see something that looks like it might be gmake, use that.
      foreach my $try (qw( gmake mingw32-make ))
      {
        return $my_make = $try if which $try;
      }

      if( which 'make' )
      {
        my $out = capture { system 'make', '--version' };
        if( $out =~ /GNU make/i || $out =~ /Microsoft \(R\) Program Maintenance/ )
        {
          return $my_make = 'make';
        }
      }

      # if we see something that looks like it might be nmake, use that.
      foreach my $try (qw( nmake ))
      {
        return $my_make = $try if which $try;
      }

      $my_make = $perl_make;
    });
  }
  else
  {
    $self->add_helper( make => sub { _config 'make' }, 'Config' );
  }


  $self->add_helper( mkdir_deep => sub { $^O eq 'MSWin32' ? 'md' : 'mkdir -p'}, 'Alien::Build' => '1.04' );
  $self->add_helper( make_path  => sub { $^O eq 'MSWin32' ? 'md' : 'mkdir -p'}, 'Alien::Build' => '1.05' );


  $self->add_helper( nasm => undef, sub {
    my $helper = shift;
    if(which 'nasm')
    {
      $helper->code(sub { 'nasm' });
      return  ();
    }
    else
    {
      return 'Alien::nasm' => '0.11';
    }
  });


  $self->add_helper( patch => undef, sub {
    my $helper = shift;
    if(which 'patch')
    {
      if($^O eq 'MSWin32')
      {
        $helper->code(sub { 'patch --binary' });
      }
      else
      {
        $helper->code(sub { 'patch' });
      }
      return  ();
    }
    else
    {
      return 'Alien::patch' => '0.09';
    }
  });


  $self->add_helper( perl => sub {
      my $perl = Devel::FindPerl::find_perl_interpreter();
      $perl =~ s{\\}{/}g if $^O eq 'MSWin32';
      $perl;
  }, 'Devel::FindPerl' );


  $self->add_helper( pkgconf => undef, 'Alien::pkgconf' => 0.06 );


  $self->add_helper( cwd => sub {
    my $cwd = "$CWD";
    $cwd =~ s{\\}{/}g if $^O eq 'MSWin32';
    $cwd;
  } );


  $self->add_helper( sh => sub { 'sh' }, 'Alien::MSYS' => '0.07' );


  $self->add_helper( rm => sub { _config 'rm' }, 'Config' );



  $self->add_helper( xz => undef, sub {
    my $helper = shift;
    if(which 'xz')
    {
      $helper->code(sub { 'xz' });
      return  ();
    }
    else
    {
      return 'Alien::xz' => '0.02';
    }
  });

  $self;
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::Interpolate::Default - Default interpolator for Alien::Build

=head1 VERSION

version 2.88

=head1 CONSTRUCTOR

=head2 new

 my $intr = Alien::Build::Interpolate::Default->new;

=head1 HELPERS

=head2 ar

 %{ar}

The ar command.

=head2 bison

 %{bison}

Requires: L<Alien::bison> 0.17 if not already in C<PATH>.

=head2 bzip2

 %{bzip2}

Requires: L<Alien::Libbz2> 0.04 if not already in C<PATH>.

=head2 cc

 %{cc}

The C Compiler used to build Perl

=head2 cmake

 %{cmake}

Requires: L<Alien::CMake> 0.07 if cmake is not already in C<PATH>.

Deprecated: Use the L<Alien::Build::Plugin::Build::CMake> plugin instead (which will replace
this helper with one that works with L<Alien::cmake3> that works better).

=head2 cp

 %{cp}

The copy command.

=head2 devnull

 %{devnull}

The null device, if available.  On Unix style operating systems this will be C</dev/null> on Windows it is C<NUL>.

=head2 flex

 %{flex}

Requires: L<Alien::flex> 0.08 if not already in C<PATH>.

=head2 gmake

 %{gmake}

Requires: L<Alien::gmake> 0.11

Deprecated: use L<Alien::Build::Plugin::Build::Make> instead.

=head2 install

 %{install}

The Unix C<install> command.  Not normally available on Windows.

=head2 ld

 %{ld}

The linker used to build Perl

=head2 m4

 %{m4}

Requires: L<Alien::m4> 0.08

L<Alien::m4> should pull in a version of C<m4> that will work with Autotools.

=head2 make

 %{make}

Make.  On Unix this will be the same make used by Perl.  On Windows this will be
C<gmake> or C<nmake> if those are available, and only C<dmake> if the first two
are not available.

=head2 make_path

 %{make_path}

Make directory, including all parent directories as needed.  This is usually C<mkdir -p>
on Unix and simply C<md> on windows.

=head2 nasm

 %{nasm}

Requires: L<Alien::nasm> 0.11 if not already in the C<PATH>.

=head2 patch

 %{patch}

Requires: L<Alien::patch> 0.09 if not already in the C<PATH>.

On Windows this will normally render C<patch --binary>, which makes patch work like it does on Unix.

=head2 perl

 %{perl}

Requires: L<Devel::FindPerl>

=head2 pkgconf

 %{pkgconf}

Requires: L<Alien::pkgconf> 0.06.

=head2 cwd

 %{cwd}

=head2 sh

 %{sh}

Unix style command interpreter (/bin/sh).

Deprecated: use the L<Alien::Build::Plugin::Build::MSYS> plugin instead.

=head2 rm

 %{rm}

The remove command

=head2 xz

 %{xz}

Requires: L<Alien::xz> 0.02 if not already in the C<PATH>.

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
