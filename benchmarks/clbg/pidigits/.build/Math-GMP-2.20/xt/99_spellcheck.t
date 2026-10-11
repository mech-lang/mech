#!perl

## Spellcheck as much as we can

use 5.006;
use strict;
use warnings;
use Test::More;
select(($|=1,select(STDERR),$|=1)[1]);

my (@testfiles, @perlfiles, @textfiles, @commentfiles, $fh);

if (! $ENV{RELEASE_TESTING}) {
	plan (skip_all =>  'Test skipped unless environment variable RELEASE_TESTING is set');
}
elsif (!eval { require Text::SpellChecker; 1 }) {
	plan skip_all => 'Could not find Text::SpellChecker';
}
else {
	opendir my $dir, 't' or die qq{Could not open directory 't': $!\n};
	@testfiles = map { "t/$_" } grep { /^.+\.(t|pl)$/ } readdir $dir;
	closedir $dir or die qq{Could not closedir "$dir": $!\n};

	open my $fh, '<', 'MANIFEST' or die qq{Could not open the MANIFEST file: $!\n};
	while (<$fh>) {
		next unless /(.*\.pm)/;
		push @perlfiles, $1;
	}
	close $fh or die qq{Could not close the MANIFEST file: $!\n};

	@textfiles = qw/README Changes LICENSE/;

	@commentfiles = (@testfiles, 'Makefile.PL', @perlfiles);

	plan tests => @textfiles + @perlfiles + @commentfiles;
}

my %okword;
my $file = 'Common';
while (<DATA>) {
	if (/^## (.+):/) {
		$file = $1;
		next;
	}
	next if /^#/ or ! /\w/;
	for (split) {
		++$okword{$file}{$_};
	}
}

sub spellcheck {
	my ($desc, $text, $sfile) = @_;
	my $check = Text::SpellChecker->new(text => $text);
	my %badword;
	my $class = $sfile =~ /\.pm$/ ? 'Perl' : $sfile =~ /\.t$/ ? 'Test' : '';
	while (my $word = $check->next_word) {
		next if $okword{Common}{$word} or $okword{$sfile}{$word} or $okword{$class}{$word};
		++$badword{$word};
	}
	my $count = keys %badword;
	if (! $count) {
		pass ("Spell check passed for $desc");
		return;
	}
	fail ("Spell check failed for $desc. Bad words: $count");
	for (sort keys %badword) {
		diag "$_\n";
	}
	return;
}


## General spellchecking
for my $file (@textfiles) {
	if (!open $fh, '<', $file) {
		fail (qq{Could not find the file "$file"!});
	}
	else {
		{ local $/; $_ = <$fh>; }
		close $fh or warn qq{Could not close "$file": $!\n};
		spellcheck ($file => $_, $file);
	}
}

## Now the embedded POD
SKIP: {
	if (!eval { require Pod::Spell; 1 }) {
		my $files = @perlfiles;
		skip ('Need Pod::Spell to test the spelling of embedded POD', $files);
	}

	for my $file (@perlfiles) {
		if (! -e $file) {
			fail (qq{Could not find the file "$file"!});
		}
		my $string = qx{podspell $file};
		spellcheck ("POD from $file" => $string, $file);
	}
}

## Now the comments
SKIP: {
	if (!eval { require File::Comments; 1 }) {
		my $files = @commentfiles;
		skip ('Need File::Comments to test the spelling inside comments', $files);
	}

	my $fc = File::Comments->new();

	for my $file (@commentfiles) {
		if (! -e $file) {
			fail (qq{Could not find the file "$file"!});
		}
		my $string = $fc->comments($file);
		if (! $string) {
			fail (qq{Could not get comments from file $file});
			next;
		}
		$string = join "\n" => @$string;
		$string =~ s/=head1.+//sm;
		spellcheck ("comments from $file" => $string, $file);
	}

}


__DATA__
## These words are okay

## Common:
babcdefgh
badd
bcmp
bdiv
bfac
bgcd
Biggar
BigInt
bior
bloodgate
bmod
bmul
bsub
bxor
chipt
cpan
CPAN
cturner
Divisior
dk
des
DES
env
fac
 fibonacci
gcd
gmp
GMP
GPL
Haydon
gmppmt
Ilya
intify
jacobi
legendre
Makefile
MERCHANTABILITY
Behaviour
Probabilistically
mmod
mul
Mullane
namespace
nicholas
n'th
ok
Oxh
oxhoej
perl
perl
powm
README
redhat
Sabino
Sep
sizeinbase
Spellcheck
spellchecking
sqrt
Tels
tels
tradeoff
tstbit
ui
Ulrich
uintify
xff
xs
YAML
YAMLiciousness
yml
Zakharevich

## Changes

iglu
il
libgmp
probab
shlomif
blcm
bmodinv
bsqrt
lcm
