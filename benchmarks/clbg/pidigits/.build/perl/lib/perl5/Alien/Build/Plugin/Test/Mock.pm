package Alien::Build::Plugin::Test::Mock;

use strict;
use warnings;
use 5.008004;
use Alien::Build::Plugin;
use Carp ();
use Path::Tiny ();
use File::chdir;

# ABSTRACT: Mock plugin for testing
our $VERSION = '2.88'; # VERSION


has 'probe';


has 'download';


has 'extract';


has 'build';


has 'gather';


has check_digest => 1;

sub init
{
  my($self, $meta) = @_;

  if(my $probe = $self->probe)
  {
    if($probe =~ /^(share|system)$/)
    {
      $meta->register_hook(
        probe => sub {
          $probe;
        },
      );
    }
    elsif($probe eq 'die')
    {
      $meta->register_hook(
        probe => sub {
          die "fail";
        },
      );
    }
    else
    {
      Carp::croak("usage: plugin 'Test::Mock' => ( probe => $probe ); where $probe is one of share, system or die");
    }
  }

  if(my $download = $self->download)
  {
    $download = { 'foo-1.00.tar.gz' => _tarball() } unless ref $download eq 'HASH';
    $meta->register_hook(
      download => sub {
        my($build) = @_;
        _fs($build, $download, 1);
      },
    );
  }

  if(my $extract = $self->extract)
  {
    $extract = {
      'foo-1.00' => {
        'configure' => _tarball_configure(),
        'foo.c'     => _tarball_foo_c(),
      },
    } unless ref $extract eq 'HASH';
    $meta->register_hook(
      extract => sub {
        my($build) = @_;
        _fs($build, $extract);
      },
    );
  }

  if(my $build = $self->build)
  {
    $build = [
      {
        'foo.o',   => _build_foo_o(),
        'libfoo.a' => _build_libfoo_a(),
      },
      {
        'lib' => {
          'libfoo.a' => _build_libfoo_a(),
          'pkgconfig' => {
            'foo.pc' => sub {
              my($build) = @_;
              "prefix=$CWD\n" .
              "exec_prefix=\${prefix}\n" .
              "libdir=\${prefix}/lib\n" .
              "includedir=\${prefix}/include\n" .
              "\n" .
              "Name: libfoo\n" .
              "Description: libfoo\n" .
              "Version: 1.0.0\n" .
              "Cflags: -I\${includedir}\n" .
              "Libs: -L\${libdir} -lfoo\n";
            },
          },
        },
      },
    ] unless ref $build eq 'ARRAY';

    my($build_dir, $install_dir) = @$build;

    $meta->register_hook(
      build => sub {
        my($build) = @_;
        _fs($build, $build_dir);
        local $CWD = $build->install_prop->{prefix};
        _fs($build, $install_dir);
      },
    );
  }

  if(my $gather = $self->gather)
  {
    $meta->register_hook(
      $_ => sub {
        my($build) = @_;
        if(ref $gather eq 'HASH')
        {
          foreach my $key (keys %$gather)
          {
            $build->runtime_prop->{$key} = $gather->{$key};
          }
        }
        else
        {
          my $prefix = $build->runtime_prop->{prefix};
          $build->runtime_prop->{cflags} = "-I$prefix/include";
          $build->runtime_prop->{libs}   = "-L$prefix/lib -lfoo";
        }
      },
    ) for qw( gather_share gather_system );
  }

  if(my $cd = $self->check_digest)
  {
    $meta->register_hook(
      check_digest => ref($cd) eq 'CODE' ? $cd : sub {
        my($build, $file, $algorithm, $digest) = @_;
        if($algorithm ne 'FOO92')
        {
          return 'FAKE';
        }
        if($digest eq 'deadbeaf')
        {
          return 1;
        }
        else
        {
          die "Digest FAKE does not match: got deadbeaf, expected $digest";
        }
      }
    );
    $meta->register_hook(
      check_download => sub {
        my($build) = @_;
        my $path = $build->install_prop->{download};
        if(defined $path)
        {
          $build->check_digest($path);
        }
      },
    );
  }
}

sub _fs
{
  my($build, $hash, $download) = @_;

  foreach my $key (sort keys %$hash)
  {
    my $val = $hash->{$key};
    if(ref $val eq 'HASH')
    {
      mkdir $key;
      local $CWD = $key;
      _fs($build,$val);
    }
    elsif(ref $val eq 'CODE')
    {
      my $path = Path::Tiny->new($key)->absolute;
      $path->spew_raw($val->($build));
      if($download)
      {
        $build->install_prop->{download_detail}->{"$path"}->{protocol} = 'file';
        $build->install_prop->{download_detail}->{"$path"}->{digest}   = [ FAKE => 'deadbeaf' ];
      }
    }
    elsif(defined $val)
    {
      my $path = Path::Tiny->new($key)->absolute;
      $path->spew_raw($val);
      if($download)
      {
        $build->install_prop->{download_detail}->{"$path"}->{protocol} = 'file';
        $build->install_prop->{download_detail}->{"$path"}->{digest}   = [ FAKE => 'deadbeaf' ];
      }
    }
  }
}

sub _tarball
{
  return unpack 'u', <<'EOF';
M'XL(`+DM@5@``^V4P4K$,!"&>YZGF-V]J*SM9#=)#RN^B'BHV;0)U`32U(OX
M[D;0*LJREZVRF.\R?TA@)OS\TWI_S4JBJI@/(JJ%P%19+>AKG4"V)4Z;C922
M(;T=6(%BQIDFQB$V(8WB^]X.W>%WQ^[?_S'5,Z']\%]YU]IN#/KT/8[ZO^6?
M_B=-C-=<%$BG'^4G_]S_U:)ZL*X:#(!6QN/26(Q&![W<P5_/EIF?*?])E&J>
M'BD/DO/#^6<DON__6O*<_]]@99WJQ[W&FR'NK2_-+8!U$1X;ZRZ2P"9T:HW*
D-`&ODGZZN[^$9T`,.H[!(>W@)2^*3":3.3]>`:%LBYL`#@``
`
EOF
}

sub _tarball_configure
{
  return unpack 'u', <<'EOF';
<(R$O8FEN+W-H"@IE8VAO(")H:2!T:&5R92(["@``
`
EOF
}

sub _tarball_foo_c
{
  return unpack 'u', <<'EOF';
M(VEN8VQU9&4@/'-T9&EO+F@^"@II;G0*;6%I;BAI;G0@87)G8RP@8VAA<B`J
887)G=EM=*0I["B`@<F5T=7)N(#`["GT*
`
EOF
}

sub _build_foo_o
{
  return unpack 'u', <<'EOF';
MS_KM_@<```$#`````0````0```"P`0```"`````````9````.`$`````````
M`````````````````````````&@`````````T`$```````!H``````````<`
M```'`````P````````!?7W1E>'0`````````````7U]415A4````````````
M````````````"`````````#0`0``!`````````````````0`@```````````
M`````%]?8V]M<&%C=%]U;G=I;F1?7TQ$````````````````"``````````@
M`````````-@!```#````.`(```$````````"````````````````7U]E:%]F
M<F%M90```````%]?5$585``````````````H`````````$``````````^`$`
M``,```````````````L``&@````````````````D````$``````-"@``````
M`@```!@```!``@```0```%`"```(````"P```%`````````````````````!
M`````0``````````````````````````````````````````````````````
M``````````````````!52(GE,<!=PP``````````"`````````$`````````
M````````````%``````````!>E(``7@0`1`,!PB0`0``)````!P```"X____
M_____P@``````````$$.$(8"0PT&```````````````!```&`0````\!````
/``````````!?;6%I;@``
`
EOF
}

sub _build_libfoo_a
{
  return unpack 'u', <<'EOF';
M(3QA<F-H/@HC,2\R,"`@("`@("`@("`@,34S,S$U-38Q."`@-3`Q("`@,C`@
M("`@,3`P-C0T("`T-"`@("`@("`@8`I?7RY364U$148@4T]25$5$``````@`
M````````<`````@```!?;6%I;@```",Q+S$R("`@("`@("`@("`Q-3,S,34U
M-#8X("`U,#$@("`R,"`@("`Q,#`V-#0@(#8Q,B`@("`@("!@"F9O;RYO````
M`````,_Z[?X'```!`P````$````$````L`$````@````````&0```#@!````
M``````````````````````````````!H`````````-`!````````:```````
M```'````!P````,`````````7U]T97AT`````````````%]?5$585```````
M``````````````````@`````````T`$```0````````````````$`(``````
M``````````!?7V-O;7!A8W1?=6YW:6YD7U],1`````````````````@`````
M````(`````````#8`0```P```#@"```!`````````@```````````````%]?
M96A?9G)A;64```````!?7U1%6%0`````````````*`````````!`````````
M`/@!```#```````````````+``!H````````````````)````!``````#0H`
M``````(````8````0`(```$```!0`@``"`````L```!0````````````````
M`````0````$`````````````````````````````````````````````````
M````````````````````````54B)Y3'`7<,```````````@````````!````
M`````````````````!0``````````7I2``%X$`$0#`<(D`$``"0````<````
MN/________\(``````````!!#A"&`D,-!@```````````````0``!@$````/
3`0``````````````7VUA:6X`````
`
EOF
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::Plugin::Test::Mock - Mock plugin for testing

=head1 VERSION

version 2.88

=head1 SYNOPSIS

 use alienfile;
 plugin 'Test::Mock' => (
   probe    => 'share',
   download => 1,
   extract  => 1,
   build    => 1,
   gather   => 1,
 );

=head1 DESCRIPTION

This plugin is used for testing L<Alien::Build> plugins.  Usually you only want to test
one or two phases in an L<alienfile> for your plugin, but you still have to have a fully
formed L<alienfile> that contains all required phases.  This plugin lets you fill in the
other phases with the appropriate hooks.  This is usually better than using real plugins
which may pull in additional dynamic requirements that you do not want to rely on at
test time.

=head1 PROPERTIES

=head2 probe

 plugin 'Test::Mock' => (
   probe => $probe,
 );

Override the probe behavior by one of the following:

=over

=item share

For a C<share> build.

=item system

For a C<system> build.

=item die

To throw an exception in the probe hook.  This will usually cause L<Alien::Build>
to try the next probe hook, if available, or to assume a C<share> install.

=back

=head2 download

 plugin 'Test::Mock' => (
   download => \%fs_spec,
 );
 
 plugin 'Test::Mock' => (
   download => 1,
 );

Mock out a download.  The C<%fs_spec> is a hash where the hash values are directories
and the string values are files.  This a spec like this:

 plugin 'Test::Mock' => (
   download => {
     'foo-1.00' => {
       'README.txt' => "something to read",
       'foo.c' => "#include <stdio.h>\n",
                  "int main() {\n",
                  "  printf(\"hello world\\n\");\n",
                  "}\n",
     }
   },
 );

Would generate two files in the directory 'foo-1.00', a C<README.txt> and a C file named C<foo.c>.
The default, if you provide a true non-hash value is to generate a single tarball with the name
C<foo-1.00.tar.gz>.

=head2 extract

 plugin 'Test::Mock' => (
   extract => \%fs_spec,
 );
 
 plugin 'Test::Mock' => (
   extract => 1,
 );

Similar to C<download> above, but for the C<extract> phase.

=head2 build

 plugin 'Test::Mock' => (
   build => [ \%fs_spec_build, \%fs_spec_install ],
 );
 
 plugin 'Test::Mock' => (
   build => 1,
 );

=head2 gather

 plugin 'Test::Mock' => (
   gather => \%runtime_prop,
 );
 
 plugin 'Test::Mock' => (
   gather => 1,
 );

This adds a gather hook (for both C<share> and C<system>) that adds the given runtime properties, or
if a true non-hash value is provided, some reasonable runtime properties for testing.

=head2 check_digest

 plugin 'Test::Mock' => (
   check_digest => 1,  # the default
 );

This adds a check_digest hook that uses fake algorithm FAKE that hashes everything to C<deadbeaf>.
The mock download above will set the digest for download_details so that this will pass the
signature check.

 plugin 'Test::Mock' => (
   check_digest => sub {
     my($build, $file, $algo, $digest) = @_;
     ...
   },
 );

If you give it a code reference then you can write your own faux digest.  See the
L<check_digest hook|Alien::Build::Manual::PluginAuthor/"check_digest hook"> in
L<Alien::Build::Manual::PluginAuthor> for details.

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
