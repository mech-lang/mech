# Current final-cleanup samples

Three complete process captures from compiler source `caa8b22ac5cd4d7c3cf30b5c97f78fd786fe5c16` contain 223 final-cleanup samples. Route searches account for 95 (42.6%), verification 50 (22.4%), and graph construction 21 (9.4%). These samples locate work; matched builds supply the compilation-time measurements.

All three program artifacts match the qualified 64,444-byte worker in every capture. The initial two-second captures ended before final cleanup. They are preserved under `initial-window-excluded` and are not used in the attribution. The later captures sample until process exit, up to four seconds.

The next experiment reuses the existing immutable use index for loop-route live-out checks. The existing traversal stays available as a same-compiler ablation. The [completed same-compiler experiment](../indexed-loop-liveouts/README.md) saves a paired 46.909 ms through final link, with 9/10 faster and all 113 control-flow tests passing. The exact 64,444 B worker is unchanged.
