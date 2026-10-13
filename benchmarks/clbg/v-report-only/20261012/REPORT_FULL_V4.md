All ten V workloads independently reconciled from three actual full samples per source and matched Mech controls (90 processes). No extrapolation or source/timing relabeling. The report remains excluded from the benchmark dataset, charts and published Site.

The faster V form is selected per workload. **1× is matched Mech; larger means slower elapsed time.**

| Workload | V form | V / Mech |
|---|---|---:|
| Regex redux | optimized | 49.819× |
| Reverse complement | optimized | 9.376× |
| Pi digits | optimized | 8.064× |
| Spectral norm | optimized | 1.324× |
| Mandelbrot | optimized | 1.506× |
| FASTA | optimized | 3.797× |
| K-nucleotide | optimized | 6.605× |
| Binary trees | compact | 12.131× |
| N-body | optimized | 1.520× |
| Fannkuch redux | optimized | 2.606× |

Evidence: [all-ten reconciliation](reconciliation/all-ten-full-samples-v4.json). Original manifests, saved native/sampler ownership, full output, power/resource observations, source/binary pins and all three samples are bound by the independent checker. The missing eight tree slots were appended in a separate continuation; its successful original first sample remains unchanged. These results describe the qualified implementations and build options in this suite. They do not establish a general bound on V performance or prove the source of a performance gap.
