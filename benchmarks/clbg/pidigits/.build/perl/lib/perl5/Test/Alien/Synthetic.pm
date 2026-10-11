package Test::Alien::Synthetic;

use strict;
use warnings;
use 5.008004;
use Test2::API qw( context );

# ABSTRACT: A mock alien object for testing
our $VERSION = '2.88'; # VERSION


sub _def ($) { my($val) = @_; defined $val ? $val : '' }

sub cflags       { _def shift->{cflags}             }
sub libs         { _def shift->{libs}               }
sub dynamic_libs { @{ shift->{dynamic_libs} || [] } }

sub runtime_prop
{
  my($self) = @_;
  defined $self->{runtime_prop}
    ? $self->{runtime_prop}
    : {};
}

sub cflags_static
{
  my($self) = @_;
  defined $self->{cflags_static}
    ? $self->{cflags_static}
    : $self->cflags;
}

sub libs_static
{
  my($self) = @_;
  defined $self->{libs_static}
    ? $self->{libs_static}
    : $self->libs;
}

sub bin_dir
{
  my $dir = shift->{bin_dir};
  defined $dir && -d $dir ? ($dir) : ();
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Test::Alien::Synthetic - A mock alien object for testing

=head1 VERSION

version 2.88

=head1 SYNOPSIS

 use Test2::V0;
 use Test::Alien;
 
 my $alien = synthetic {
   cflags => '-I/foo/bar/include',
   libs   => '-L/foo/bar/lib -lbaz',
 };
 
 alien_ok $alien;
 
 done_testing;

=head1 DESCRIPTION

This class is used to model a synthetic L<Alien>
class that implements the minimum L<Alien::Base>
interface needed by L<Test::Alien>.

It can be useful if you have a non-L<Alien::Base>
based L<Alien> distribution that you need to test.

B<NOTE>: The name of this class may move in the
future, so do not refer to this class name directly.
Instead create instances of this class using the
L<Test::Alien#synthetic> function.

=head1 ATTRIBUTES

=head2 cflags

String containing the compiler flags

=head2 cflags_static

String containing the static compiler flags

=head2 libs

String containing the linker and library flags

=head2 libs_static

String containing the static linker and library flags

=head2 dynamic_libs

List reference containing the dynamic libraries.

=head2 bin_dir

Tool binary directory.

=head2 runtime_prop

Runtime properties.

=head1 EXAMPLE

Here is a complete example using L<Alien::Libarchive> which is a non-L<Alien::Base>
based L<Alien> distribution.

 use strict;
 use warnings;
 use Test2::V0;
 use Test::Alien;
 use Alien::Libarchive;
 
 my $real = Alien::Libarchive->new;
 my $alien = synthetic {
   cflags       => scalar $real->cflags,
   libs         => scalar $real->libs,
   dynamic_libs => [$real->dlls],
 };
 
 alien_ok $alien;
 
 xs_ok do { local $/; <DATA> }, with_subtest {
   my($module) = @_;
   my $ptr = $module->archive_read_new;
   like $ptr, qr{^[0-9]+$};
   $module->archive_read_free($ptr);
 };
 
 ffi_ok { symbols => [qw( archive_read_new )] }, with_subtest {
   my($ffi) = @_;
   my $new  = $ffi->function(archive_read_new => [] => 'opaque');
   my $free = $ffi->function(archive_read_close => ['opaque'] => 'void');
   my $ptr = $new->();
   like $ptr, qr{^[0-9]+$};
   $free->($ptr);
 };
 
 done_testing;
 
 __DATA__
 
 #include "EXTERN.h"
 #include "perl.h"
 #include "XSUB.h"
 #include <archive.h>
 
 MODULE = TA_MODULE PACKAGE = TA_MODULE
 
 void *archive_read_new(class);
     const char *class;
   CODE:
     RETVAL = (void*) archive_read_new();
   OUTPUT:
     RETVAL
 
 void archive_read_free(class, ptr);
     const char *class;
     void *ptr;
   CODE:
     archive_read_free(ptr);

=head1 SEE ALSO

=over 4

=item L<Test::Alien>

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
