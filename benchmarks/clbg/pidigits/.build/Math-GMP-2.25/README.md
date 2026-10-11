# NAME

Math::GMP - High speed arbitrary size integer math

# SYNOPSIS

```perl
use Math::GMP;
my $n = Math::GMP->new('2');

$n = $n ** (256*1024);
$n = $n - 1;
print "n is now $n\n";
```

# DESCRIPTION

Math::GMP gives you access to the fast GMP library for fast big integer math.

# AUTHOR

Chip Turner <chip@redhat.com>, based on Math::BigInt by Mark Biggar
and Ilya Zakharevich.  Further extensive work provided by
Tels <tels@bloodgate.com>. Later non-extensive work by Greg Sabino Mullane.
Later work by [Shlomi Fish](https://www.shlomifish.org/) while putting his
changes under CC0.

# DEVELOPMENT

* [GitHub Repository](https://github.com/turnstep/Math-GMP)
* [rt.cpan](https://rt.cpan.org/Dist/Display.html?Name=Math-GMP)

See [dzil / Dist-Zilla](http://dzil.org/) (also https://metacpan.org/pod/Dist::Zilla ) for how to build from the `dist.ini`-based repository sources.
