package Alien::GMP;

use strict;
use warnings;
use base 'Alien::Base';

our $VERSION = '1.16'; # VERSION
# ABSTRACT: Alien package for the GNU Multiple Precision library.

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::GMP - Alien package for the GNU Multiple Precision library.

=head1 VERSION

version 1.16

=head1 SYNOPSIS

In your Build.PL:

 use Module::Build;
 use Alien::GMP;
 my $builder = Module::Build->new(
   ...
   configure_requires => {
     'Alien::GMP' => '0',
     ...
   },
   extra_compiler_flags => Alien::GMP->cflags,
   extra_linker_flags   => Alien::GMP->libs,
   ...
 );
 
 $build->create_build_script;

In your Makefile.PL:

 use ExtUtils::MakeMaker;
 use Config;
 use Alien::GMP;
 
 WriteMakefile(
   ...
   CONFIGURE_REQUIRES => {
     'Alien::GMP' => '0',
   },
   CCFLAGS => Alien::GMP->cflags . " $Config{ccflags}",
   LIBS    => [ Alien::GMP->libs ],
   ...
 );

=head1 DESCRIPTION

This distribution installs GMP so that it can be used by other Perl distributions.  If already
installed for your operating system, and it can be found, this distribution will use the GMP
that comes with your operating system, otherwise it will download it from the Internet, build and
install it for you.

=head2 C++ support

This Alien will provide the C++ bindings for GMP (libgmpxx) if possible.  If you do not have a
C++ compiler, or if your operating system vendor provides a GMP package without the needed C++
files then it will not be available.  To use the C++ bindings, you can use the C<alt> method
to create a C++ instance of this GMP Alien.  For example:

 use ExtUtils::MakeMaker;
 use Config;
 use Alien::GMP;
 
 WriteMakefile(
   ...
   CONFIGURE_REQUIRES => {
     'Alien::GMP' => '1.06', # require version that provides C++ bindings
   },
   CCFLAGS => Alien::GMP->alt('gmpxx')->cflags . " $Config{ccflags}",
   LIBS    => [ Alien::GMP->alt('gmpxx')->libs ],
   ...
 );

=head1 Inline support

This module supports L<Inline's with functionality|Inline/"Playing 'with' Others">.

=head1 SEE ALSO

L<GMP|https://gmplib.org/>

=head1 AUTHOR

Original author: Richard Simões (RSIMOES)

Current maintainer: Graham Ollis E<lt>plicease@cpan.orgE<gt>

Contributors:

Zakariyya Mughal (ZMUGHAL)

William N. Braswell, Jr. (WBRASWELL)

=head1 COPYRIGHT AND LICENSE

This software is Copyright (c) 2012-2018 by Richard Simões.

This is free software, licensed under:

  The GNU Lesser General Public License, Version 3, June 2007

=cut
