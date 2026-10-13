# GitLab Pages benchmark publication QA

The user requested publication through the existing Mech website at
https://mech-lang.org/benchmarks/. The old Sites-oriented C3 session was stopped
before packaging; its staged data and original session failure are preserved.
No benchmark, compiler, formatter or prior fixture was rerun for publication.

The complete 29-file distribution was copied to the website repository's
`public/benchmarks/`. Dataset SHA-256 is
`4fe402c21360fbbbe9d650c15d1fd4c62422c17cfad8cafd24721fe7f0bd2947`.
The existing Pages CI publishes `public` from `main`. The preparation receipt
records all asset digests, checks 11 relative asset/download URLs, and confirms
that the destination copy matches the source. Independent source/diff review
accepted the copy, README update and three navigation links.

Actual browser checks at http://127.0.0.1:58302/benchmarks/ loaded 42 language
families and 48 execution variants with no loading error. Opening C3 showed
10/10 workloads, 20 source forms, 60 timing runs and 20 program median markers.
The table uses `Time × best`, and the hover-details strip is outside the SVG.
The regex optimized source opened with 4,000 source characters, three timing
samples, and the fixed best-time baseline description. The prior C3 staging
session additionally opened all 20 main sources and four counted helper files,
and checked exact-program comparison/return navigation and unclipped run centers.

Independent medians computed in log2 coordinates from the qualified data match
the actual rendered C3 median markers:

| Profile | LOC ratio | Time ratio |
|---|---:|---:|
| Fastest | 9.173482314681690 | 3.374585704809872 |
| Fewest LOC | 5.303300858899107 | 10.545404757523870 |

The unchanged application module passed Node syntax checking. Screenshot
capture returned unavailable; these are DOM/source checks, not pixel QA.
The public page loaded its complete data in the browser. C3's live overview
showed 20 programs, 60 runs, `Time × best`, and the external hover strip.
The public pipeline passed and all 29 published assets matched their reviewed
digests. The slashless URL redirects to `/benchmarks/`. Complete verification
is recorded in `published.json`; a transient HTTP2 download failure was resolved
by checking the assets through HTTP1.1. One browser request saw a cached 404
during propagation; the fresh release URL loaded the published chart correctly.
