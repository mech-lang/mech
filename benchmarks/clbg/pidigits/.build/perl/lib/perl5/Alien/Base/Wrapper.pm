package Alien::Base::Wrapper;

use strict;
use warnings;
use 5.006;
use Config;
use Text::ParseWords qw( shellwords );

# NOTE: Although this module is now distributed with Alien-Build,
# it should have NO non-perl-core dependencies for all Perls
# 5.6.0-5.30.1 (as of this writing, and any Perl more recent).
# You should be able to extract this module from the rest of
# Alien-Build and use it by itself.  (There is a dzil plugin
# for this [AlienBase::Wrapper::Bundle]

# ABSTRACT: Compiler and linker wrapper for Alien
our $VERSION = '2.88'; # VERSION


sub _version_parts
{
  my($version) = @_;
  $version = '0' unless defined $version;

  # dotted-decimal style, e.g. 1.2.3 or v1.2.3: compare each part
  # separately, as integers.
  if($version =~ /^v?[0-9]+(\.[0-9]+){2,}$/)
  {
    $version =~ s/^v//;
    return [ split /\./, $version ];
  }

  # plain decimal style, e.g. 1.00 or 6.52: compare numerically as
  # a single number, so that (say) 6.6 is newer than 6.52.
  my($number) = $version =~ /^([0-9]+(?:\.[0-9]+)?)/;
  $number = 0 unless defined $number;
  return [ $number ];
}

sub _version_cmp
{
  my($v1, $v2) = @_;
  my $p1 = _version_parts($v1);
  my $p2 = _version_parts($v2);

  for my $i (0 .. (@$p1 > @$p2 ? $#$p1 : $#$p2))
  {
    my $n1 = defined $p1->[$i] ? $p1->[$i] : 0;
    my $n2 = defined $p2->[$i] ? $p2->[$i] : 0;
    my $cmp = $n1 <=> $n2;
    return $cmp if $cmp;
  }

  return 0;
}

# returns whichever of the two version numbers is newer (either may
# be undef, in which case the other, possibly also undef, is returned).
sub _version_max
{
  my($v1, $v2) = @_;
  return $v2 unless defined $v1;
  return $v1 unless defined $v2;
  return _version_cmp($v1, $v2) >= 0 ? $v1 : $v2;
}

sub _join
{
  join ' ',
    map {
      my $x = $_;
      $x =~ s/(\s)/\\$1/g;
      $x;
    } @_;
}

sub new
{
  my($class, @aliens) = @_;

  my $export = 1;
  my $writemakefile = 0;

  my @cflags_I;
  my @cflags_other;
  my @ldflags_L;
  my @ldflags_l;
  my @ldflags_other;
  my %requires = (
    'ExtUtils::MakeMaker'  => '6.52',
    'Alien::Base::Wrapper' => '1.97',
  );

  foreach my $alien (@aliens)
  {
    if($alien eq '!export')
    {
      $export = 0;
      next;
    }
    if($alien eq 'WriteMakefile')
    {
      $writemakefile = 1;
      next;
    }
    my $version = 0;
    if($alien =~ s/=(.*)$//)
    {
      $version = $1;
    }
    $alien = "Alien::$alien" unless $alien =~ /::/;
    $requires{$alien} = $version;
    my $alien_pm = $alien . '.pm';
    $alien_pm =~ s/::/\//g;
    require $alien_pm unless eval { $alien->can('cflags') } && eval { $alien->can('libs') };
    my $cflags;
    my $libs;
    if($alien->install_type eq 'share' && $alien->can('cflags_static'))
    {
      $cflags = $alien->cflags_static;
      $libs   = $alien->libs_static;
    }
    else
    {
      $cflags = $alien->cflags;
      $libs   = $alien->libs;
    }

    $cflags = '' unless defined $cflags;
    $libs = '' unless defined $libs;

    push @cflags_I,     grep  /^-I/, shellwords $cflags;
    push @cflags_other, grep !/^-I/, shellwords $cflags;

    push @ldflags_L,     grep  /^-L/,    shellwords $libs;
    push @ldflags_l,     grep  /^-l/,    shellwords $libs;
    push @ldflags_other, grep !/^-[Ll]/, shellwords $libs;
  }

  my @cflags_define = grep  /^-D/, @cflags_other;
  my @cflags_other2 = grep !/^-D/, @cflags_other;

  my @mm;

  push @mm, INC       => _join @cflags_I                             if @cflags_I;
  push @mm, CCFLAGS   => _join(@cflags_other2) . " $Config{ccflags}" if @cflags_other2;
  push @mm, DEFINE    => _join(@cflags_define)                       if @cflags_define;

  # TODO: handle spaces in -L paths
  push @mm, LIBS      => ["@ldflags_L @ldflags_l"];
  my @ldflags = (@ldflags_L, @ldflags_other);
  push @mm, LDDLFLAGS => _join(@ldflags) . " $Config{lddlflags}"     if @ldflags;
  push @mm, LDFLAGS   => _join(@ldflags) . " $Config{ldflags}"       if @ldflags;

  my @mb;

  push @mb, extra_compiler_flags => _join(@cflags_I, @cflags_other);
  push @mb, extra_linker_flags   => _join(@ldflags_l);

  if(@ldflags)
  {
    push @mb, config => {
      lddlflags => _join(@ldflags) . " $Config{lddlflags}",
      ldflags   => _join(@ldflags) . " $Config{ldflags}",
    },
  }

  bless {
    cflags_I       => \@cflags_I,
    cflags_other   => \@cflags_other,
    ldflags_L      => \@ldflags_L,
    ldflags_l      => \@ldflags_l,
    ldflags_other  => \@ldflags_other,
    mm             => \@mm,
    mb             => \@mb,
    _export        => $export,
    _writemakefile => $writemakefile,
    requires       => \%requires,
  }, $class;
}

my $default_abw = __PACKAGE__->new;

# for testing only
sub _reset { __PACKAGE__->new }


sub _myexec
{
  my @command = @_;
  if($^O eq 'MSWin32')
  {
    # To handle weird quoting on MSWin32
    # this logic needs to be improved.
    my $command = "@command";
    $command =~ s{"}{\\"}g;
    system $command;

    if($? == -1 )
    {
      die "failed to execute: $!\n";
    }
    elsif($? & 127)
    {
      die "child died with signal @{[ $? & 128 ]}";
    }
    else
    {
      exit($? >> 8);
    }
  }
  else
  {
    exec @command;
  }
}

sub cc
{
  my @command = (
    shellwords($Config{cc}),
    @{ $default_abw->{cflags_I} },
    @{ $default_abw->{cflags_other} },
    @ARGV,
  );
  print "@command\n" unless $ENV{ALIEN_BASE_WRAPPER_QUIET};
  _myexec @command;
}


sub ld
{
  my @command = (
    shellwords($Config{ld}),
    @{ $default_abw->{ldflags_L} },
    @{ $default_abw->{ldflags_other} },
    @ARGV,
    @{ $default_abw->{ldflags_l} },
  );
  print "@command\n" unless $ENV{ALIEN_BASE_WRAPPER_QUIET};
  _myexec @command;
}


sub mm_args
{
  my $self = ref $_[0] ? shift : $default_abw;
  @{ $self->{mm} };
}


sub mm_args2
{
  my $self = shift;
  $self = $default_abw unless ref $self;
  my %args = @_;

  my @mm = @{ $self->{mm} };

  while(@mm)
  {
    my $key = shift @mm;
    my $value = shift @mm;
    if(defined $args{$key})
    {
      if($args{$key} eq 'LIBS')
      {
        require Carp;
        # Todo: support this maybe?
        Carp::croak("please do not specify your own LIBS key with mm_args2");
      }
      else
      {
        $args{$key} = join ' ', $value, $args{$key};
      }
    }
    else
    {
      $args{$key} = $value;
    }
  }

  foreach my $module (keys %{ $self->{requires} })
  {
    $args{CONFIGURE_REQUIRES}->{$module} = _version_max(
      $args{CONFIGURE_REQUIRES}->{$module},
      $self->{requires}->{$module},
    );
  }

  %args;
}


sub checklib_args2
{
  my $self = shift;
  $self = $default_abw unless ref $self;
  my %args = @_;

  if(defined $args{LIBS} || defined $args{INC})
  {
    require Carp;
    Carp::croak("please do not specify your own LIBS or INC key with checklib_args2");
  }

  if(@{ $self->{cflags_I} })
  {
    my @old = defined $args{incpath} ? (ref $args{incpath} ? @{ $args{incpath} } : ($args{incpath})) : ();
    my @new = map { (my $x = $_) =~ s/^-I//; $x } @{ $self->{cflags_I} };
    $args{incpath} = [ @new, @old ];
  }

  if(@{ $self->{cflags_other} })
  {
    $args{ccflags} = join ' ', @{ $self->{cflags_other} }, (defined $args{ccflags} ? $args{ccflags} : ());
  }

  if(@{ $self->{ldflags_L} })
  {
    my @old = defined $args{libpath} ? (ref $args{libpath} ? @{ $args{libpath} } : ($args{libpath})) : ();
    my @new = map { (my $x = $_) =~ s/^-L//; $x } @{ $self->{ldflags_L} };
    $args{libpath} = [ @new, @old ];
  }

  if(@{ $self->{ldflags_l} } || @{ $self->{ldflags_other} })
  {
    $args{ldflags} = join ' ', @{ $self->{ldflags_l} }, @{ $self->{ldflags_other} }, (defined $args{ldflags} ? $args{ldflags} : ());
  }

  %args;
}


sub mb_args
{
  my $self = ref $_[0] ? shift : $default_abw;
  @{ $self->{mb} };
}

sub import
{
  shift;
  my $abw = $default_abw = __PACKAGE__->new(@_);
  if($abw->_export)
  {
    my $caller = caller;
    no strict 'refs';
    *{"${caller}::cc"} = \&cc;
    *{"${caller}::ld"} = \&ld;
  }
  if($abw->_writemakefile)
  {
    my $caller = caller;
    no strict 'refs';
    *{"${caller}::WriteMakefile"} = \&WriteMakefile;
  }
}


sub WriteMakefile
{
  my %args = @_;

  require ExtUtils::MakeMaker;
  ExtUtils::MakeMaker->VERSION('6.52');

  my @aliens;

  if(my $reqs = delete $args{alien_requires})
  {
    if(ref $reqs eq 'HASH')
    {
      @aliens = map {
        my $module  = $_;
        my $version = $reqs->{$module};
        $version ? "$module=$version" : "$module";
      } sort keys %$reqs;
    }
    elsif(ref $reqs eq 'ARRAY')
    {
      @aliens = @$reqs;
    }
    else
    {
      require Carp;
      Carp::croak("aliens_require must be either a hash or array reference");
    }
  }
  else
  {
    require Carp;
    Carp::croak("You are using Alien::Base::Wrapper::WriteMakefile, but didn't specify any alien requirements");
  }

  ExtUtils::MakeMaker::WriteMakefile(
    Alien::Base::Wrapper->new(@aliens)->mm_args2(%args),
  );
}

sub _export        { shift->{_export} }
sub _writemakefile { shift->{_writemakefile} }

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Base::Wrapper - Compiler and linker wrapper for Alien

=head1 VERSION

version 2.88

=head1 SYNOPSIS

From the command line:

 % perl -MAlien::Base::Wrapper=Alien::Foo,Alien::Bar -e cc -- -o foo.o -c foo.c
 % perl -MAlien::Base::Wrapper=Alien::Foo,Alien::Bar -e ld -- -o foo foo.o

From Makefile.PL (static):

 use ExtUtils::MakeMaker;
 use Alien::Base::Wrapper ();
 
 WriteMakefile(
   Alien::Base::Wrapper->new( 'Alien::Foo', 'Alien::Bar')->mm_args2(
     'NAME'              => 'Foo::XS',
     'VERSION_FROM'      => 'lib/Foo/XS.pm',
   ),
 );

From Makefile.PL (static with wrapper)

 use Alien::Base::Wrapper qw( WriteMakefile);
 
 WriteMakefile(
   'NAME'              => 'Foo::XS',
   'VERSION_FROM'      => 'lib/Foo/XS.pm',
   'alien_requires'    => {
     'Alien::Foo' => 0,
     'Alien::Bar' => 0,
   },
 );

From Makefile.PL (dynamic):

 use Devel::CheckLib qw( check_lib );
 use ExtUtils::MakeMaker 6.52;
 
 my @mm_args;
 my @libs;
 
 if(check_lib( lib => [ 'foo' ] )
 {
   push @mm_args, LIBS => [ '-lfoo' ];
 }
 else
 {
   push @mm_args,
     CC => '$(FULLPERL) -MAlien::Base::Wrapper=Alien::Foo -e cc --',
     LD => '$(FULLPERL) -MAlien::Base::Wrapper=Alien::Foo -e ld --',
     BUILD_REQUIRES => {
       'Alien::Foo'           => 0,
       'Alien::Base::Wrapper' => 0,
     }
   ;
 }
 
 WriteMakefile(
   'NAME'         => 'Foo::XS',
   'VERSION_FROM' => 'lib/Foo/XS.pm',
   'CONFIGURE_REQUIRES => {
     'ExtUtils::MakeMaker' => 6.52,
   },
   @mm_args,
 );

=head1 DESCRIPTION

This module acts as a wrapper around one or more L<Alien> modules.  It is designed to work
with L<Alien::Base> based aliens, but it should work with any L<Alien> which uses the same
essential API.

In the first example (from the command line), this class acts as a wrapper around the
compiler and linker that Perl is configured to use.  It takes the normal compiler and
linker flags and adds the flags provided by the Aliens specified, and then executes the
command.  It will print the command to the console so that you can see exactly what is
happening.

In the second example (from Makefile.PL non-dynamic), this class is used to generate the
appropriate L<ExtUtils::MakeMaker> (EUMM) arguments needed to C<WriteMakefile>.

In the third example (from Makefile.PL dynamic), we do a quick check to see if the simple
linker flag C<-lfoo> will work, if so we use that.  If not, we use a wrapper around the
compiler and linker that will use the alien flags that are known at build time.  The
problem that this form attempts to solve is that compiler and linker flags typically
need to be determined at I<configure> time, when a distribution is installed, meaning
if you are going to use an L<Alien> module then it needs to be a configure prerequisite,
even if the library is already installed and easily detected on the operating system.

The author of this module believes that the third (from Makefile.PL dynamic) form is
somewhat unnecessary.  L<Alien> modules based on L<Alien::Base> have a few prerequisites,
but they are well maintained and reliable, so while there is a small cost in terms of extra
dependencies, the overall reliability thanks to reduced overall complexity.

=head1 CONSTRUCTOR

=head2 new

 my $abw = Alien::Base::Wrapper->new(@aliens);

Instead of passing the aliens you want to use into this modules import you can create
a non-global instance of C<Alien::Base::Wrapper> using the OO interface.

=head1 FUNCTIONS

=head2 cc

 % perl -MAlien::Base::Wrapper=Alien::Foo -e cc -- cflags

Invoke the C compiler with the appropriate flags from C<Alien::Foo> and what
is provided on the command line.

=head2 ld

 % perl -MAlien::Base::Wrapper=Alien::Foo -e ld -- ldflags

Invoke the linker with the appropriate flags from C<Alien::Foo> and what
is provided on the command line.

=head2 mm_args

 my %args = $abw->mm_args;
 my %args = Alien::Base::Wrapper->mm_args;

Returns arguments that you can pass into C<WriteMakefile> to compile/link against
the specified Aliens.  Note that this does not set  C<CONFIGURE_REQUIRES>.  You
probably want to use C<mm_args2> below instead for that reason.

=head2 mm_args2

 my %args = $abw->mm_args2(%args);
 my %args = Alien::Base::Wrapper->mm_args2(%args);

Returns arguments that you can pass into C<WriteMakefile> to compile/link against.  It works
a little differently from C<mm_args> above in that you can pass in arguments.  It also adds
the appropriate C<CONFIGURE_REQUIRES> for you so you do not have to do that explicitly.

[version 2.87]

If you pass in your own C<CONFIGURE_REQUIRES> with a version for a module that this class
also requires (currently L<ExtUtils::MakeMaker> and C<Alien::Base::Wrapper> itself, plus any
Alien that you specify a minimum version for), then the newer of the two version numbers
will be used, so that neither requirement is weakened.  Version numbers may be given in
either decimal (C<1.23>) or dotted-decimal (C<1.2.3>) form.

=head2 checklib_args2

 my %args = $abw->checklib_args2(%args);
 my %args = Alien::Base::Wrapper->checklib_args2(%args);

Returns arguments that you can pass into C<assert_lib>, C<check_lib> or C<check_lib_or_exit> from
L<Devel::CheckLib> in order to compile/link against the Aliens specified.  It works a little like
C<mm_args2> above, except that instead of being applied to a hash of L<ExtUtils::MakeMaker>
arguments, it is applied to a hash of L<Devel::CheckLib> arguments.

 use Devel::CheckLib qw( check_lib_or_exit );
 use Alien::Base::Wrapper ();

 check_lib_or_exit(
   Alien::Base::Wrapper->new('Alien::Foo')->checklib_args2(
     lib    => 'foo',
     header => 'foo.h',
   ),
 );

The Alien's include paths are merged into C<incpath>, its other compiler flags (such as
C<-D> defines) are merged into C<ccflags>, its library paths are merged into C<libpath>, and
its libraries and other linker flags are merged into C<ldflags>, so that the C<lib> and
C<header> that you are checking for can be found using the environment provided by the Alien.

Since L<Devel::CheckLib> also allows you to specify C<INC> and C<LIBS> in the
L<ExtUtils::MakeMaker> style, and merging those styles with the Alien flags is ambiguous,
C<checklib_args2> will throw an exception if you attempt to specify either of those.  Use
C<incpath> / C<ccflags> and C<libpath> / C<ldflags> instead.

=head2 mb_args

 my %args = $abw->mb_args;
 my %args = Alien::Base::Wrapper->mb_args;

Returns arguments that you can pass into the constructor to L<Module::Build>.

=head2 WriteMakefile

 use Alien::Base::Wrapper qw( WriteMakefile );
 WriteMakefile(%args, alien_requires => \%aliens);
 WriteMakefile(%args, alien_requires => \@aliens);

This is a thin wrapper around C<WriteMakefile> from L<ExtUtils::MakeMaker>, which adds the
given aliens to the configure requirements and sets the appropriate compiler and linker
flags.

If the aliens are specified as a hash reference, then the keys are the module names and the
values are the versions.  For a list it is just the name of the aliens.

For the list form you can specify a version by appending C<=version> to the name of the
Aliens, that is:

 WriteMakefile(
   alien_requires => [ 'Alien::libfoo=1.23', 'Alien::libbar=4.56' ],
 );

The list form is recommended if the ordering of the aliens matter.  The aliens are sorted in
the hash form to make it consistent, but it may not be the order that you want.

=head1 ENVIRONMENT

Alien::Base::Wrapper responds to these environment variables:

=over 4

=item ALIEN_BASE_WRAPPER_QUIET

If set to true, do not print the command before executing

=back

=head1 SEE ALSO

L<Alien::Base>, L<Alien::Base>

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
