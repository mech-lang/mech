# Zig, Odin, Roc and Elm compared with Mech

Time ratios use the fresh unchanged fastest Mech AOT control for that workload for both profiles. Fastest LOC ratios use the fastest Mech source; smallest LOC ratios use the smallest Mech source in the preserved explorer dataset. Below 1x means faster or fewer LOC than that Mech reference.

Values are full-workload medians of three fresh-process runs. LOC includes the complete application and its helpers/FFI declarations; general library implementation and the generic process I/O adapter are excluded.

Roc uses the pinned alpha4 LLVM compiler from September 2025. Elm uses the pinned optimized compiler and Node; all benchmark algorithms remain in Elm.

## Zig

| Benchmark | Profile | Seconds | Time × Mech | LOC | LOC × Mech |
|---|---|---:|---:|---:|---:|
| nbody | fastest | 1.000683 | 0.954× | 117 | 4.680× |
| nbody | smallest | 1.075134 | 1.025× | 116 | 5.800× |
| mandelbrot | fastest | 2.248791 | 5.068× | 102 | 3.517× |
| mandelbrot | smallest | 2.248791 | 5.068× | 102 | 5.100× |
| knucleotide | fastest | 1.056632 | 1.984× | 190 | 11.176× |
| knucleotide | smallest | 1.693048 | 3.179× | 183 | 10.765× |
| spectralnorm | fastest | 0.106528 | 1.183× | 113 | 9.417× |
| spectralnorm | smallest | 0.760719 | 8.448× | 76 | 6.333× |
| revcomp | fastest | 0.301399 | 2.066× | 66 | 9.429× |
| revcomp | smallest | 0.301399 | 2.066× | 66 | 9.429× |
| fannkuchredux | fastest | 3.437727 | 2.012× | 149 | 2.525× |
| fannkuchredux | smallest | 17.894304 | 10.473× | 87 | 2.231× |
| fasta | fastest | 0.763955 | 1.319× | 145 | 3.372× |
| fasta | smallest | 0.763955 | 1.319× | 145 | 4.531× |
| binarytrees | fastest | 0.557472 | 1.414× | 71 | 2.731× |
| binarytrees | smallest | 2.694176 | 6.832× | 51 | 1.962× |
| regexredux | fastest | 1.758803 | 3.576× | 102 | 4.250× |
| regexredux | smallest | 11.048065 | 22.463× | 101 | 4.208× |
| pidigits | fastest | 0.548138 | 1.909× | 66 | 1.737× |
| pidigits | smallest | 0.548138 | 1.909× | 66 | 1.886× |

## Odin

| Benchmark | Profile | Seconds | Time × Mech | LOC | LOC × Mech |
|---|---|---:|---:|---:|---:|
| nbody | fastest | 1.827126 | 1.741× | 156 | 6.240× |
| nbody | smallest | 1.843046 | 1.756× | 110 | 5.500× |
| mandelbrot | fastest | 2.212999 | 4.987× | 65 | 2.241× |
| mandelbrot | smallest | 2.212999 | 4.987× | 65 | 3.250× |
| knucleotide | fastest | 2.093316 | 3.931× | 107 | 6.294× |
| knucleotide | smallest | 8.538036 | 16.032× | 86 | 5.059× |
| spectralnorm | fastest | 0.743546 | 8.257× | 51 | 4.250× |
| spectralnorm | smallest | 0.743546 | 8.257× | 51 | 4.250× |
| revcomp | fastest | 0.384478 | 2.635× | 71 | 10.143× |
| revcomp | smallest | 0.384478 | 2.635× | 71 | 10.143× |
| fannkuchredux | fastest | 28.116074 | 16.456× | 47 | 0.797× |
| fannkuchredux | smallest | 28.116074 | 16.456× | 47 | 1.205× |
| fasta | fastest | 0.746829 | 1.289× | 64 | 1.488× |
| fasta | smallest | 2.135921 | 3.687× | 57 | 1.781× |
| binarytrees | fastest | 1.073160 | 2.721× | 75 | 2.885× |
| binarytrees | smallest | 5.571199 | 14.128× | 51 | 1.962× |
| regexredux | fastest | 1.895714 | 3.854× | 101 | 4.208× |
| regexredux | smallest | 25.701571 | 52.256× | 62 | 2.583× |
| pidigits | fastest | 0.550815 | 1.919× | 67 | 1.763× |
| pidigits | smallest | 0.550815 | 1.919× | 67 | 1.914× |

## Roc

| Benchmark | Profile | Seconds | Time × Mech | LOC | LOC × Mech |
|---|---|---:|---:|---:|---:|
| nbody | fastest | 3.068911 | 2.925× | 74 | 2.960× |
| nbody | smallest | 3.068911 | 2.925× | 74 | 3.700× |
| mandelbrot | fastest | 10.649482 | 23.999× | 28 | 0.966× |
| mandelbrot | smallest | 10.649482 | 23.999× | 28 | 1.400× |
| knucleotide | fastest | 15.823156 | 29.712× | 74 | 4.353× |
| knucleotide | smallest | 15.823156 | 29.712× | 74 | 4.353× |
| spectralnorm | fastest | 1.061362 | 11.786× | 41 | 3.417× |
| spectralnorm | smallest | 1.061362 | 11.786× | 41 | 3.417× |
| revcomp | fastest | 2.591674 | 17.763× | 44 | 6.286× |
| revcomp | smallest | 2.591674 | 17.763× | 44 | 6.286× |
| fannkuchredux | fastest | 36.945730 | 21.623× | 45 | 0.763× |
| fannkuchredux | smallest | 36.945730 | 21.623× | 45 | 1.154× |
| fasta | fastest | 3.606151 | 6.225× | 58 | 1.349× |
| fasta | smallest | 3.606151 | 6.225× | 58 | 1.812× |
| binarytrees | fastest | 15.428277 | 39.126× | 34 | 1.308× |
| binarytrees | smallest | 15.428277 | 39.126× | 34 | 1.308× |
| regexredux | fastest | 45.446718 | 92.401× | 165 | 6.875× |
| regexredux | smallest | 74.607301 | 151.690× | 161 | 6.708× |
| pidigits | fastest | 15.382316 | 53.581× | 120 | 3.158× |
| pidigits | smallest | 15.382316 | 53.581× | 120 | 3.429× |

## Elm

| Benchmark | Profile | Seconds | Time × Mech | LOC | LOC × Mech |
|---|---|---:|---:|---:|---:|
| nbody | fastest | 143.869542 | 137.113× | 75 | 3.000× |
| nbody | smallest | 143.869542 | 137.113× | 75 | 3.750× |
| mandelbrot | fastest | 12.455820 | 28.070× | 35 | 1.207× |
| mandelbrot | smallest | 12.455820 | 28.070× | 35 | 1.750× |
| knucleotide | fastest | 301.239682 | 565.652× | 59 | 3.471× |
| knucleotide | smallest | 301.239682 | 565.652× | 59 | 3.471× |
| spectralnorm | fastest | 4.080818 | 45.318× | 38 | 3.167× |
| spectralnorm | smallest | 4.080818 | 45.318× | 38 | 3.167× |
| revcomp | fastest | 40.609683 | 278.328× | 50 | 7.143× |
| revcomp | smallest | 40.609683 | 278.328× | 50 | 7.143× |
| fannkuchredux | fastest | 101.005305 | 59.116× | 88 | 1.492× |
| fannkuchredux | smallest | 302.209015 | 176.875× | 46 | 1.179× |
| fasta | fastest | 6.992985 | 12.071× | 65 | 1.512× |
| fasta | smallest | 6.992985 | 12.071× | 65 | 2.031× |
| binarytrees | fastest | 8.192095 | 20.775× | 39 | 1.500× |
| binarytrees | smallest | 8.192095 | 20.775× | 39 | 1.500× |
| regexredux | fastest | 1.420736 | 2.889× | 33 | 1.375× |
| regexredux | smallest | 1.420736 | 2.889× | 33 | 1.375× |
| pidigits | fastest | 164.362667 | 572.527× | 44 | 1.158× |
| pidigits | smallest | 164.362667 | 572.527× | 44 | 1.257× |
