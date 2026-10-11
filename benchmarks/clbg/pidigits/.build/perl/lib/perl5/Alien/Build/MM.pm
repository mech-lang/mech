package Alien::Build::MM;

use strict;
use warnings;
use 5.008004;
use Alien::Build;
use Path::Tiny ();
use Capture::Tiny qw( capture );
use Carp ();

# ABSTRACT: Alien::Build installer code for ExtUtils::MakeMaker
our $VERSION = '2.88'; # VERSION


sub new
{
  my($class, %prop) = @_;

  my $self = bless {}, $class;

  my %meta = map { $_ => $prop{$_} } grep /^my_/, keys %prop;

  my $build = $self->{build} =
    Alien::Build->load('alienfile',
      root     => "_alien",
      (-d 'patch' ? (patch => 'patch') : ()),
      meta_prop => \%meta,
    )
  ;

  if(%meta)
  {
    $build->meta->add_requires(configure => 'Alien::Build::MM' => '1.20');
    $build->meta->add_requires(configure => 'Alien::Build' => '1.20');
  }

  if(defined $prop{alienfile_meta})
  {
    $self->{alienfile_meta} = $prop{alienfile_meta};
  }
  else
  {
    $self->{alienfile_meta} = 1;
  }

  $self->{clean_install} = $prop{clean_install};

  $self->build->load_requires('configure');
  $self->build->root;
  $self->build->checkpoint;

  $self;
}


sub build
{
  shift->{build};
}


sub alienfile_meta
{
  shift->{alienfile_meta};
}


sub clean_install
{
  shift->{clean_install};
}


sub mm_args
{
  my($self, %args) = @_;

  if($args{DISTNAME})
  {
    $self->build->set_stage(Path::Tiny->new("blib/lib/auto/share/dist/$args{DISTNAME}")->absolute->stringify);
    $self->build->install_prop->{mm}->{distname} = $args{DISTNAME};
    my $module = $args{DISTNAME};
    $module =~ s/-/::/g;
    # See if there is an existing version installed, without pulling it into this process
    my($old_prefix, $err, $ret) = capture { system $^X, "-M$module", -e => "print $module->dist_dir"; $? };
    if($ret == 0)
    {
      chomp $old_prefix;
      my $file = Path::Tiny->new($old_prefix, qw( _alien alien.json ));
      if(-r $file)
      {
        my $old_runtime = eval {
          require JSON::PP;
          JSON::PP::decode_json($file->slurp);
        };
        unless($@)
        {
          $self->build->install_prop->{old}->{runtime} = $old_runtime;
          $self->build->install_prop->{old}->{prefix}  = $old_prefix;
        }
      }
    }
  }
  else
  {
    Carp::croak "DISTNAME is required";
  }

  my $ab_version = '0.25';

  if($self->clean_install)
  {
    $ab_version = '1.74';
  }

  $args{CONFIGURE_REQUIRES} = Alien::Build::_merge(
    'Alien::Build::MM' => $ab_version,
    %{ $args{CONFIGURE_REQUIRES} || {} },
    %{ $self->build->requires('configure') || {} },
  );

  if($self->build->install_type eq 'system')
  {
    $args{BUILD_REQUIRES} = Alien::Build::_merge(
      'Alien::Build::MM' => $ab_version,
      %{ $args{BUILD_REQUIRES} || {} },
      %{ $self->build->requires('system') || {} },
    );
  }
  elsif($self->build->install_type eq 'share')
  {
    $args{BUILD_REQUIRES} = Alien::Build::_merge(
      'Alien::Build::MM' => $ab_version,
      %{ $args{BUILD_REQUIRES} || {} },
      %{ $self->build->requires('share') || {} },
    );
  }
  else
  {
    die "unknown install type: @{[ $self->build->install_type ]}"
  }

  $args{PREREQ_PM} = Alien::Build::_merge(
    'Alien::Build' => $ab_version,
    %{ $args{PREREQ_PM} || {} },
  );

  #$args{META_MERGE}->{'meta-spec'}->{version} = 2;
  $args{META_MERGE}->{dynamic_config} = 1;

  if($self->alienfile_meta)
  {
    $args{META_MERGE}->{x_alienfile} = {
      generated_by => "@{[ __PACKAGE__ ]} version @{[ __PACKAGE__->VERSION || 'dev' ]}",
      requires => {
        map {
          my %reqs = %{ $self->build->requires($_) };
          $reqs{$_} = "$reqs{$_}" for keys %reqs;
          $_ => \%reqs;
        } qw( share system )
      },
    };
  }

  $self->build->checkpoint;
  %args;
}


sub mm_postamble
{
  # NOTE: older versions of the Alien::Build::MM documentation
  # didn't include $mm and @rest args, so anything that this
  # method uses them for has to be optional.
  # (as of this writing they are unused, but are being added
  #  to match the way mm_install works).

  my($self, $mm, @rest) = @_;

  my $postamble = '';

  # remove the _alien directory on a make realclean:
  $postamble .= "realclean :: alien_realclean\n" .
                "\n" .
                "alien_realclean:\n" .
                "\t\$(RM_RF) _alien\n\n";

  # remove the _alien directory on a make clean:
  $postamble .= "clean :: alien_clean\n" .
                "\n" .
                "alien_clean:\n" .
                "\t\$(RM_RF) _alien\n\n";

  my $dirs = $self->build->meta_prop->{arch}
    ? '$(INSTALLARCHLIB) $(INSTALLSITEARCH) $(INSTALLVENDORARCH)'
    : '$(INSTALLPRIVLIB) $(INSTALLSITELIB) $(INSTALLVENDORLIB)'
  ;

  # set prefix
  $postamble .= "alien_prefix : _alien/mm/prefix\n\n" .
                "_alien/mm/prefix :\n" .
                "\t\$(FULLPERL) -MAlien::Build::MM=cmd -e prefix \$(INSTALLDIRS) $dirs\n\n";

  # set version
  $postamble .= "alien_version : _alien/mm/version\n\n" .
                "_alien/mm/version : _alien/mm/prefix\n" .
                "\t\$(FULLPERL) -MAlien::Build::MM=cmd -e version \$(VERSION)\n\n";

  # download
  $postamble .= "alien_download : _alien/mm/download\n\n" .
                "_alien/mm/download : _alien/mm/prefix _alien/mm/version\n" .
                "\t\$(FULLPERL) -MAlien::Build::MM=cmd -e download\n\n";

  # build
  $postamble .= "alien_build : _alien/mm/build\n\n" .
                "_alien/mm/build : _alien/mm/download\n" .
                "\t\$(FULLPERL) -MAlien::Build::MM=cmd -e build\n\n";

  # append to all
  $postamble .= "pure_all :: _alien/mm/build\n\n";

  $postamble .= "subdirs-test_dynamic subdirs-test_static subdirs-test :: alien_test\n\n";
  $postamble .= "alien_test :\n" .
                "\t\$(FULLPERL) -MAlien::Build::MM=cmd -e test\n\n";

  # prop
  $postamble .= "alien_prop :\n" .
                "\t\$(FULLPERL) -MAlien::Build::MM=cmd -e dumpprop\n\n";
  $postamble .= "alien_prop_meta :\n" .
                "\t\$(FULLPERL) -MAlien::Build::MM=cmd -e dumpprop meta\n\n";
  $postamble .= "alien_prop_install :\n" .
                "\t\$(FULLPERL) -MAlien::Build::MM=cmd -e dumpprop install\n\n";
  $postamble .= "alien_prop_runtime :\n" .
                "\t\$(FULLPERL) -MAlien::Build::MM=cmd -e dumpprop runtime\n\n";

  # install
  $postamble .= "alien_clean_install : _alien/mm/prefix\n" .
                "\t\$(FULLPERL) -MAlien::Build::MM=cmd -e clean_install\n\n";

  $postamble;
}


sub mm_install
{
  # NOTE: older versions of the Alien::Build::MM documentation
  # didn't include this method, so anything that this method
  # does has to be optional

  my($self, $mm, @rest) = @_;

  my $section = do {
    package
      MY;
    $mm->SUPER::install(@rest);
  };

  return
      ".NOTPARALLEL : \n\n"
    . ".NO_PARALLEL : \n\n"
    . "install :: alien_clean_install\n\n"
    . $section;
}

sub import
{
  my(undef, @args) = @_;
  foreach my $arg (@args)
  {
    if($arg eq 'cmd')
    {
      package main;

      *_args = sub
      {
        my $build = Alien::Build->resume('alienfile', '_alien');
        $build->load_requires('configure');
        $build->load_requires($build->install_type);
        ($build, @ARGV)
      };

      *_touch = sub {
        my($name) = @_;
        my $path = Path::Tiny->new("_alien/mm/$name");
        $path->parent->mkpath;
        $path->touch;
      };

      *prefix = sub
      {
        my($build, $type, $perl, $site, $vendor) = _args();

        my $distname = $build->install_prop->{mm}->{distname};

        my $prefix = $type eq 'perl'
          ? $perl
          : $type eq 'site'
            ? $site
            : $type eq 'vendor'
              ? $vendor
              : die "unknown INSTALLDIRS ($type)";
        $prefix = Path::Tiny->new($prefix)->child("auto/share/dist/$distname")->absolute->stringify;

        $build->log("prefix $prefix");
        $build->set_prefix($prefix);
        $build->checkpoint;
        _touch('prefix');
      };

      *version = sub
      {
        my($build, $version) = _args();

        $build->runtime_prop->{perl_module_version} = $version;
        $build->checkpoint;
        _touch('version');
      };

      *download = sub
      {
        my($build) = _args();
        $build->download;
        $build->checkpoint;
       _touch('download');
      };

      *build = sub
      {
        my($build) = _args();

        $build->build;

        my $distname = $build->install_prop->{mm}->{distname};

        if($build->meta_prop->{arch})
        {
          my $archdir = Path::Tiny->new("blib/arch/auto/@{[ join '/', split /-/, $distname ]}");
          $archdir->mkpath;
          my $archfile = $archdir->child($archdir->basename . '.txt');
          $archfile->spew('Alien based distribution with architecture specific file in share');
        }

        my $cflags = $build->runtime_prop->{cflags};
        my $libs   = $build->runtime_prop->{libs};

        if(($cflags && $cflags !~ /^\s*$/)
        || ($libs   && $libs   !~ /^\s*$/))
        {
          my $mod = join '::', split /-/, $distname;
          my $install_files_pm = Path::Tiny->new("blib/lib/@{[ join '/', split /-/, $distname ]}/Install/Files.pm");
          $install_files_pm->parent->mkpath;
          $install_files_pm->spew(
            "package ${mod}::Install::Files;\n",
            "use strict;\n",
            "use warnings;\n",
            "require ${mod};\n",
            "sub Inline { shift; ${mod}->Inline(\@_) }\n",
            "1;\n",
            "\n",
            "=begin Pod::Coverage\n",
            "\n",
            "  Inline\n",
            "\n",
            "=cut\n",
          ) unless -f "$install_files_pm";
        }

        $build->checkpoint;
        _touch('build');
      };

      *test = sub
      {
        my($build) = _args();
        $build->test;
        $build->checkpoint;
      };

      *clean_install = sub
      {
        my($build) = _args();
        $build->clean_install;
        $build->checkpoint;
      };

      *dumpprop = sub
      {
        my($build, $type) = _args();

        my %h = (
          meta    => $build->meta_prop,
          install => $build->install_prop,
          runtime => $build->runtime_prop,
        );

        require Alien::Build::Util;
        print Alien::Build::Util::_dump($type ? $h{$type} : \%h);
      }
    }
  }
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::MM - Alien::Build installer code for ExtUtils::MakeMaker

=head1 VERSION

version 2.88

=head1 SYNOPSIS

In your C<Makefile.PL>:

 use ExtUtils::MakeMaker;
 use Alien::Build::MM;
 
 my $abmm = Alien::Build::MM->new;
 
 WriteMakefile($abmm->mm_args(
   ABSTRACT     => 'Discover or download and install libfoo',
   DISTNAME     => 'Alien-Libfoo',
   NAME         => 'Alien::Libfoo',
   VERSION_FROM => 'lib/Alien/Libfoo.pm',
   ...
 ));
 
 sub MY::postamble {
   $abmm->mm_postamble(@_);
 }
 
 sub MY::install {
   $abmm->mm_install(@_);
 }

In your C<lib/Alien/Libfoo.pm>:

 package Alien::Libfoo;
 use parent qw( Alien::Base );
 1;

In your alienfile (needs to be named C<alienfile> and should be in the root of your dist):

 use alienfile;
 
 plugin 'PkgConfig' => 'libfoo';
 
 share {
   start_url 'http://libfoo.org';
   ...
 };

=head1 DESCRIPTION

This class allows you to use Alien::Build and Alien::Base with L<ExtUtils::MakeMaker>.
It load the L<alienfile> recipe in the root of your L<Alien> dist, updates the prereqs
passed into C<WriteMakefile> if any are specified by your L<alienfile> or its plugins,
and adds a postamble to the C<Makefile> that will download/build/test the alienized
package as appropriate.

The L<alienfile> must be named C<alienfile>.

If you are using L<Dist::Zilla> to author your L<Alien> dist, you should consider using
the L<Dist::Zilla::Plugin::AlienBuild> plugin.

I personally don't recommend it, but if you want to use L<Module::Build> instead, you
can use L<Alien::Build::MB>.

=head1 CONSTRUCTOR

=head2 new

 my $abmm = Alien::Build::MM->new;

Create a new instance of L<Alien::Build::MM>.

=head1 PROPERTIES

=head2 build

 my $build = $abmm->build;

The L<Alien::Build> instance.

=head2 alienfile_meta

 my $bool = $abmm->alienfile_meta

Set to a false value, in order to turn off the x_alienfile meta

=head2 clean_install

 my $bool = $abmm->clean_install;

Set to a true value, in order to clean the share directory prior to
installing.  If you use this you have to make sure that you install
the install handler in your C<Makefile.PL>:

 $abmm = Alien::Build::MM->new(
   clean_install => 1,
 );
 
 ...
 
 sub MY::install {
   $abmm->mm_install(@_);
 }

=head1 METHODS

=head2 mm_args

 my %args = $abmm->mm_args(%args);

Adjust the arguments passed into C<WriteMakefile> as needed by L<Alien::Build>.

=head2 mm_postamble

 my $postamble $abmm->mm_postamble;
 my $postamble $abmm->mm_postamble($mm);

Returns the postamble for the C<Makefile> needed for L<Alien::Build>.
This adds the following C<make> targets which are normally called when
you run C<make all>, but can be run individually if needed for debugging.

=over 4

=item alien_prefix

Determines the final install prefix (C<%{.install.prefix}>).

=item alien_version

Determine the perl_module_version (C<%{.runtime.perl_module_version}>)

=item alien_download

Downloads the source from the internet.  Does nothing for a system install.

=item alien_build

Build from source (if a share install).  Gather configuration (for either
system or share install).

=item alien_prop, alien_prop_meta, alien_prop_install, alien_prop_runtime

Prints the meta, install and runtime properties for the Alien.

=item alien_realclean, alien_clean

Removes the alien specific files.  These targets are executed when you call
the C<realclean> and C<clean> targets respectively.

=item alien_clean_install

Cleans out the Alien's share directory.  Caution should be used in invoking
this target directly, as if you do not understand what you are doing you
are likely to break your already installed Alien.

=back

=head2 mm_install

 sub MY::install {
   $abmm->mm_install(@_);
 }

B<EXPERIMENTAL>

Adds an install rule to clean the final install dist directory prior to installing.

=head1 SEE ALSO

L<Alien::Build>, L<Alien::Base>, L<Alien>, L<Dist::Zilla::Plugin::AlienBuild>, L<Alien::Build::MB>

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
