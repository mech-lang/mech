# Root source review

Root read the overview state, source catalog, log-coordinate median calculation, dedicated domains, chart markers, inspector navigation, comparison view and event handlers. The comparison uses a detached view containing the exact selected source, with the original fixed benchmark baselines. It does not change canonical endpoint selection or measured data. Inventory and endpoint records share checked source and timing identities.

The highlighted view enables all retained program medians and individual runs, the coordinate medians and benchmark covariance. The per-program table reports timing medians, recorded standard deviations and numerical samples. It distinguishes run variation from variation across benchmarks and labels unavailable workload results. The fixed hover strip remains above the SVG.

The implementation keeps prior chart selections and inspector state, comparison source bookmarks and source file index. Variant and workload controls persist through rerenders. Root accepted these source changes for syntax and browser verification. This note does not assert execution, browser success, commit, push or publication.

The actual verification and publication result will be recorded separately after the serialized chart session. No new benchmark runs are required for this interface.
