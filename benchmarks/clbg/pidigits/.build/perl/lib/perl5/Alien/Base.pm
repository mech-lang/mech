package Alien::Base;

use strict;
use warnings;
use 5.008004;
use Carp;
use Path::Tiny ();
use Scalar::Util qw/blessed/;
use Capture::Tiny 0.17 qw/capture_stdout/;
use Text::ParseWords qw/shellwords/;
use Alien::Util;

# ABSTRACT: Base classes for Alien:: modules
our $VERSION = '2.88'; # VERSION


sub import {
  my $class = shift;

  return if $class eq __PACKAGE__;

  return if $class->runtime_prop;

  return if $class->install_type('system');

  require DynaLoader;

  # Sanity check in order to ensure that dist_dir can be found.
  # This will throw an exception otherwise.
  $class->dist_dir;

  # get a reference to %Alien::MyLibrary::AlienLoaded
  # which contains names of already loaded libraries
  # this logic may be replaced by investigating the DynaLoader arrays
  my $loaded = do {
    no strict 'refs';
    no warnings 'once';
    \%{ $class . "::AlienLoaded" };
  };

  my @libs = $class->split_flags( $class->libs );

  my @L = grep { s/^-L// } map { "$_" } @libs;  ## no critic (ControlStructures::ProhibitMutatingListFunctions)
  my @l = grep { /^-l/ } @libs;

  unshift @DynaLoader::dl_library_path, @L;

  my @libpaths;
  foreach my $l (@l) {
    next if $loaded->{$l};

    my $path = DynaLoader::dl_findfile( $l );
    unless ($path) {
      carp "Could not resolve $l";
      next;
    }

    push @libpaths, $path;
    $loaded->{$l} = $path;
  }

  push @DynaLoader::dl_resolve_using, @libpaths;

  my @librefs = map { DynaLoader::dl_load_file( $_, 0x01 ) } grep !/\.(a|lib)$/, @libpaths;
  push @DynaLoader::dl_librefs, @librefs;

}


sub _dist_dir ($)
{
  my($dist_name) = @_;

  my @pm = split /-/, $dist_name;
  $pm[-1] .= ".pm";

  foreach my $inc (@INC)
  {
    my $pm = Path::Tiny->new($inc, @pm);
    if(-f $pm)
    {
      my $share = Path::Tiny->new($inc, qw( auto share dist ), $dist_name );
      if(-d $share)
      {
        return $share->absolute->stringify;
      }
      last;
    }
  }
  Carp::croak("unable to find dist share directory for $dist_name");
}

sub dist_dir {
  my $class = shift;

  my $dist = blessed $class || $class;
  $dist =~ s/::/-/g;

  my $dist_dir =
    $class->config('finished_installing')
      ? _dist_dir $dist
      : $class->config('working_directory');

  croak "Failed to find share dir for dist '$dist'"
    unless defined $dist_dir && -d $dist_dir;

  return $dist_dir;
}


sub new { return bless {}, $_[0] }

sub _flags
{
  my($class, $key) = @_;

  my $config = $class->runtime_prop;
  my $flags = $config->{$key};

  my $prefix = $config->{prefix};
  $prefix =~ s{\\}{/}g if $^O =~ /^(MSWin32|msys)$/;
  my $distdir = $config->{distdir};
  $distdir =~ s{\\}{/}g if $^O =~ /^(MSWin32|msys)$/;

  if(defined $flags && $prefix ne $distdir)
  {
    $flags = join ' ', map {
      my $flag = $_;
      $flag =~ s/^(-I|-L|-LIBPATH:)?\Q$prefix\E/$1$distdir/;
      $flag =~ s/(\s)/\\$1/g;
      $flag;
    } $class->split_flags($flags);
  }

  $flags;
}


sub cflags {
  my $class = shift;
  return $class->runtime_prop ? $class->_flags('cflags') : $class->_pkgconfig_keyword('Cflags');
}


sub cflags_static {
  my $class = shift;
  return $class->runtime_prop ? $class->_flags('cflags_static') : $class->_pkgconfig_keyword('Cflags', 'static');
}


sub libs {
  my $class = shift;
  return $class->runtime_prop ? $class->_flags('libs') : $class->_pkgconfig_keyword('Libs');
}


sub libs_static {
  my $class = shift;
  return $class->runtime_prop ? $class->_flags('libs_static') : $class->_pkgconfig_keyword('Libs', 'static');
}


sub version {
  my $self = shift;
  return $self->runtime_prop
    ? $self->runtime_prop->{version}
    : do {
      my $version = $self->config('version');
      chomp $version;
      $version;
    };
}


sub atleast_version {
  my $self = shift;
  my ($wantver) = @_;

  defined(my $version = $self->version) or
    croak "$self has no defined ->version";

  return $self->version_cmp($version, $wantver) >= 0;
}

sub exact_version {
  my $self = shift;
  my ($wantver) = @_;

  defined(my $version = $self->version) or
    croak "$self has no defined ->version";

  return $self->version_cmp($version, $wantver) == 0;
}

sub max_version {
  my $self = shift;
  my ($wantver) = @_;

  defined(my $version = $self->version) or
    croak "$self has no defined ->version";

  return $self->version_cmp($version, $wantver) <= 0;
}


sub version_cmp {
  shift;
  goto &Alien::Util::version_cmp;
}


sub install_type {
  my $self = shift;
  my $type = $self->config('install_type');
  return @_ ? $type eq $_[0] : $type;
}



sub is_system_install
{
  my($self) = @_;
  $self->install_type('system');
}


sub is_share_install
{
  my($self) = @_;
  $self->install_type('share');
}


sub _pkgconfig_keyword {
  my $self = shift;
  my $keyword = shift;
  my $static = shift;

  # use pkg-config if installed system-wide
  if ($self->install_type('system')) {
    my $name = $self->config('name');
    require Alien::Base::PkgConfig;
    my $command = Alien::Base::PkgConfig->pkg_config_command . " @{[ $static ? '--static' : '' ]} --\L$keyword\E $name";

    $! = 0;
    chomp ( my $pcdata = capture_stdout { system( $command ) } );

    # if pkg-config fails for whatever reason, then we try to
    # fallback on alien_provides_*
    $pcdata = '' if $! || $?;

    $pcdata =~ s/\s*$//;

    if($self->config('system_provides')) {
      if(my $system_provides = $self->config('system_provides')->{$keyword}) {
        $pcdata = length $pcdata ? "$pcdata $system_provides" : $system_provides;
      }
    }

    return $pcdata;
  }

  # use parsed info from build .pc file
  my $dist_dir = $self->dist_dir;
  my @pc = $self->_pkgconfig(@_);
  my @strings =
    grep defined,
    map { $_->keyword($keyword,
      #{ pcfiledir => $dist_dir }
    ) }
    @pc;

  if(defined $self->config('original_prefix') && $self->config('original_prefix') ne $self->dist_dir)
  {
    my $dist_dir = $self->dist_dir;
    $dist_dir =~ s{\\}{/}g if $^O eq 'MSWin32';
    my $old = quotemeta $self->config('original_prefix');
    @strings = map {
      my $flag = $_;
      $flag =~ s{^(-I|-L|-LIBPATH:)?($old)}{$1.$dist_dir}e;
      $flag =~ s/(\s)/\\$1/g;
      $flag;
    } map { $self->split_flags($_) } @strings;
  }

  return join( ' ', @strings );
}

sub _pkgconfig {
  my $self = shift;
  my %all = %{ $self->config('pkgconfig') };

  # merge in found pc files
  require File::Find;
  my $wanted = sub {
    return if ( -d or not /\.pc$/ );
    require Alien::Base::PkgConfig;
    my $pkg = Alien::Base::PkgConfig->new($_);
    $all{$pkg->{package}} = $pkg;
  };
  File::Find::find( $wanted, $self->dist_dir );

  croak "No Alien::Base::PkgConfig objects are stored!"
    unless keys %all;

  # Run through all pkgconfig objects and ensure that their modules are loaded:
  for my $pkg_obj (values %all) {
    my $perl_module_name = blessed $pkg_obj;
    my $pm = "$perl_module_name.pm";
    $pm =~ s/::/\//g;
    eval { require $pm };
  }

  return @all{@_} if @_;

  my $manual = delete $all{_manual};

  if (keys %all) {
    return values %all;
  } else {
    return $manual;
  }
}


# helper method to call Alien::MyLib::ConfigData->config(@_)
sub config {
  my $class = shift;
  $class = blessed $class || $class;

  if(my $ab_config = $class->runtime_prop)
  {
    my $key = shift;
    return $ab_config->{legacy}->{$key};
  }

  my $config = $class . '::ConfigData';
  my $pm = "$class/ConfigData.pm";
  $pm =~ s{::}{/}g;
  eval { require $pm };

  if($@)
  {
    warn "Cannot find either a share directory or a ConfigData module for $class.\n";
    my $pm = "$class.pm";
    $pm =~ s{::}{/}g;
    warn "($class loaded from $INC{$pm})\n" if $INC{$pm};
    warn "Please see https://metacpan.org/pod/distribution/Alien-Build/lib/Alien/Build/Manual/FAQ.pod#Cannot-find-either-a-share-directory-or-a-ConfigData-module\n";
    die $@;
  }

  return $config->config(@_);
}

# helper method to split flags based on the OS
sub split_flags {
  my ($class, $line) = @_;
  if( $^O eq 'MSWin32' ) {
    $class->split_flags_windows($line);
  } else {
    # $os eq 'Unix'
    $class->split_flags_unix($line);
  }
}

sub split_flags_unix {
  my ($class, $line) = @_;
  shellwords($line);
}

sub split_flags_windows {
  # NOTE a better approach would be to write a function that understands cmd.exe metacharacters.
  my ($class, $line) = @_;

  # Double the backslashes so that when they are unescaped by shellwords(),
  # they become a single backslash. This should be fine on Windows since
  # backslashes are not used to escape metacharacters in cmd.exe.
  $line =~ s,\\,\\\\,g;
  shellwords($line);
}


sub dynamic_libs {
  my ($class) = @_;

  require FFI::CheckLib;

  my @find_lib_flags;

  if($class->install_type('system')) {

    if(my $prop = $class->runtime_prop)
    {
      if($prop->{ffi_checklib}->{system})
      {
        push @find_lib_flags, @{ $prop->{ffi_checklib}->{system} };
      }
      return FFI::CheckLib::find_lib( lib => $prop->{ffi_name}, @find_lib_flags )
        if defined $prop->{ffi_name};
    }

    my $name = $class->config('ffi_name');
    unless(defined $name)
    {
      $name = $class->config('name');
      $name = '' unless defined $name;
      # strip leading lib from things like libarchive or libffi
      $name =~ s/^lib//;
      # strip trailing version numbers
      $name =~ s/-[0-9\.]+$//;
    }

    my @libpath;
    if(defined $class->libs)
    {
      foreach my $flag ($class->split_flags($class->libs))
      {
        if($flag =~ /^-L(.*)$/)
        {
          push @libpath, $1;
        }
      }
    }

    return FFI::CheckLib::find_lib(lib => $name, libpath => \@libpath, @find_lib_flags );

  } else {

    my $dir = $class->dist_dir;
    my $dynamic = Path::Tiny->new($class->dist_dir, 'dynamic');

    if(my $prop = $class->runtime_prop)
    {
      if($prop->{ffi_checklib}->{share})
      {
        push @find_lib_flags, @{ $prop->{ffi_checklib}->{share_flags} };
      }
    }

    if(-d $dynamic)
    {
      return FFI::CheckLib::find_lib(
        lib        => '*',
        libpath    => "$dynamic",
        systempath => [],
      );
    }

    return FFI::CheckLib::find_lib(
      lib        => '*',
      libpath    => $dir,
      systempath => [],
      recursive  => 1,
    );
  }
}


sub bin_dir {
  my ($class) = @_;
  if($class->install_type('system'))
  {
    my $prop = $class->runtime_prop;
    return () unless defined $prop;
    return () unless defined $prop->{system_bin_dir};
    return ref $prop->{system_bin_dir} ? @{ $prop->{system_bin_dir} } : ($prop->{system_bin_dir});
  }
  else
  {
    my $dir = Path::Tiny->new($class->dist_dir, 'bin');
    return -d $dir ? ("$dir") : ();
  }
}



sub dynamic_dir {
  my ($class) = @_;
  if($class->install_type('system'))
  {
    return ();
  }
  else
  {
    my $dir = Path::Tiny->new($class->dist_dir, 'dynamic');
    return -d $dir ? ("$dir") : ();
  }
}


sub alien_helper {
  {};
}


sub inline_auto_include {
  my ($class) = @_;
  $class->runtime_prop->{inline_auto_include} || $class->config('inline_auto_include') || []
}

sub Inline {
  my ($class, $language) = @_;
  return unless defined $language;
  return if $language !~ /^(C|CPP)$/;
  my $config = {
    # INC should arguably be for -I flags only, but
    # this improves compat with ExtUtils::Depends.
    # see gh#107, gh#108
    INC          => $class->cflags,
    LIBS         => $class->libs,
  };

  if (@{ $class->inline_auto_include } > 0) {
    $config->{AUTO_INCLUDE} = join "\n", map { "#include \"$_\"" } @{ $class->inline_auto_include };
  }

  $config;
}


{
  my %alien_build_config_cache;

  sub runtime_prop
  {
    my($class) = @_;

    if(ref($class))
    {
      # called as an instance method.
      my $self = $class;
      $class = ref $self;
      return $self->{_alt}->{runtime_prop} if defined $self->{_alt};
    }

    return $alien_build_config_cache{$class} if
      exists $alien_build_config_cache{$class};

    $alien_build_config_cache{$class} ||= do {
      my $dist = ref $class ? ref $class : $class;
      $dist =~ s/::/-/g;
      my $dist_dir = eval { _dist_dir $dist };
      return if $@;
      my $alien_json = Path::Tiny->new($dist_dir, '_alien', 'alien.json');
      return unless -r $alien_json;
      my $json = $alien_json->slurp;
      require JSON::PP;
      my $config = JSON::PP::decode_json($json);
      $config->{distdir} = $dist_dir;
      $config;
    };
  }
}


sub alt
{
  my($old, $name) = @_;
  my $new = ref $old ? (ref $old)->new : $old->new;

  my $orig;

  if(ref($old) && defined $old->{_alt})
  { $orig = $old->{_alt}->{orig} }
  else
  { $orig = $old->runtime_prop }

  require Storable;
  my $runtime_prop = Storable::dclone($orig);

  if($runtime_prop->{alt}->{$name})
  {
    foreach my $key (keys %{ $runtime_prop->{alt}->{$name} })
    {
      $runtime_prop->{$key} = $runtime_prop->{alt}->{$name}->{$key};
    }
  }
  else
  {
    Carp::croak("no such alt: $name");
  }

  $new->{_alt} = {
    runtime_prop => $runtime_prop,
    orig         => $orig,
  };

  $new;
}


sub alt_names
{
  my($class) = @_;
  my $alts = $class->runtime_prop->{alt};
  defined $alts
    ? sort keys %$alts
    : ();
}


sub alt_exists
{
  my($class, $alt_name) = @_;
  my $alts = $class->runtime_prop->{alt};
  defined $alts
    ? exists $alts->{$alt_name} && defined $alts->{$alt_name}
    : 0;
}

1;

=pod

=encoding UTF-8

=head1 NAME

Alien::Base - Base classes for Alien:: modules

=head1 VERSION

version 2.88

=head1 SYNOPSIS

 package Alien::MyLibrary;
 
 use strict;
 use warnings;
 
 use parent 'Alien::Base';
 
 1;

(for details on the C<Makefile.PL> or C<Build.PL> and L<alienfile>
that should be bundled with your L<Alien::Base> subclass, please see
L<Alien::Build::Manual::AlienAuthor>).

Then a C<MyLibrary::XS> can use C<Alien::MyLibrary> in its C<Makefile.PL>:

 use Alien::MyLibrary
 use ExtUtils::MakeMaker;
 use Alien::Base::Wrapper qw( Alien::MyLibrary !export );
 use Config;
 
 WriteMakefile(
   ...
   Alien::Base::Wrapper->mm_args,
   ...
 );

Or if you prefer L<Module::Build>, in its C<Build.PL>:

 use Alien::MyLibrary;
 use Module::Build 0.28; # need at least 0.28
 use Alien::Base::Wrapper qw( Alien::MyLibrary !export );
 
 my $builder = Module::Build->new(
   ...
   Alien::Base::Wrapper->mb_args,
   ...
 );
 
 $builder->create_build_script;

Or if you are using L<ExtUtils::Depends>:

 use ExtUtils::MakeMaker;
 use ExtUtils::Depends;
 my $eud = ExtUtils::Depends->new(qw( MyLibrary::XS Alien::MyLibrary ));
 WriteMakefile(
   ...
   $eud->get_makefile_vars
 );

If you are using L<Alien::Base::ModuleBuild> instead of the recommended L<Alien::Build>
and L<alienfile>, then in your C<MyLibrary::XS> module, you may need something like
this in your main C<.pm> file IF your library uses dynamic libraries:

 package MyLibrary::XS;
 
 use Alien::MyLibrary; # may only be needed if you are using Alien::Base::ModuleBuild
 
 ...

Or you can use it from an FFI module:

 package MyLibrary::FFI;
 
 use Alien::MyLibrary;
 use FFI::Platypus;
 use FFI::CheckLib 0.28 qw( find_lib_or_die );
 
 my $ffi = FFI::Platypus->new;
 $ffi->lib(find_lib_or_die lib => 'mylib', alien => ['Alien::MyLibrary']);
 
 $ffi->attach( 'my_library_function' => [] => 'void' );

You can even use it with L<Inline> (C and C++ languages are supported):

 package MyLibrary::Inline;
 
 use Alien::MyLibrary;
 # Inline 0.56 or better is required
 use Inline 0.56 with => 'Alien::MyLibrary';
 ...

=head1 DESCRIPTION

B<NOTE>: L<Alien::Base::ModuleBuild> is no longer bundled with L<Alien::Base> and has been spun off into a separate distribution.
L<Alien::Base::ModuleBuild> will be a prerequisite for L<Alien::Base> until October 1, 2017.  If you are using L<Alien::Base::ModuleBuild>
you need to make sure it is declared as a C<configure_requires> in your C<Build.PL>.  You may want to also consider using L<Alien::Base> and
L<alienfile> as a more modern alternative.

L<Alien::Base> comprises base classes to help in the construction of C<Alien::> modules. Modules in the L<Alien> namespace are used to locate and install (if necessary) external libraries needed by other Perl modules.

This is the documentation for the L<Alien::Base> module itself. If you
are starting out you probably want to do so from one of these documents:

=over 4

=item L<Alien::Build::Manual::AlienUser>

For users of an C<Alien::libfoo> that is implemented using L<Alien::Base>.
(The developer of C<Alien::libfoo> I<should> provide the documentation
necessary, but if not, this is the place to start).

=item L<Alien::Build::Manual::AlienAuthor>

If you are writing your own L<Alien> based on L<Alien::Build> and L<Alien::Base>.

=item L<Alien::Build::Manual::FAQ>

If you have a common question that has already been answered, like
"How do I use L<alienfile> with some build system".

=item L<Alien::Build::Manual::PluginAuthor>

This is for the brave souls who want to write plugins that will work with
L<Alien::Build> + L<alienfile>.

=back

Before using an L<Alien::Base> based L<Alien> directly, please consider the following advice:

If you are wanting to use an L<Alien::Base> based L<Alien> with an XS module using L<ExtUtils::MakeMaker> or L<Module::Build>, it is highly
recommended that you use L<Alien::Base::Wrapper>, rather than using the L<Alien> directly, because it handles a number of sharp edges and avoids
pitfalls common when trying to use an L<Alien> directly with L<ExtUtils::MakeMaker>.

In the same vein, if you are wanting to use an L<Alien::Base> based L<Alien> with an XS module using L<Dist::Zilla> it is highly recommended
that you use L<Dist::Zilla::Plugin::AlienBase::Wrapper> for the same reasons.

As of version 0.28, L<FFI::CheckLib> has a good interface for working with L<Alien::Base> based L<Alien>s in fallback mode, which is
recommended.

You should typically only be using an L<Alien::Base> based L<Alien> directly, if you need to integrate it with some other system, or if it
is a tool based L<Alien> that you don't need to link.

The above synopsis and linked manual documents will lead you down the right path, but it is worth knowing before you read further in this
document.

=head1 METHODS

In the example snippets here, C<Alien::MyLibrary> represents any
subclass of L<Alien::Base>.

=head2 dist_dir

 my $dir = Alien::MyLibrary->dist_dir;

Returns the directory that contains the install root for
the packaged software, if it was built from install (i.e., if
C<install_type> is C<share>).

=head2 new

 my $alien = Alien::MyLibrary->new;

Creates an instance of an L<Alien::Base> object.  This is typically
unnecessary.

=head2 cflags

 my $cflags = Alien::MyLibrary->cflags;
 
 use Text::ParseWords qw( shellwords );
 my @cflags = shellwords( Alien::MyLibrary->cflags );

Returns the C compiler flags necessary to compile an XS
module using the alien software.  If you need this in list
form (for example if you are calling system with a list
argument) you can pass this value into C<shellwords> from
the Perl core L<Text::ParseWords> module.

=head2 cflags_static

 my $cflags = Alien::MyLibrary->cflags_static;

Same as C<cflags> above, but gets the static compiler flags,
if they are different.

=head2 libs

 my $libs = Alien::MyLibrary->libs;
 
 use Text::ParseWords qw( shellwords );
 my @cflags = shellwords( Alien::MyLibrary->libs );

Returns the library linker flags necessary to link an XS
module against the alien software.  If you need this in list
form (for example if you are calling system with a list
argument) you can pass this value into C<shellwords> from
the Perl core L<Text::ParseWords> module.

=head2 libs_static

 my $libs = Alien::MyLibrary->libs_static;

Same as C<libs> above, but gets the static linker flags,
if they are different.

=head2 version

 my $version = Alien::MyLibrary->version;

Returns the version of the alienized library or tool that was
determined at install time.

=head2 atleast_version

=head2 exact_version

=head2 max_version

 my $ok = Alien::MyLibrary->atleast_version($wanted_version);
 my $ok = Alien::MyLibrary->exact_version($wanted_version);
 my $ok = Alien::MyLibrary->max_version($wanted_version);

Returns true if the version of the alienized library or tool is at least,
exactly, or at most the version specified, respectively.

=head2 version_cmp

  $cmp = Alien::MyLibrary->version_cmp($x, $y)

Comparison method used by L</atleast_version>, L</exact_version> and
L</max_version>. May be useful to implement custom comparisons, or for
subclasses to overload to get different version comparison semantics than the
default rules, for packages that have some other rules than the F<pkg-config>
behaviour.

Should return a number less than, equal to, or greater than zero; similar in
behaviour to the C<< <=> >> and C<cmp> operators.

=head2 install_type

 my $install_type = Alien::MyLibrary->install_type;
 my $bool = Alien::MyLibrary->install_type($install_type);

Returns the install type that was used when C<Alien::MyLibrary> was
installed.  

If a type is provided (the second form in the synopsis)
returns true if the actual install type matches.  
For this use case it is recommended to use C<is_system_install> 
or C<is_share_install> instead as these are less prone to 
typographical errors.

Types include:

=over 4

=item system

The library was provided by the operating system

=item share

The library was not available when C<Alien::MyLibrary> was installed, so
it was built from source code, either downloaded from the Internet
or bundled with C<Alien::MyLibrary>.

=back

=head2 is_system_install

 my $type = $build->is_system_install;

Returns true if the alien is a system install type.  

=head2 is_share_install

 my $type = $build->is_share_install;

Returns true if the alien is a share install type.  

=head2 config

 my $value = Alien::MyLibrary->config($key);

Returns the configuration data as determined during the install
of C<Alien::MyLibrary>.  For the appropriate config keys, see
L<Alien::Base::ModuleBuild::API/"CONFIG DATA">.

This is not typically used by L<Alien::Base> and L<alienfile>,
but a compatible interface will be provided.

=head2 dynamic_libs

 my @dlls = Alien::MyLibrary->dynamic_libs;
 my($dll) = Alien::MyLibrary->dynamic_libs;

Returns a list of the dynamic library or shared object files for the
alien software.

=head2 bin_dir

 my(@dir) = Alien::MyLibrary->bin_dir

Returns a list of directories with executables in them.  For a C<system>
install this will be an empty list.  For a C<share> install this will be
a directory under C<dist_dir> named C<bin> if it exists.  You may wish
to override the default behavior if you have executables or scripts that
get installed into non-standard locations.

Example usage:

 use Env qw( @PATH );
 
 unshift @PATH, Alien::MyLibrary->bin_dir;

=head2 dynamic_dir

 my(@dir) = Alien::MyLibrary->dynamic_dir

Returns the dynamic dir for a dynamic build (if the main
build is static).  For a C<share> install this will be a
directory under C<dist_dir> named C<dynamic> if it exists.
System builds return an empty list.

Example usage:

 use Env qw( @PATH );
 
 unshift @PATH, Alien::MyLibrary->dynamic_dir;

=head2 alien_helper

 my $helpers = Alien::MyLibrary->alien_helper;

Returns a hash reference of helpers provided by the Alien module.
The keys are helper names and the values are code references.  The
code references will be executed at command time and the return value
will be interpolated into the command before execution.  The default
implementation returns an empty hash reference, and you are expected
to override the method to create your own helpers.

For use with commands specified in and L<alienfile> or in your C<Build.Pl>
when used with L<Alien::Base::ModuleBuild>.

Helpers allow users of your Alien module to use platform or environment
determined logic to compute command names or arguments in your installer
logic.  Helpers allow you to do this without making your Alien module a
requirement when a build from source code is not necessary.

As a concrete example, consider L<Alien::gmake>, which provides the
helper C<gmake>:

 package Alien::gmake;
 
 ...
 
 sub alien_helper {
   my($class) = @_;
   return {
     gmake => sub {
       # return the executable name for GNU make,
       # usually either make or gmake depending on
       # the platform and environment
       $class->exe;
     }
   },
 }

Now consider L<Alien::nasm>.  C<nasm> requires GNU Make to build from
source code, but if the system C<nasm> package is installed we don't
need it.  From the L<alienfile> of C<Alien::nasm>:

 use alienfile;
 
 plugin 'Probe::CommandLine' => (
   command => 'nasm',
   args    => ['-v'],
   match   => qr/NASM version/,
 );
 
 share {
   ...
   plugin 'Extract' => 'tar.gz';
   plugin 'Build::MSYS';
 
   build [
     'sh configure --prefix=%{alien.install.prefix}',
     '%{gmake}',
     '%{gmake} install',
   ];
 };
 
 ...

=head2 inline_auto_include

 my(@headers) = Alien::MyLibrary->inline_auto_include;

List of header files to automatically include in inline C and C++
code when using L<Inline::C> or L<Inline::CPP>.  This is provided
as a public interface primarily so that it can be overridden at run
time.  This can also be specified in your C<Build.PL> with
L<Alien::Base::ModuleBuild> using the C<alien_inline_auto_include>
property.

=head2 runtime_prop

 my $hashref = Alien::MyLibrary->runtime_prop;

Returns a hash reference of the runtime properties computed by L<Alien::Build> during its
install process.  If the L<Alien::Base> based L<Alien> was not built using L<Alien::Build>,
then this will return undef.

=head2 alt

 my $new_alien = Alien::MyLibrary->alt($alt_name);
 my $new_alien = $old_alien->alt($alt_name);

Returns an L<Alien::Base> instance with the alternate configuration.

Some packages come with multiple libraries, and multiple C<.pc> files to
use with them.  This method can be used with C<pkg-config> plugins to
access different configurations.  (It could also be used with non-pkg-config
based packages too, though there are not as of this writing any build
time plugins that take advantage of this feature).

From your L<alienfile>

 use alienfile;
 
 plugin 'PkgConfig' => (
   pkg_name => [ 'libfoo', 'libbar', ],
 );

Then in your base class works like normal:

 package Alien::MyLibrary;
 
 use parent qw( Alien::Base );
 
 1;

Then you can use it:

 use Alien::MyLibrary;
 
 my $cflags = Alien::MyLibrary->alt('foo1')->cflags;
 my $libs   = Alien::MyLibrary->alt('foo1')->libs;

=head2 alt_names

 my @alt_names = Alien::MyLibrary->alt_names

Returns the list of all available alternative configuration names.

=head2 alt_exists

 my $bool = Alien::MyLibrary->alt_exists($alt_name)

Returns true if the given alternative configuration exists.

=head1 SUPPORT AND CONTRIBUTING

First check the L<Alien::Build::Manual::FAQ> for questions that have already been answered.

IRC: #native on irc.perl.org

L<(click for instant chatroom login)|http://chat.mibbit.com/#native@irc.perl.org>

If you find a bug, please report it on the projects issue tracker on GitHub:

=over 4

=item L<https://github.com/PerlAlien/Alien-Build/issues>

=back

Development is discussed on the projects google groups.  This is also
a reasonable place to post a question if you don't want to open an issue
in GitHub.

=over 4

=item L<https://groups.google.com/forum/#!forum/perl5-alien>

=back

If you have implemented a new feature or fixed a bug, please open a pull
request.

=over 4

=item L<https://github.com/PerlAlien/Alien-Build/pulls>

=back

=head1 SEE ALSO

=over

=item *

L<Alien::Build>

=item *

L<alienfile>

=item *

L<Alien>

=item *

L<Alien::Build::Manual::FAQ>

=back

=head1 THANKS

C<Alien::Base> was originally written by Joel Berger, and that
code is still Copyright (C) 2012-2017 Joel Berger.  It has the
same license as the rest of the L<Alien::Build>.

Special thanks for the early development of C<Alien::Base> go to:

=over

=item Christian Walde (Mithaldu)

For productive conversations about component interoperability.

=item kmx

For writing Alien::Tidyp from which I drew many of my initial ideas.

=item David Mertens (run4flat)

For productive conversations about implementation.

=item Mark Nunberg (mordy, mnunberg)

For graciously teaching me about rpath and dynamic loading,

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

__END__
__POD__

