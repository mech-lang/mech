# Crystal implementations for The Computer Language Benchmarks Game

Compared to C, and Ruby implementations from http://benchmarksgame.alioth.debian.org/ .

Running on AMD Ryzen 7 3800X, 16Gb DDR-4 3200 Mghz, Gentoo Linux x86_64.

My other benchmarks: [Benchmarks](https://github.com/kostya/benchmarks), [Jit Benchmarks](https://github.com/kostya/jit-benchmarks).

To run and build:
```
cd DIR
sh build.sh 
sh run.sh
```

### Binarytrees

| Language        | Time,s  | Memory, Mb |
| --------------- | ------- | ---------- |
| Java            | 0.46    | 296.5      |
| C++             | 1.16    | 29.4       |
| Crystal         | 1.35    | 49.5       |
| C               | 2.02    | 33.4       |
| Ruby            | 15.15   | 106.5      |

### Fannkuchredux

| Language        | Time,s  | Memory, Mb |
| --------------- | ------- | ---------- |
| C               | 2.55    | 0.7        |
| Crystal         | 2.56    | 3.9        |
| Ruby            | 75.32   | 9.0        |

### Fasta

| Language        | Time,s  | Memory, Mb |
| --------------- | ------- | ---------- |
| C               | 1.71    | 0.7        |
| Crystal         | 2.21    | 3.8        |
| Ruby            | 57.58   | 149.0      |

### Knucleotide

| Language        | Time,s  | Memory, Mb |
| --------------- | ------- | ---------- |
| Go              | 0.15    | 6.6        |
| Crystal         | 0.87    | 33.8       |
| Ruby            | 3.93    | 38.4       |

### Mandelbrot

| Language        | Time,s  | Memory, Mb |
| --------------- | ------- | ---------- |
| Crystal         | 5.51    | 3.8        |
| C               | 5.61    | 0.7        |
| Ruby            | 243.38  | 10.2       |

### Meteor

| Language        | Time,s  | Memory, Mb |
| --------------- | ------- | ---------- |
| C               | 0.04    | 0.7        |
| Crystal         | 0.10    | 4.0        |

### NBody

| Language        | Time,s  | Memory, Mb |
| --------------- | ------- | ---------- |
| C               | 0.76    | 1.0        |
| Crystal         | 0.99    | 3.9        |
| Ruby            | 62.68   | 8.8        |

### Pidigits

| Language        | Time,s  | Memory, Mb |
| --------------- | ------- | ---------- |
| C               | 2.22    | 3.3        |
| Crystal         | 11.87   | 7.9        |
| Ruby            | 15.02   | 153.3      |

### Regexdna

| Language        | Time,s  | Memory, Mb |
| --------------- | ------- | ---------- |
| Crystal         | 0.84    | 20.7       |
| C               | 0.92    | 11.1       |
| Ruby            | 0.93    | 19.0       |

### Revcomp

| Language        | Time,s  | Memory, Mb |
| --------------- | ------- | ---------- |
| C               | 0.13    | 3.7        |
| Crystal         | 0.99    | 326.4      |
| Ruby            | 1.52    | 84.7       |

### Spectralnorm

| Language        | Time,s  | Memory, Mb |
| --------------- | ------- | ---------- |
| C               | 1.15    | 0.9        |
| Crystal         | 1.27    | 4.4        |
| Ruby            | 188.35  | 10.7       |

### Threadring

| Language        | Time,s  | Memory, Mb |
| --------------- | ------- | ---------- |
| Crystal         | 0.27    | 5.9        |
| Go              | 0.70    | 2.9        |
| Erlang          | 1.33    | 30.9       |
| Ruby            | 8.39    | 22.0       |

## TODO
* chameneosredux

## Versions
* `gcc (Gentoo 9.2.0-r2 p3) 9.2.0`
* `ruby 2.5.8p224 (2020-03-31 revision 67882) [x86_64-linux]`
* `Crystal 0.33.0 (2020-04-20) LLVM: 9.0.1`
* `go version go1.13.9 linux/amd64`
* `java: openjdk version "1.8.0_242" OpenJDK Runtime Environment (IcedTea 3.15.0) (Gentoo icedtea-3.15.0)`
* `Erlang/OTP 22 [erts-10.7] [source] [64-bit] [smp:16:16] [ds:16:16:10] [async-threads:1] [hipe]`
