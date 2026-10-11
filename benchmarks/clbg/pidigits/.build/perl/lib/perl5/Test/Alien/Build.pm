package Test::Alien::Build;

use strict;
use warnings;
use 5.008004;
use Exporter qw( import );
use Path::Tiny qw( path );
use Carp qw( croak );
use Test2::API qw( context run_subtest );
use Capture::Tiny qw( capture_merged );
use Alien::Build::Util qw( _mirror );
use List::Util 1.33 qw( any );
use Alien::Build::Temp;

our @EXPORT = qw(
  alienfile
  alienfile_ok
  alienfile_skip_if_missing_prereqs
  alien_download_ok
  alien_extract_ok
  alien_build_ok
  alien_build_clean
  alien_clean_install
  alien_install_type_is
  alien_checkpoint_ok
  alien_resume_ok
  alien_subtest
  alien_rc
);

# ABSTRACT: Tools for testing Alien::Build + alienfile
our $VERSION = '2.88'; # VERSION


my $build;
my $build_alienfile;
my $build_root;
my $build_targ;

sub alienfile::targ
{
  $build_targ;
}

sub alienfile
{
  my($package, $filename, $line) = caller;
  ($package, $filename, $line) = caller(2) if $package eq __PACKAGE__;
  $filename = path($filename)->absolute;
  my %args = @_ == 0 ? (filename => 'alienfile') : @_ % 2 ? ( source => do { '# line '. $line . ' "' . path($filename)->absolute . qq("\n) . $_[0] }) : @_;

  require alienfile;
  push @alienfile::EXPORT, 'targ' unless any { /^targ$/ } @alienfile::EXPORT;

  my $temp = Alien::Build::Temp->newdir;
  my $get_temp_root = do{
    my $root; # may be undef;
    sub {
      $root ||= Path::Tiny->new($temp);

      if(@_)
      {
        my $path = $root->child(@_);
        $path->mkpath;
        $path;
      }
      else
      {
        return $root;
      }
    };
  };

  if($args{source})
  {
    my $file = $get_temp_root->()->child('alienfile');
    $file->spew_utf8($args{source});
    $args{filename} = $file->stringify;
  }
  else
  {
    unless(defined $args{filename})
    {
      croak "You must specify at least one of filename or source";
    }
    $args{filename} = path($args{filename})->absolute->stringify;
  }

  $args{stage}  ||= $get_temp_root->('stage')->stringify;
  $args{prefix} ||= $get_temp_root->('prefix')->stringify;
  $args{root}   ||= $get_temp_root->('root')->stringify;

  require Alien::Build;

  _alienfile_clear();
  my $out = capture_merged {
    $build_targ = $args{targ};
    $build = Alien::Build->load($args{filename}, root => $args{root});
    $build->set_stage($args{stage});
    $build->set_prefix($args{prefix});
  };

  my $ctx = context();
  $ctx->note($out) if $out;
  $ctx->release;

  $build_alienfile = $args{filename};
  $build_root      = $temp;
  $build
}

sub _alienfile_clear
{
  eval { defined $build_root && -d $build_root && path($build_root)->remove_tree };
  undef $build;
  undef $build_alienfile;
  undef $build_root;
  undef $build_targ;
}


sub alienfile_ok
{
  my $build;
  my $name;
  my $error;

  if(@_ == 1 && ! defined $_[0])
  {
    $build = $_[0];
    $error = 'no alienfile given';
    $name = 'alienfile compiled';
  }
  elsif(@_ == 1 && eval { $_[0]->isa('Alien::Build') })
  {
    $build = $_[0];
    $name = 'alienfile compiled';
  }
  else
  {
    $build = eval { alienfile(@_) };
    $error = $@;
    $name = 'alienfile compiles';
  }

  my $ok = !! $build;

  my $ctx = context();
  $ctx->ok($ok, $name);
  $ctx->diag("error: $error") if $error;
  $ctx->release;

  $build;
}


sub alienfile_skip_if_missing_prereqs
{
  my($phase) = @_;

  if($build)
  {
    eval { $build->load_requires('configure', 1) };
    if(my $error = $@)
    {
      my $reason = "Missing configure prereq";
      if($error =~ /Required (.*) (.*),/)
      {
        $reason .= ": $1 $2";
      }
      my $ctx = context();
      $ctx->plan(0, SKIP => $reason);
      $ctx->release;
      return;
    }
    $phase ||= $build->install_type;
    eval { $build->load_requires($phase, 1) };
    if(my $error = $@)
    {
      my $reason = "Missing $phase prereq";
      if($error =~ /Required (.*) (.*),/)
      {
        $reason .= ": $1 $2";
      }
      my $ctx = context();
      $ctx->plan(0, SKIP => $reason);
      $ctx->release;
      return;
    }
  }
}


sub alien_install_type_is
{
  my($type, $name) = @_;

  croak "invalid install type" unless defined $type && $type =~ /^(system|share)$/;
  $name ||= "alien install type is $type";

  my $ok = 0;
  my @diag;

  if($build)
  {
    my($out, $actual) = capture_merged {
      $build->load_requires('configure');
      $build->install_type;
    };
    if($type eq $actual)
    {
      $ok = 1;
    }
    else
    {
      push @diag, "expected install type of $type, but got $actual";
    }
  }
  else
  {
    push @diag, 'no alienfile'
  }

  my $ctx = context();
  $ctx->ok($ok, $name);
  $ctx->diag($_) for @diag;
  $ctx->release;

  $ok;
}


sub alien_download_ok
{
  my($name) = @_;

  $name ||= 'alien download';

  my $ok;
  my $file;
  my @diag;
  my @note;

  if($build)
  {
    my($out, $error) = capture_merged {
      eval {
        $build->load_requires('configure');
        $build->load_requires($build->install_type);
        $build->download;
      };
      $@;
    };
    if($error)
    {
      $ok = 0;
      push @diag, $out if defined $out;
      push @diag, "extract threw exception: $error";
    }
    else
    {
      $file = $build->install_prop->{download};
      if(-d $file || -f $file)
      {
        $ok = 1;
        push @note, $out if defined $out;
      }
      else
      {
        $ok = 0;
        push @diag, $out if defined $out;
        push @diag, 'no file or directory';
      }
    }
  }
  else
  {
    $ok = 0;
    push @diag, 'no alienfile';
  }

  my $ctx = context();
  $ctx->ok($ok, $name);
  $ctx->note($_) for @note;
  $ctx->diag($_) for @diag;
  $ctx->release;

  $file;
}


sub alien_extract_ok
{
  my($archive, $name) = @_;

  $name ||= $archive ? "alien extraction of $archive" : 'alien extraction';
  my $ok;
  my $dir;
  my @diag;
  my @note;

  if($build)
  {
    my($out, $error);
    ($out, $dir, $error) = capture_merged {
      my $dir = eval {
        $build->load_requires('configure');
        $build->load_requires($build->install_type);
        $build->download;
        $build->extract($archive);
      };
      ($dir, $@);
    };
    if($error)
    {
      $ok = 0;
      push @diag, $out if defined $out;
      push @diag, "extract threw exception: $error";
    }
    else
    {
      if(-d $dir)
      {
        $ok = 1;
        push @note, $out if defined $out;
      }
      else
      {
        $ok = 0;
        push @diag, $out if defined $out;
        push @diag, 'no directory';
      }
    }
  }
  else
  {
    $ok = 0;
    push @diag, 'no alienfile';
  }

  my $ctx = context();
  $ctx->ok($ok, $name);
  $ctx->note($_) for @note;
  $ctx->diag($_) for @diag;
  $ctx->release;

  $dir;
}


my $count = 1;

sub alien_build_ok
{
  my $opt = defined $_[0] && ref($_[0]) eq 'HASH'
    ? shift : { class => 'Alien::Base' };

  my($name) = @_;

  $name ||= 'alien builds okay';
  my $ok;
  my @diag;
  my @note;
  my $alien;

  if($build)
  {
    my($out,$error) = capture_merged {
      eval {
        $build->load_requires('configure');
        $build->load_requires($build->install_type);
        $build->download;
        $build->build;
      };
      $@;
    };
    if($error)
    {
      $ok = 0;
      push @diag, $out if defined $out;
      push @diag, "build threw exception: $error";
    }
    else
    {
      $ok = 1;

      push @note, $out if defined $out;

      require Alien::Base;

      my $prefix = $build->runtime_prop->{prefix};
      my $stage  = $build->install_prop->{stage};
      my %prop   = %{ $build->runtime_prop };

      $prop{distdir} = $prefix;

      _mirror $stage, $prefix;

      my $dist_dir = sub {
        $prefix;
      };

      my $runtime_prop = sub {
        \%prop;
      };

      $alien = sprintf 'Test::Alien::Build::Faux%04d', $count++;
      {
        no strict 'refs';
        @{ "${alien}::ISA" }          = $opt->{class};
        *{ "${alien}::dist_dir" }     = $dist_dir;
        *{ "${alien}::runtime_prop" } = $runtime_prop;
      }
    }
  }
  else
  {
    $ok = 0;
    push @diag, 'no alienfile';
  }

  my $ctx = context();
  $ctx->ok($ok, $name);
  $ctx->diag($_) for @diag;
  $ctx->note($_) for @note;
  $ctx->release;

  $alien;
}


sub alien_build_clean
{
  my $ctx = context();
  if($build_root)
  {
    foreach my $child (path($build_root)->children)
    {
      next if $child->basename eq 'prefix';
      $ctx->note("clean: rm: $child");
      $child->remove_tree;
    }
  }
  else
  {
    $ctx->note("no build to clean");
  }
  $ctx->release;
}


sub alien_clean_install
{
  my($name) = @_;

  $name ||= "run clean_install";

  my $ok;
  my @diag;
  my @note;

  if($build)
  {
    my($out,$error) = capture_merged {
      eval {
        $build->clean_install;
      };
      $@;
    };
    if($error)
    {
      $ok = 0;
      push @diag, $out if defined $out && $out ne '';
      push @diag, "build threw exception: $error";
    }
    else
    {
      $ok = 1;
      push @note, $out if defined $out && $out ne '';
    }
  }
  else
  {
    $ok = 0;
    push @diag, 'no alienfile';
  }

  my $ctx = context();
  $ctx->ok($ok, $name);
  $ctx->diag($_) for @diag;
  $ctx->note($_) for @note;
  $ctx->release;
}


sub alien_checkpoint_ok
{
  my($name) = @_;

  $name ||= "alien checkpoint ok";
  my $ok;
  my @diag;

  if($build)
  {
    eval { $build->checkpoint };
    if($@)
    {
      push @diag, "error in checkpoint: $@";
      $ok = 0;
    }
    else
    {
      $ok = 1;
    }
    undef $build;
  }
  else
  {
    push @diag, "no build to checkpoint";
    $ok = 0;
  }

  my $ctx = context();
  $ctx->ok($ok, $name);
  $ctx->diag($_) for @diag;
  $ctx->release;

  $ok;
}


sub alien_resume_ok
{
  my($name) = @_;

  $name ||= "alien resume ok";
  my $ok;
  my @diag;

  if($build_alienfile && $build_root && !defined $build)
  {
    $build = eval { Alien::Build->resume($build_alienfile, "$build_root/root") };
    if($@)
    {
      push @diag, "error in resume: $@";
      $ok = 0;
    }
    else
    {
      $ok = 1;
    }
  }
  else
  {
    if($build)
    {
      push @diag, "build has not been checkpointed";
    }
    else
    {
      push @diag, "no build to resume";
    }
    $ok = 0;
  }

  my $ctx = context();
  $ctx->ok($ok, $name);
  $ctx->diag($_) for @diag;
  $ctx->release;

  ($ok && $build) || $ok;
}


my $alien_rc_root;

sub alien_rc
{
  my($code) = @_;

  croak "passed in undef rc" unless defined $code;
  croak "looks like you have already defined a rc.pl file" if $ENV{ALIEN_BUILD_RC} ne '-';

  my(undef, $filename, $line) = caller;
  my $code2 = "use strict; use warnings;\n" .
              '# line ' . $line . ' "' . path($filename)->absolute . "\n$code";

  $alien_rc_root ||= Alien::Build::Temp->newdir;

  my $rc = path($alien_rc_root)->child('rc.pl');
  $rc->spew_utf8($code2);
  $ENV{ALIEN_BUILD_RC} = "$rc";
  return 1;
}


sub alien_subtest
{
  my($name, $code, @args) = @_;

  _alienfile_clear;

  my $ctx = context();
  my $pass = run_subtest($name, $code, { buffered => 1 }, @args);
  $ctx->release;

  _alienfile_clear;

  $pass;
}

delete $ENV{$_} for qw( ALIEN_BUILD_LOG ALIEN_BUILD_PRELOAD ALIEN_BUILD_POSTLOAD ALIEN_INSTALL_TYPE PKG_CONFIG_PATH ALIEN_BUILD_PKG_CONFIG );
$ENV{ALIEN_BUILD_RC} = '-';

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Test::Alien::Build - Tools for testing Alien::Build + alienfile

=head1 VERSION

version 2.88

=head1 SYNOPSIS

 use Test2::V0;
 use Test::Alien::Build;
 
 # returns an instance of Alien::Build.
 my $build = alienfile_ok q{
   use alienfile;
 
   plugin 'My::Plugin' => (
     foo => 1,
     bar => 'string',
     ...
   );
 };
 
 alien_build_ok 'builds okay.';
 
 done_testing;

=head1 DESCRIPTION

This module provides some tools for testing L<Alien::Build> and L<alienfile>.  Outside of L<Alien::Build>
core development, It is probably most useful for L<Alien::Build::Plugin> developers.

This module also unsets a number of L<Alien::Build> specific environment variables, in order to make tests
reproducible even when overrides are set in different environments.  So if you want to test those variables in
various states you should explicitly set them in your test script.  These variables are unset if they defined:
C<ALIEN_BUILD_PRELOAD> C<ALIEN_BUILD_POSTLOAD> C<ALIEN_INSTALL_TYPE>.

=head1 FUNCTIONS

=head2 alienfile

 my $build = alienfile;
 my $build = alienfile q{ use alienfile ... };
 my $build = alienfile filename => 'alienfile';

Create a Alien::Build instance from the given L<alienfile>.  The first two forms are abbreviations.

 my $build = alienfile;
 # is the same as
 my $build = alienfile filename => 'alienfile';

and

 my $build = alienfile q{ use alienfile ... };
 # is the same as
 my $build = alienfile source => q{ use alienfile ... };

Except for the second abbreviated form sets the line number before feeding the source into L<Alien::Build>
so that you will get diagnostics with the correct line numbers.

=over 4

=item source

The source for the alienfile as a string.  You must specify one of C<source> or C<filename>.

=item filename

The filename for the alienfile.  You must specify one of C<source> or C<filename>.

=item root

The build root.

=item stage

The staging area for the build.

=item prefix

The install prefix for the build.

=back

=head2 alienfile_ok

 my $build = alienfile_ok;
 my $build = alienfile_ok q{ use alienfile ... };
 my $build = alienfile_ok filename => 'alienfile';
 my $build = alienfile_ok $build;

Same as C<alienfile> above, except that it runs as a test, and will not throw an exception
on failure (it will return undef instead).

[version 1.49]

As of version 1.49 you can also pass in an already formed instance of L<Alien::Build>.  This
allows you to do something like this:

 subtest 'a subtest' => sub {
   my $build = alienfile q{ use alienfile; ... };
   alienfile_skip_if_missing_prereqs; # skip if alienfile prereqs are missing
   alienfile_ok $build;  # delayed pass/fail for the compile of alienfile
 };

=head2 alienfile_skip_if_missing_prereqs

 alienfile_skip_if_missing_prereqs;
 alienfile_skip_if_missing_prereqs $phase;

Skips the test or subtest if the prereqs for the alienfile are missing.
If C<$phase> is not given, then either C<share> or C<system> will be
detected.

=head2 alien_install_type_is

 alien_install_type_is $type;
 alien_install_type_is $type, $name;

Simple test to see if the install type is what you expect.
C<$type> should be one of C<system> or C<share>.

=head2 alien_download_ok

 my $file = alien_download_ok;
 my $file = alien_download_ok $name;

Makes a download attempt and test that a file or directory results.  Returns
the file or directory if successful.  Returns C<undef> otherwise.

=head2 alien_extract_ok

 my $dir = alien_extract_ok;
 my $dir = alien_extract_ok $archive;
 my $dir = alien_extract_ok $archive, $name;
 my $dir = alien_extract_ok undef, $name;

Makes an extraction attempt and test that a directory results.  Returns
the directory if successful.  Returns C<undef> otherwise.

=head2 alien_build_ok

 my $alien = alien_build_ok;
 my $alien = alien_build_ok $name;
 my $alien = alien_build_ok { class => $class };
 my $alien = alien_build_ok { class => $class }, $name;

Runs the download and build stages.  Passes if the build succeeds.  Returns an instance
of L<Alien::Base> which can be passed into C<alien_ok> from L<Test::Alien>.  Returns
C<undef> if the test fails.

Options

=over 4

=item class

The base class to use for your alien.  This is L<Alien::Base> by default.  Should
be a subclass of L<Alien::Base>, or at least adhere to its API.

=back

=head2 alien_build_clean

 alien_build_clean;

Removes all files with the current build, except for the runtime prefix.
This helps test that the final install won't depend on the build files.

=head2 alien_clean_install

 alien_clean_install;

Runs C<$build-E<gt>clean_install>, and verifies it did not crash.

=head2 alien_checkpoint_ok

 alien_checkpoint_ok;
 alien_checkpoint_ok $test_name;

Test the checkpoint of a build.

=head2 alien_resume_ok

 alien_resume_ok;
 alien_resume_ok $test_name;

Test a resume a checkpointed build.

=head2 alien_rc

 alien_rc $code;

Creates C<rc.pl> file in a temp directory and sets ALIEN_BUILD_RC.  Useful for testing
plugins that should be called from C<~/.alienbuild/rc.pl>.  Note that because of the
nature of how the C<~/.alienbuild/rc.pl> file works, you can only use this once!

=head2 alien_subtest

 alien_subtest $test_name => sub {
   ...
 };

Clear the build object and clear the build object before and after the subtest.

=head1 SEE ALSO

=over 4

=item L<Alien>

=item L<alienfile>

=item L<Alien::Build>

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
