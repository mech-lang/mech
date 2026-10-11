# Remaining Benchmark Game languages compared with Mech

Time × Mech uses the fresh unchanged fastest Mech AOT control from the same cohort for both profiles. Fastest LOC × Mech compares fastest sources; smallest LOC × Mech compares smallest sources. Below 1× means faster or shorter than Mech.

Full prescribed workloads; medians of three fresh processes. All times are seconds. Complete application helpers/FFI count toward LOC; general library implementations are excluded.

## C# (JIT)

| Benchmark | Profile | Seconds | Time × Mech | LOC | LOC × Mech |
|---|---|---:|---:|---:|---:|
| nbody | fastest | 1.736036 | 1.645× | 165 | 6.600× |
| nbody | smallest | 1.886258 | 1.788× | 87 | 4.350× |
| mandelbrot | fastest | 0.671872 | 1.527× | 67 | 2.310× |
| mandelbrot | smallest | 50.022617 | 113.700× | 43 | 2.150× |
| knucleotide | fastest | 1.486446 | 2.847× | 168 | 9.882× |
| knucleotide | smallest | 23.890815 | 45.765× | 65 | 3.824× |
| spectralnorm | fastest | 0.321852 | 3.607× | 84 | 7.000× |
| spectralnorm | smallest | 1.019937 | 11.430× | 46 | 3.833× |
| fannkuchredux | fastest | 2.421079 | 1.420× | 159 | 2.695× |
| fannkuchredux | smallest | 23.995441 | 14.070× | 63 | 1.615× |
| fasta | fastest | 2.257991 | 3.920× | 143 | 3.326× |
| fasta | smallest | 3.447910 | 5.986× | 78 | 2.438× |
| binarytrees | fastest | 1.632295 | 3.970× | 64 | 2.462× |
| binarytrees | smallest | 1.673876 | 4.071× | 62 | 2.385× |
| regexredux | fastest | 0.697474 | 1.408× | 140 | 5.833× |
| regexredux | smallest | 0.724733 | 1.463× | 53 | 2.208× |
| pidigits | fastest | 0.323749 | 1.062× | 119 | 3.132× |
| pidigits | smallest | 4.287537 | 14.063× | 62 | 1.771× |

## C# (AOT)

| Benchmark | Profile | Seconds | Time × Mech | LOC | LOC × Mech |
|---|---|---:|---:|---:|---:|
| nbody | fastest | 1.709337 | 1.620× | 165 | 6.600× |
| nbody | smallest | 1.868802 | 1.771× | 87 | 4.350× |
| mandelbrot | fastest | 0.419311 | 0.953× | 67 | 2.310× |
| mandelbrot | smallest | 49.847046 | 113.301× | 43 | 2.150× |
| knucleotide | fastest | 1.676538 | 3.212× | 168 | 9.882× |
| knucleotide | smallest | 24.864513 | 47.630× | 65 | 3.824× |
| spectralnorm | fastest | 0.121809 | 1.365× | 84 | 7.000× |
| spectralnorm | smallest | 2.503176 | 28.051× | 46 | 3.833× |
| fannkuchredux | fastest | 2.413904 | 1.415× | 159 | 2.695× |
| fannkuchredux | smallest | 23.406499 | 13.724× | 63 | 1.615× |
| fasta | fastest | 2.562407 | 4.448× | 143 | 3.326× |
| fasta | smallest | 4.061334 | 7.050× | 78 | 2.438× |
| binarytrees | fastest | 1.504400 | 3.659× | 62 | 2.385× |
| binarytrees | smallest | 1.504400 | 3.659× | 62 | 2.385× |
| regexredux | fastest | 0.685856 | 1.384× | 140 | 5.833× |
| regexredux | smallest | 1.213399 | 2.449× | 53 | 2.208× |
| pidigits | fastest | 0.305775 | 1.003× | 119 | 3.132× |
| pidigits | smallest | 4.121555 | 13.519× | 62 | 1.771× |

## F# (JIT)

| Benchmark | Profile | Seconds | Time × Mech | LOC | LOC × Mech |
|---|---|---:|---:|---:|---:|
| nbody | fastest | 1.864224 | 1.767× | 111 | 4.440× |
| nbody | smallest | 2.555566 | 2.422× | 99 | 4.950× |
| mandelbrot | fastest | 1.295833 | 2.945× | 63 | 2.172× |
| mandelbrot | smallest | 1.443724 | 3.282× | 57 | 2.850× |
| knucleotide | fastest | 2.862068 | 5.483× | 43 | 2.529× |
| knucleotide | smallest | 21.632734 | 41.439× | 26 | 1.529× |
| spectralnorm | fastest | 0.209788 | 2.351× | 46 | 3.833× |
| spectralnorm | smallest | 0.209788 | 2.351× | 46 | 3.833× |
| fannkuchredux | fastest | 2.773684 | 1.626× | 97 | 1.644× |
| fannkuchredux | smallest | 40.984096 | 24.031× | 74 | 1.897× |
| fasta | fastest | 0.755072 | 1.311× | 93 | 2.163× |
| fasta | smallest | 0.755072 | 1.311× | 93 | 2.906× |
| binarytrees | fastest | 1.814269 | 4.413× | 28 | 1.077× |
| binarytrees | smallest | 1.814269 | 4.413× | 28 | 1.077× |
| regexredux | fastest | 0.714430 | 1.442× | 50 | 2.083× |
| regexredux | smallest | 2.572620 | 5.192× | 24 | 1.000× |
| pidigits | fastest | 0.325947 | 1.069× | 80 | 2.105× |
| pidigits | smallest | 0.325947 | 1.069× | 80 | 2.286× |

## F# (AOT)

| Benchmark | Profile | Seconds | Time × Mech | LOC | LOC × Mech |
|---|---|---:|---:|---:|---:|
| nbody | fastest | 1.826110 | 1.731× | 111 | 4.440× |
| nbody | smallest | 2.484898 | 2.355× | 99 | 4.950× |
| mandelbrot | fastest | 1.181820 | 2.686× | 63 | 2.172× |
| mandelbrot | smallest | 1.277560 | 2.904× | 57 | 2.850× |
| knucleotide | fastest | 2.530281 | 4.847× | 43 | 2.529× |
| knucleotide | smallest | 23.352800 | 44.734× | 26 | 1.529× |
| spectralnorm | fastest | 0.178409 | 1.999× | 46 | 3.833× |
| spectralnorm | smallest | 0.178409 | 1.999× | 46 | 3.833× |
| fannkuchredux | fastest | 2.708165 | 1.588× | 97 | 1.644× |
| fannkuchredux | smallest | 38.535863 | 22.595× | 74 | 1.897× |
| fasta | fastest | 0.735919 | 1.278× | 93 | 2.163× |
| fasta | smallest | 0.735919 | 1.278× | 93 | 2.906× |
| binarytrees | fastest | 1.715437 | 4.172× | 28 | 1.077× |
| binarytrees | smallest | 1.715437 | 4.172× | 28 | 1.077× |
| regexredux | fastest | 1.206478 | 2.435× | 50 | 2.083× |
| regexredux | smallest | 5.104968 | 10.304× | 24 | 1.000× |
| pidigits | fastest | 0.309286 | 1.014× | 80 | 2.105× |
| pidigits | smallest | 0.309286 | 1.014× | 80 | 2.286× |

## Ada

| Benchmark | Profile | Seconds | Time × Mech | LOC | LOC × Mech |
|---|---|---:|---:|---:|---:|
| nbody | fastest | 1.545714 | 1.465× | 154 | 6.160× |
| nbody | smallest | 1.842307 | 1.746× | 143 | 7.150× |
| mandelbrot | fastest | 1.148045 | 2.609× | 173 | 5.966× |
| mandelbrot | smallest | 10.081947 | 22.916× | 53 | 2.650× |
| knucleotide | fastest | 1.623197 | 3.109× | 162 | 9.529× |
| knucleotide | smallest | 5.986411 | 11.468× | 130 | 7.647× |
| spectralnorm | fastest | 0.357817 | 4.010× | 221 | 18.417× |
| spectralnorm | smallest | 1.265849 | 14.186× | 69 | 5.750× |
| fannkuchredux | fastest | 2.999494 | 1.759× | 243 | 4.119× |
| fannkuchredux | smallest | 2.999494 | 1.759× | 243 | 6.231× |
| fasta | fastest | 2.149485 | 3.731× | 200 | 4.651× |
| fasta | smallest | 7.671455 | 13.318× | 121 | 3.781× |
| binarytrees | fastest | 1.040173 | 2.530× | 259 | 9.962× |
| binarytrees | smallest | 21.419847 | 52.099× | 99 | 3.808× |
| pidigits | fastest | 0.310870 | 1.020× | 112 | 2.947× |
| pidigits | smallest | 0.310870 | 1.020× | 112 | 3.200× |

## Pascal

| Benchmark | Profile | Seconds | Time × Mech | LOC | LOC × Mech |
|---|---|---:|---:|---:|---:|
| nbody | fastest | 2.183910 | 2.070× | 125 | 5.000× |
| nbody | smallest | 2.183910 | 2.070× | 125 | 6.250× |
| mandelbrot | fastest | 1.828544 | 4.156× | 103 | 3.552× |
| mandelbrot | smallest | 17.186955 | 39.066× | 60 | 3.000× |
| knucleotide | fastest | 7.545197 | 14.454× | 116 | 6.824× |
| knucleotide | smallest | 40.187724 | 76.983× | 81 | 4.765× |
| spectralnorm | fastest | 0.449656 | 5.039× | 70 | 5.833× |
| spectralnorm | smallest | 0.961560 | 10.776× | 49 | 4.083× |
| fannkuchredux | fastest | 5.547926 | 3.253× | 159 | 2.695× |
| fannkuchredux | smallest | 5.547926 | 3.253× | 159 | 4.077× |
| fasta | fastest | 2.841302 | 4.932× | 119 | 2.767× |
| fasta | smallest | 2.936542 | 5.098× | 113 | 3.531× |
| binarytrees | fastest | 0.793320 | 1.930× | 83 | 3.192× |
| binarytrees | smallest | 10.107074 | 24.583× | 53 | 2.038× |
| regexredux | fastest | 11.364769 | 22.938× | 103 | 4.292× |
| regexredux | smallest | 11.364769 | 22.938× | 103 | 4.292× |
| pidigits | fastest | 0.294968 | 0.968× | 55 | 1.447× |
| pidigits | smallest | 0.294968 | 0.968× | 55 | 1.571× |

## Fortran

| Benchmark | Profile | Seconds | Time × Mech | LOC | LOC × Mech |
|---|---|---:|---:|---:|---:|
| nbody | fastest | 1.759346 | 1.667× | 117 | 4.680× |
| nbody | smallest | 1.759346 | 1.667× | 117 | 5.850× |
| mandelbrot | fastest | 1.118857 | 2.543× | 62 | 2.138× |
| mandelbrot | smallest | 15.277667 | 34.726× | 51 | 2.550× |
| knucleotide | fastest | 7.038603 | 13.483× | 168 | 9.882× |
| knucleotide | smallest | 12.445623 | 23.841× | 160 | 9.412× |
| spectralnorm | fastest | 0.203598 | 2.282× | 81 | 6.750× |
| spectralnorm | smallest | 0.560304 | 6.279× | 55 | 4.583× |
| fannkuchredux | fastest | 6.952576 | 4.077× | 122 | 2.068× |
| fannkuchredux | smallest | 25.117056 | 14.727× | 63 | 1.615× |
| fasta | fastest | 1.971298 | 3.422× | 117 | 2.721× |
| fasta | smallest | 1.971298 | 3.422× | 117 | 3.656× |
| binarytrees | fastest | 0.446175 | 1.085× | 124 | 4.769× |
| binarytrees | smallest | 10.212154 | 24.839× | 69 | 2.654× |
| regexredux | fastest | 0.534004 | 1.078× | 237 | 9.875× |
| regexredux | smallest | 0.534004 | 1.078× | 237 | 9.875× |
| pidigits | fastest | 0.300481 | 0.986× | 121 | 3.184× |
| pidigits | smallest | 0.300481 | 0.986× | 121 | 3.457× |

## Dart (AOT)

| Benchmark | Profile | Seconds | Time × Mech | LOC | LOC × Mech |
|---|---|---:|---:|---:|---:|
| nbody | fastest | 1.865314 | 1.768× | 136 | 5.440× |
| nbody | smallest | 2.089876 | 1.981× | 114 | 5.700× |
| mandelbrot | fastest | 2.158383 | 4.906× | 104 | 3.586× |
| mandelbrot | smallest | 111.544949 | 253.539× | 41 | 2.050× |
| knucleotide | fastest | 4.181071 | 8.009× | 143 | 8.412× |
| knucleotide | smallest | 444.701349 | 851.865× | 53 | 3.118× |
| spectralnorm | fastest | 0.199915 | 2.240× | 133 | 11.083× |
| spectralnorm | smallest | 1.090063 | 12.216× | 43 | 3.583× |
| fannkuchredux | fastest | 3.826696 | 2.244× | 161 | 2.729× |
| fannkuchredux | smallest | 29.207710 | 17.126× | 60 | 1.538× |
| fasta | fastest | 2.849379 | 4.946× | 153 | 3.558× |
| fasta | smallest | 21.876427 | 37.977× | 71 | 2.219× |
| binarytrees | fastest | 5.636898 | 13.711× | 36 | 1.385× |
| binarytrees | smallest | 5.636898 | 13.711× | 36 | 1.385× |
| regexredux | fastest | 7.284314 | 14.702× | 59 | 2.458× |
| regexredux | smallest | 10.276477 | 20.741× | 39 | 1.625× |
| pidigits | fastest | 1.122172 | 3.681× | 53 | 1.395× |
| pidigits | smallest | 1.122172 | 3.681× | 53 | 1.514× |

## Dart (JIT)

| Benchmark | Profile | Seconds | Time × Mech | LOC | LOC × Mech |
|---|---|---:|---:|---:|---:|
| nbody | fastest | 1.895362 | 1.796× | 136 | 5.440× |
| nbody | smallest | 2.105891 | 1.996× | 114 | 5.700× |
| mandelbrot | fastest | 2.162141 | 4.914× | 104 | 3.586× |
| mandelbrot | smallest | 113.075577 | 257.018× | 41 | 2.050× |
| knucleotide | fastest | 4.846459 | 9.284× | 143 | 8.412× |
| knucleotide | smallest | 454.047835 | 869.769× | 53 | 3.118× |
| spectralnorm | fastest | 0.390728 | 4.379× | 133 | 11.083× |
| spectralnorm | smallest | 1.158020 | 12.977× | 43 | 3.583× |
| fannkuchredux | fastest | 3.671098 | 2.153× | 161 | 2.729× |
| fannkuchredux | smallest | 29.681027 | 17.403× | 60 | 1.538× |
| fasta | fastest | 2.027756 | 3.520× | 153 | 3.558× |
| fasta | smallest | 21.612607 | 37.519× | 71 | 2.219× |
| binarytrees | fastest | 5.204353 | 12.659× | 36 | 1.385× |
| binarytrees | smallest | 5.204353 | 12.659× | 36 | 1.385× |
| regexredux | fastest | 7.213419 | 14.559× | 59 | 2.458× |
| regexredux | smallest | 10.284546 | 20.758× | 39 | 1.625× |
| pidigits | fastest | 1.160047 | 3.805× | 53 | 1.395× |
| pidigits | smallest | 1.160047 | 3.805× | 53 | 1.514× |

## Smalltalk (Pharo)

| Benchmark | Profile | Seconds | Time × Mech | LOC | LOC × Mech |
|---|---|---:|---:|---:|---:|
| nbody | fastest | 60.720840 | 57.545× | 174 | 6.960× |
| nbody | smallest | 99.769931 | 94.552× | 65 | 3.250× |
| mandelbrot | fastest | 167.260208 | 380.179× | 53 | 1.828× |
| mandelbrot | smallest | 167.260208 | 380.179× | 53 | 2.650× |
| knucleotide | fastest | 87.903662 | 168.387× | 70 | 4.118× |
| knucleotide | smallest | 87.903662 | 168.387× | 70 | 4.118× |
| spectralnorm | fastest | 12.880128 | 144.339× | 36 | 3.000× |
| spectralnorm | smallest | 12.880128 | 144.339× | 36 | 3.000× |
| fannkuchredux | fastest | 131.003359 | 76.813× | 50 | 0.847× |
| fannkuchredux | smallest | 131.003359 | 76.813× | 50 | 1.282× |
| fasta | fastest | 9.785650 | 16.988× | 69 | 1.605× |
| fasta | smallest | 9.785650 | 16.988× | 69 | 2.156× |
| binarytrees | fastest | 17.087356 | 41.561× | 42 | 1.615× |
| binarytrees | smallest | 17.087356 | 41.561× | 42 | 1.615× |
| regexredux | fastest | 2.861119 | 5.775× | 143 | 5.958× |
| regexredux | smallest | 524.227987 | 1058.070× | 51 | 2.125× |
| pidigits | fastest | 3.930609 | 12.892× | 75 | 1.974× |
| pidigits | smallest | 4.226747 | 13.864× | 35 | 1.000× |

Groups awaiting complete samples are omitted; no estimated or scaled timings are shown.

## Excluded full-workload failures

- regexredux:regexredux-ada-pcre2: Command '['/Users/cim310/.codex/worktrees/clbg-nbody-v04/mech/benchmarks/clbg/remaining-languages-comparison/20261011/support/ada-pascal/builds/regexredux-ada-pcre2/regexredux-ada-pcre2']' returned non-zero exit status 1.; 
raised STORAGE_ERROR : stack overflow

