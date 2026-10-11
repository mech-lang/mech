# BQN and K compared with Mech

Median runtime divided by the fastest measured Mech AOT runtime. All ten full workloads; five n-body samples and three samples per other program.

| Benchmark | BQN / Mech | K / Mech |
|---|---:|---:|
| nbody | 17.9× | 22.1× |
| mandelbrot | 60.6× | 78.2× |
| knucleotide | 22.8× | 43.5× |
| spectralnorm | 24.5× | 31.1× |
| revcomp | 6.8× | 7.2× |
| fannkuchredux | 95.3× | 131.1× |
| fasta | 18.8× | 28.9× |
| binarytrees | 75.1× | 98.3× |
| regexredux | 5.0× | 4.9× |
| pidigits | 1.4× | 1.4× |

Equal-weight geometric means: BQN 18.16×; K 23.02×.

All full outputs and runtime/source pins passed. No current array result beats the leading measured native-language result; earlier required-work shortcuts stay excluded.
