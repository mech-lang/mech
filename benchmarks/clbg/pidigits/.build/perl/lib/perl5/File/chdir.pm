package File::chdir;
use 5.004;
use strict;
use vars qw($VERSION @ISA @EXPORT $CWD @CWD);
# ABSTRACT: a more sensible way to change directories

our $VERSION = '0.1011';

require Exporter;
@ISA = qw(Exporter);
@EXPORT = qw(*CWD);

use Carp;
use Cwd 3.16;
use File::Spec::Functions 3.27 qw/canonpath splitpath catpath splitdir catdir/;

tie $CWD, 'File::chdir::SCALAR' or die "Can't tie \$CWD";
tie @CWD, 'File::chdir::ARRAY'  or die "Can't tie \@CWD";

sub _abs_path {
    # Otherwise we'll never work under taint mode.
    my($cwd) = Cwd::getcwd =~ /(.*)/s;
    # Run through File::Spec, since everything else uses it
    return canonpath($cwd);
}

# splitpath but also split directory
sub _split_cwd {
    my ($vol, $dir) = splitpath(_abs_path, 1);
    my @dirs = splitdir( $dir );
    shift @dirs; # get rid of leading empty "root" directory
    return ($vol, @dirs);
}

# catpath, but take list of directories
# restore the empty root dir and provide an empty file to avoid warnings
sub _catpath {
    my ($vol, @dirs) = @_;
    return catpath($vol, catdir(q{}, @dirs), q{});
}

sub _chdir {
    # Untaint target directory
    my ($new_dir) = $_[0] =~ /(.*)/s;

    local $Carp::CarpLevel = $Carp::CarpLevel + 1;
    if ( ! CORE::chdir($new_dir) ) {
        croak "Failed to change directory to '$new_dir': $!";
    };
    return 1;
}

{
    package File::chdir::SCALAR;
    use Carp;

    BEGIN {
        *_abs_path = \&File::chdir::_abs_path;
        *_chdir = \&File::chdir::_chdir;
        *_split_cwd = \&File::chdir::_split_cwd;
        *_catpath = \&File::chdir::_catpath;
    }

    sub TIESCALAR {
        bless [], $_[0];
    }

    # To be safe, in case someone chdir'd out from under us, we always
    # check the Cwd explicitly.
    sub FETCH {
        return _abs_path;
    }

    sub STORE {
        return unless defined $_[1];
        _chdir($_[1]);
    }
}


{
    package File::chdir::ARRAY;
    use Carp;

    BEGIN {
        *_abs_path = \&File::chdir::_abs_path;
        *_chdir = \&File::chdir::_chdir;
        *_split_cwd = \&File::chdir::_split_cwd;
        *_catpath = \&File::chdir::_catpath;
    }

    sub TIEARRAY {
        bless {}, $_[0];
    }

    sub FETCH {
        my($self, $idx) = @_;
        my ($vol, @cwd) = _split_cwd;
        return $cwd[$idx];
    }

    sub STORE {
        my($self, $idx, $val) = @_;

        my ($vol, @cwd) = _split_cwd;
        if( $self->{Cleared} ) {
            @cwd = ();
            $self->{Cleared} = 0;
        }

        $cwd[$idx] = $val;
        my $dir = _catpath($vol,@cwd);

        _chdir($dir);
        return $cwd[$idx];
    }

    sub FETCHSIZE {
        my ($vol, @cwd) = _split_cwd;
        return scalar @cwd;
    }
    sub STORESIZE {}

    sub PUSH {
        my($self) = shift;

        my $dir = _catpath(_split_cwd, @_);
        _chdir($dir);
        return $self->FETCHSIZE;
    }

    sub POP {
        my($self) = shift;

        my ($vol, @cwd) = _split_cwd;
        my $popped = pop @cwd;
        my $dir = _catpath($vol,@cwd);
        _chdir($dir);
        return $popped;
    }

    sub SHIFT {
        my($self) = shift;

        my ($vol, @cwd) = _split_cwd;
        my $shifted = shift @cwd;
        my $dir = _catpath($vol,@cwd);
        _chdir($dir);
        return $shifted;
    }

    sub UNSHIFT {
        my($self) = shift;

        my ($vol, @cwd) = _split_cwd;
        my $dir = _catpath($vol, @_, @cwd);
        _chdir($dir);
        return $self->FETCHSIZE;
    }

    sub CLEAR  {
        my($self) = shift;
        $self->{Cleared} = 1;
    }

    sub SPLICE {
        my $self = shift;
        my $offset = shift || 0;
        my $len = shift || $self->FETCHSIZE - $offset;
        my @new_dirs = @_;

        my ($vol, @cwd) = _split_cwd;
        my @orig_dirs = splice @cwd, $offset, $len, @new_dirs;
        my $dir = _catpath($vol, @cwd);
        _chdir($dir);
        return @orig_dirs;
    }

    sub EXTEND { }
    sub EXISTS {
        my($self, $idx) = @_;
        return $self->FETCHSIZE >= $idx ? 1 : 0;
    }

    sub DELETE {
        my($self, $idx) = @_;
        croak "Can't delete except at the end of \@CWD"
            if $idx < $self->FETCHSIZE - 1;
        local $Carp::CarpLevel = $Carp::CarpLevel + 1;
        $self->POP;
    }
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

File::chdir - a more sensible way to change directories

=head1 VERSION

version 0.1011

=head1 SYNOPSIS

  use File::chdir;

  $CWD = "/foo/bar";     # now in /foo/bar
  {
      local $CWD = "/moo/baz";  # now in /moo/baz
      ...
  }

  # still in /foo/bar!

=head1 DESCRIPTION

Perl's C<chdir()> has the unfortunate problem of being very, very, very
global.  If any part of your program calls C<chdir()> or if any library
you use calls C<chdir()>, it changes the current working directory for
the *whole* program.

This sucks.

File::chdir gives you an alternative, C<$CWD> and C<@CWD>.  These two
variables combine all the power of C<chdir()>, L<File::Spec> and L<Cwd>.

=head1 $CWD

Use the C<$CWD> variable instead of C<chdir()> and Cwd.

    use File::chdir;
    $CWD = $dir;  # just like chdir($dir)!
    print $CWD;   # prints the current working directory

It can be localized, and it does the right thing.

    $CWD = "/foo";      # it's /foo out here.
    {
        local $CWD = "/bar";  # /bar in here
    }
    # still /foo out here!

C<$CWD> always returns the absolute path in the native form for the
operating system.

C<$CWD> and normal C<chdir()> work together just fine.

=head1 @CWD

C<@CWD> represents the current working directory as an array, each
directory in the path is an element of the array.  This can often make
the directory easier to manipulate, and you don't have to fumble with
C<< File::Spec->splitpath >> and C<< File::Spec->catdir >> to make portable code.

  # Similar to chdir("/usr/local/src/perl")
  @CWD = qw(usr local src perl);

pop, push, shift, unshift and splice all work.  pop and push are
probably the most useful.

  pop @CWD;                 # same as chdir(File::Spec->updir)
  push @CWD, 'some_dir'     # same as chdir('some_dir')

C<@CWD> and C<$CWD> both work fine together.

*NOTE* Due to a perl bug you can't localize C<@CWD>.  See L</CAVEATS> for a work around.

=head1 EXAMPLES

(We omit the C<use File::chdir> from these examples for terseness)

Here's C<$CWD> instead of C<chdir()>:

    $CWD = 'foo';           # chdir('foo')

and now instead of Cwd.

    print $CWD;             # use Cwd;  print Cwd::abs_path

you can even do zsh style C<cd foo bar>

    $CWD = '/usr/local/foo';
    $CWD =~ s/usr/var/;

if you want to localize that, make sure you get the parens right

    {
        (local $CWD) =~ s/usr/var/;
        ...
    }

It's most useful for writing polite subroutines which don't leave the
program in some strange directory:

    sub foo {
        local $CWD = 'some/other/dir';
        ...do your work...
    }

which is much simpler than the equivalent:

    sub foo {
        use Cwd;
        my $orig_dir = Cwd::getcwd;
        chdir('some/other/dir');

        ...do your work...

        chdir($orig_dir);
    }

C<@CWD> comes in handy when you want to start moving up and down the
directory hierarchy in a cross-platform manner without having to use
File::Spec.

    pop @CWD;                   # chdir(File::Spec->updir);
    push @CWD, 'some', 'dir'    # chdir(File::Spec->catdir(qw(some dir)));

You can easily change your parent directory:

    # chdir from /some/dir/bar/moo to /some/dir/foo/moo
    $CWD[-2] = 'foo';

=head1 CAVEATS

=head2 C<local @CWD> does not work.

C<local @CWD> will not localize C<@CWD>.  This is a bug in Perl, you
can't localize tied arrays.  As a work around localizing $CWD will
effectively localize @CWD.

    {
        local $CWD;
        pop @CWD;
        ...
    }

=head2 Assigning to C<@CWD> calls C<chdir()> for each element

    @CWD = qw/a b c d/;

Internally, Perl clears C<@CWD> and assigns each element in turn.  Thus, this
code above will do this:

    chdir 'a';
    chdir 'a/b';
    chdir 'a/b/c';
    chdir 'a/b/c/d';

Generally, avoid assigning to C<@CWD> and just use push and pop instead.

=head2 Volumes not handled

There is currently no way to change the current volume via File::chdir.

=head1 NOTES

C<$CWD> returns the current directory using native path separators, i.e. C<\>
on Win32.  This ensures that C<$CWD> will compare correctly with directories
created using File::Spec.  For example:

    my $working_dir = File::Spec->catdir( $CWD, "foo" );
    $CWD = $working_dir;
    doing_stuff_might_chdir();
    is( $CWD, $working_dir, "back to original working_dir?" );

Deleting the last item of C<@CWD> will act like a pop.  Deleting from the
middle will throw an exception.

    delete @CWD[-1]; # OK
    delete @CWD[-2]; # Dies

What should %CWD do?  Something with volumes?

    # chdir to C:\Program Files\Sierra\Half Life ?
    $CWD{C} = '\\Program Files\\Sierra\\Half Life';

=head1 DIAGNOSTICS

If an error is encountered when changing C<$CWD> or C<@CWD>, one of
the following exceptions will be thrown:

* ~Can't delete except at the end of @CWD~
* ~Failed to change directory to '$dir'~

=head1 HISTORY

Michael wanted C<local chdir> to work.  p5p didn't.  But it wasn't over!
Was it over when the Germans bombed Pearl Harbor?  Hell, no!

Abigail and/or Bryan Warnock suggested the C<$CWD> thing (Michael forgets
which).  They were right.

The C<chdir()> override was eliminated in 0.04.

David became co-maintainer with 0.06_01 to fix some chronic
Win32 path bugs.

As of 0.08, if changing C<$CWD> or C<@CWD> fails to change the directory, an
error will be thrown.

=head1 SEE ALSO

L<File::pushd>, L<File::Spec>, L<Cwd>, L<perlfunc/chdir>,
"Animal House" L<http://www.imdb.com/title/tt0077975/quotes>

=for :stopwords cpan testmatrix url annocpan anno bugtracker rt cpants kwalitee diff irc mailto metadata placeholders metacpan

=head1 SUPPORT

=head2 Bugs / Feature Requests

Please report any bugs or feature requests through the issue tracker
at L<https://github.com/dagolden/File-chdir/issues>.
You will be notified automatically of any progress on your issue.

=head2 Source Code

This is open source software.  The code repository is available for
public review and contribution under the terms of the license.

L<https://github.com/dagolden/File-chdir>

  git clone https://github.com/dagolden/File-chdir.git

=head1 AUTHORS

=over 4

=item *

David Golden <dagolden@cpan.org>

=item *

Michael G. Schwern <schwern@pobox.com>

=back

=head1 CONTRIBUTORS

=for stopwords David Golden Joel Berger Philippe Bruhat (BooK)

=over 4

=item *

David Golden <xdg@xdg.me>

=item *

Joel Berger <joel.a.berger@gmail.com>

=item *

Philippe Bruhat (BooK) <book@cpan.org>

=back

=head1 COPYRIGHT AND LICENSE

This software is copyright (c) 2016 by Michael G. Schwern and David Golden.

This is free software; you can redistribute it and/or modify it under
the same terms as the Perl 5 programming language system itself.

=cut
