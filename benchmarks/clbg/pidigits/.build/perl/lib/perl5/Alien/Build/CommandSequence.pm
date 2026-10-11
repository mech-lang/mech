package Alien::Build::CommandSequence;

use strict;
use warnings;
use 5.008004;
use Text::ParseWords qw( shellwords );
use Capture::Tiny qw( capture );

# ABSTRACT: Alien::Build command sequence
our $VERSION = '2.88'; # VERSION


sub new
{
  my($class, @commands) = @_;
  my $self = bless {
    commands => \@commands,
  }, $class;
  $self;
}


sub apply_requirements
{
  my($self, $meta, $phase) = @_;
  my $intr = $meta->interpolator;
  foreach my $command (@{ $self->{commands} })
  {
    next if ref $command eq 'CODE';
    if(ref $command eq 'ARRAY')
    {
      foreach my $arg (@$command)
      {
        next if ref $arg eq 'CODE';
        $meta->add_requires($phase, $intr->requires($arg))
      }
    }
    else
    {
      $meta->add_requires($phase, $intr->requires($command));
    }
  }
  $self;
}

my %built_in = (

  cd => sub {
    my(undef, $dir) = @_;
    if(!defined $dir)
    {
      die "undef passed to cd";
    }
    elsif(-d $dir)
    {
      chdir($dir) || die "unable to cd $dir $!";
    }
    else
    {
      die "unable to cd $dir, does not exist";
    }
  },

);

sub _run_list
{
  my($build, @cmd) = @_;
  $build->log("+ @cmd");
  return $built_in{$cmd[0]}->(@cmd) if $built_in{$cmd[0]};
  system @cmd;
  die "external command failed" if $?;
}

sub _run_string
{
  my($build, $cmd) = @_;
  $build->log("+ $cmd");

  {
    my $cmd = $cmd;
    $cmd =~ s{\\}{\\\\}g if $^O eq 'MSWin32';
    my @cmd = shellwords($cmd);
    return $built_in{$cmd[0]}->(@cmd) if $built_in{$cmd[0]};
  }

  system $cmd;
  die "external command failed" if $?;
}

sub _run_with_code
{
  my($build, @cmd) = @_;
  my $code = pop @cmd;
  $build->log("+ @cmd");
  my %args = ( command => \@cmd );

  if($built_in{$cmd[0]})
  {
    my $error;
    ($args{out}, $args{err}, $error) = capture {
      eval { $built_in{$cmd[0]}->(@cmd) };
      $@;
    };
    $args{exit} = $error eq '' ? 0 : 2;
    $args{builtin} = 1;
  }
  else
  {
    ($args{out}, $args{err}, $args{exit}) = capture {
      system @cmd; $?
    };
  }
  $build->log("[output consumed by Alien::Build recipe]");
  $code->($build, \%args);
}


sub _apply
{
  my($where, $prop, $value) = @_;
  if($where =~ /^(.*?)\.(.*?)$/)
  {
    _apply($2, $prop->{$1}, $value);
  }
  else
  {
    $prop->{$where} = $value;
  }
}

sub execute
{
  my($self, $build) = @_;
  my $intr = $build->meta->interpolator;

  foreach my $command (@{ $self->{commands} })
  {
    if(ref($command) eq 'CODE')
    {
      $command->($build);
    }
    elsif(ref($command) eq 'ARRAY')
    {
      my($command, @args) = @$command;
      my $code;
      $code = pop @args if $args[-1] && ref($args[-1]) eq 'CODE';

      if($args[-1] && ref($args[-1]) eq 'SCALAR')
      {
        my $dest = ${ pop @args };
        if($dest =~ /^\%\{((?:alien|)\.(?:install|runtime|hook)\.[a-z\.0-9_]+)\}$/)
        {
          $dest = $1;
          $dest =~ s/^\./alien./;
          $code = sub {
            my($build, $args) = @_;
            die "external command failed" if $args->{exit};
            my $out = $args->{out};
            chomp $out;
            _apply($dest, $build->_command_prop, $out);
          };
        }
        else
        {
          die "illegal destination: $dest";
        }
      }

      ($command, @args) = map { $intr->interpolate($_, $build) } ($command, @args);

      if($code)
      {
        _run_with_code $build, $command, @args, $code;
      }
      else
      {
        _run_list $build, $command, @args;
      }
    }
    else
    {
      my $command = $intr->interpolate($command, $build);
      _run_string $build, $command;
    }
  }
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build::CommandSequence - Alien::Build command sequence

=head1 VERSION

version 2.88

=head1 CONSTRUCTOR

=head2 new

 my $seq = Alien::Build::CommandSequence->new(@commands);

=head1 METHODS

=head2 apply_requirements

 $seq->apply_requirements($meta, $phase);

=head2 execute

 $seq->execute($build);

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
