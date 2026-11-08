# IR280: current DEFLATE compiler and checked row specializations

Production compiler source ccfadbdf0af69f741237f5d1099a3c3eef79386a integrates latest
PR973a7c2168 plus the previously unmerged nullary-state foundation, carrying
its selection changes into the canonical phase proof.415actual CI names and
7new upstream tests pass. Original arithmetic text remains exactIR279; compiler
integration alone has no established processing gain and its adverse whole
result is retained. Cranelift stays production; no mandatoryLLVM dependency.

All3source experiments specialize4/5tag rows, retain original other-width loops
and preserve exact scoring/ties. The combined source97fc6c5a448aee096d19eb89839512974c724fee5efae5653ca88bed51b9c00a
wins31/31processing pairs initially and independently. Confirmation2.358833ms/
1.554335x fastest eligible measured Rust1.517583ms. Paired-9.528% vs matched
IR280 control,95%[-10.115%,-8.948%]; instructions49.401M->40.770M(-17.471%).
Versus previousIR278 working control, paired processing-10.773%, whole+4.424%,
95%[+0.591%,+8.289%]. Adopted per user's phase-first policy with costs retained.
Restoration7.239709->7.538334ms, processing requested memory+86880B, native
code+187712B and cumulative stack+592B. All code/metadata/heap/RSS/footprint
metrics and every first/adverse result remain. Rust parity remains unmet.

2332exact cohort outputs,140qualification outputs,415actualnativeCI names,
7new upstream/3mask/8row/27image/4explicitLLVM checks and133exact scalar/DEFLATE
non-timing/allocation rows qualify this working source/compiler. First count
harness failure, missing nullary dependency witness and disk link failure are
preserved, with final successful source-pinned qualification. No hostedCI-green
claim. No own build/test/profile/archive overlapped either performance cohort.

Fresh same-runtime diagnostics bind the same actual production Objects.48exact
outputs/32balanced5thread hardware captures show main numeric21.060M->12.273M
instructions; worker numeric10.239M stays unchanged. Remaining22.512M numeric,
8.839Mcallbacks and8.776Mother generated-entry instructions are diagnostic
medians, not wall-time shares. The profiler is not used for promotion timing.

Run python3 verify.py for numerical and hash replay. Every copied Rust/manifest
source is compressed so source-inventory scans remain exact after publication.
