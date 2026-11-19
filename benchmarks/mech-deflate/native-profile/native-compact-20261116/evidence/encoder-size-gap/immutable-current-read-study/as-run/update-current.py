from pathlib import Path
import json
P=Path('/Users/coreymontella/.codex/worktrees/pr995-runtime-integration/mech-shared-cold-epilogues.2L6obP/benchmarks/mech-deflate/native-profile/native-compact-20261116');E='evidence/encoder-size-gap/immutable-current-read-study';source='5ccf097dc02ceb3abb17bccb85cd11a3ae144503';read=lambda p:json.loads(p.read_text());r=read(P/E/'placement/result.json');b=read(P/E/'placement/matched-builds/summary.json');profiles=read(P/E/'placement/current-runtime-profiles/summary.json')
summary='''The selected M1 recipe is **`m1-immutable-current-reads`**: **63,532 B**, saving **160 B**, with **81/81 Roc and 62/81 libdeflate wins in both complete orders**. Every paired control win and all 81/62 precision-supported wins are retained. Original Mech source and canonical bytecode are unchanged; no LLVM dependency is added. Compiler source is integrated at `5ccf097dc02ceb3abb17bccb85cd11a3ae144503`.

The source-through-final-link build is **2.211926 s**, versus **2.213335 s** with the pass disabled in the same compiler. The paired difference is **-0.139 ms**, 90% interval **-3.628 to +3.598 ms**, 8/16 faster: **no established compile-time change**. Sixteen balanced trials per actor include final linking, with support prebuilt for every actor. The complete stripped executable remains **276,072 B**. Runtime reports **13 established small slowdowns above 0.5% and 15 uncertain comparisons**; no individual peer win is lost. Large random 1 MiB costs 0.54% / 0.77% versus control. [Source, all controls, full orders, profiles and complete builds](evidence/encoder-size-gap/immutable-current-read-study/README.md).
'''
current='''## Current Apple M1 result

'''+summary+'''
| Actor | Encoder text | Source through final link | Through strip | Stripped executable |
| --- | ---: | ---: | ---: | ---: |
| Same compiler, pass disabled | 63,692 B | 2.213335 s | 2.224324 s | 276,072 B |
| Selected immutable-read reuse | **63,532 B** | **2.211926 s** | **2.223045 s** | 276,072 B |
| Roc | 57,696 B | 2.988797 s | 2.999631 s | 191,912 B |
| libdeflate | 37,080 B | 0.491145 s | 0.502016 s | 155,752 B |

Paired build ratios are **0.739157× Roc / 4.505969× C**. Encoder ratios are **1.101151× Roc / 1.713376× C**, leaving **26,452 B** above C. Compiler bootstrap is excluded for all actors; strip is separate. Historical cohort medians are not causal comparisons.

Current phase medians: parse **704.371 ms**, semantics **217.767 ms**, native pipeline **1,218.926 ms**. Catalog work overlaps parsing and independent medians do not add. The post-encode sample finds dominance, SSA construction, validation and guard inference among the leading active functions; it is not an exclusive phase timer.

The historical literal-reuse lead is now a generic immutable-loan/current-index proof. A 63,484 B alternative lost C on `320-overlap` by 0.18% in one focused order. Restricting the pass to the two certified reads and allowing backend rematerialization keeps 160 B of that saving and restores the win in both full orders. The original branches remain for all uncertified reads. All adverse variants and source snapshots are retained.

The selected worker is `834c7a7e423a69c46483a3dae74d9450949fdf142b5ad2dede894e0da3ca8693`. Normal and allocation-checked source builds reproduce identical object, full metadata and bytecode. It passes 162 output/inflation/alias observations, six failures, 12 recoveries and **4,848 B** peak stack under the 5,280 B ceiling. The root remains **46,788 B**, with a **3,296 B** frame. Four executable load tests cover both placement choices; four unchanged optimized private-contract tests cover original/decoded loans and refusal paths. Prior syntax qualification remains 539 tests; no syntax source changes in this pass.

Fresh actual-worker samples put **72.3%** of 1 MiB encoder work in matching and **14.4%** in scoring. Natural 4 KiB is **52.3% scoring / 24.5% matching**. Large random still has the largest absolute gap, about **7.344 ms/call**; natural 4 KiB is the closest next-win check at **1.063757× / 1.068946× C**, needing about **6.45%** less time in the worse order. Sample shares are not elapsed phase timers or forecasts.

The 1× libdeflate code/build goal remains active. The next major architecture task is reducing constructed/analyzed controller state, using the existing early-unit pilot and its adverse size/stack/build evidence. Keep every existing peer win and both 81/62 floors; target 63 C wins next. [Review handoff](REVIEW.md).

```sh
python3 rebuild.py /tmp/mech-immutable --variant m1-immutable-current-reads \\
  --mimalloc-object /path/to/mimalloc.o
```

'''
p=P/'CURRENT.md';old=p.read_text();old=old.replace('# PR #995 current status\n\n','',1).replace('## Current Apple M1 result','## Previous sealed-parser cohort (63,692 B)',1).replace('The selected encoder remains **63,692 B**','The then-selected encoder remained **63,692 B**',1).replace('## Matcher guard/layout qualification cohort','## Previous matcher guard/layout qualification cohort',1);p.write_text('# PR #995 current status\n\n'+current+old)
p=P/'README.md';old=p.read_text();start=old.index('This PR contains compiler and parser source');old=old[start:];old=old.replace('It is stacked\non #973 at `9087b1f07727a9bfcb7c09059f48edaca6dc2301`.','It targets #993, which is stacked on #973.',1);old=old.replace('Use `--variant m1-matcher-guard-facts` for the current selected recipe.','Use `--variant m1-immutable-current-reads` for the current selected recipe.');p.write_text('# Native compact compiler review handoff\n\n'+summary+'\nUse [CURRENT.md](CURRENT.md) for current identities, timings and tradeoffs; earlier measurements below are historical evidence.\n\n'+old)
review='''# Review and continuation handoff

Review [mech-lang/mech PR #995](https://github.com/mech-lang/mech/pull/995), branch `codex/pr993-m3-optimization`. It targets #993's `codex/native-compact-source-handoff`, stacked on #973. Compiler source, original Mech program, build recipes, runtime source and evidence are in this branch. The immutable-read compiler source is committed at `5ccf097dc02ceb3abb17bccb85cd11a3ae144503`; subsequent evidence commits preserve those Rust inputs.

```sh
git fetch origin codex/pr993-m3-optimization
git switch -c codex/review-native-compact origin/codex/pr993-m3-optimization
```

'''+summary+'''
## Build and verify

```sh
python3 rebuild.py /tmp/mech-immutable --variant m1-immutable-current-reads \\
  --mimalloc-object /path/to/mimalloc.o
python3 evidence/encoder-size-gap/immutable-current-read-study/verify.py
```

With an existing compiler built from this source, use the evidence package's `reproduce.py` for portable normal/allocation-checked object, metadata and bytecode replay. The selected fixed-support worker is `834c7a7e423a69c46483a3dae74d9450949fdf142b5ad2dede894e0da3ca8693`. It passes 162 output/inflation/alias observations, six failures, 12 recoveries and 4,848 B stack under the unchanged 5,280 B ceiling. Four source-bound executable load tests and four unchanged optimized private-contract tests cover original/decoded authority, null/default/wrapping values, current loops and divergent paths. Normal and checked artifacts match exactly. The disabled pass reproduces the previous selected 63,692 B worker.

## History and remaining runtime losses

The [19-mechanism audit and all-loss profiles](evidence/history-and-profiles-20261119/README.md) led to retained spill pairing, then unsigned guards/fresh layout, and now literal-reuse repair. Do not reject gains just for small slowdowns with peer wins intact. Preserve every adverse attempt and make bounded repairs before abandoning a mechanism. The 208 B smaller broad variant had a single near C loss; the narrower variant keeps 160 B and recovers it. The larger-random cost falls from roughly 2.7% focused to 0.54–0.77% in full 1 MiB orders.

The latest actual-worker captures put 72.3% of 1 MiB encoder samples in matching and 14.4% in scoring; natural 4 KiB is 52.3% scoring / 24.5% matching. Sample fractions are not elapsed phase timers. Large random has about 7.344 ms excess per call, while natural 4 KiB has about 25.1 microseconds. Optimize the shared random collector first, keeping natural 4 KiB as the closest next-win check: 1.063757× / 1.068946× C, needing about 6.45% less time. No 63rd C win is claimed. [Every current case ratio](evidence/encoder-size-gap/immutable-current-read-study/placement/runtime-case-ratios.json).

On 1 MiB, the new collector saves 1.15 million stack loads and 2.10 million stack stores but adds ALU work: 239.33 million instructions versus 233.15 million. Rematerialization repeats the physical byte load. Carrying a valid current-input extent through the private call may remove repeated checks/address work; that requires a real generic admission proof and fallback, not a profile-based safety assumption. The earlier cursor/countdown/ending-pointer repair remains a separate scorer lead. [Source and adverse pointer variants](evidence/natural4k-runtime-target/README.md).

## Main architecture work

The encoder is still 1.713376× C; complete build is 4.505969× C. Its 46,788 B controller alone exceeds C's entire 37,080 B encoder. Parsing takes 704 ms, semantics 218 ms and the native pipeline 1,219 ms. The post-encode compiler capture again finds dominance, SSA construction, validation and guard inference among the leading active functions. Reduce work constructed and analyzed rather than extend the small late-pass campaign indefinitely.

The existing early codebook unit is the pilot: 885 source operations omitted before root SSA, 42 input/27 output words, total allocated SSA down 18.6%, constructed instructions down 27.7%. Its recorded implementation is unselected: 69,828 B whole text, 5,808 B stack above the 5,280 B cap, and complete build 2.277083 s versus 2.209673 s in its cohort. Correctness/errors/recovery pass; it has no inherited 81/62 runtime qualification. Keep its gain and repair boundary/retirement state, active scratch and valid late sharing through source/unit maps. Old native block IDs do not remain valid after partitioning. Price all helpers, call setup, spills, stubs and alignment. [Early source emission and all adverse evidence](evidence/early-codebook-emission/README.md).

The qualified late codebook split remains available as `m1-checked-codebook-phase`: 64,444 B, the same peer wins and 160 B lower peak stack in its original cohort, with a 640 B cost versus that cohort's selected reference. It is distinct from the earlier literal-frame 64,444 B recipe. The previous `m1-matcher-guard-facts` and `m1-register-spill-pairs` recipes remain reproducible.

Sealed base phase fusion is already integrated parser maintenance (539 syntax tests), and sparse guard scheduling's earlier 34 ms saving is already adopted. Larger sealed parser-family specialization and conservative prepared-helper IR reuse remain unfinished. Do not add separate cohorts' savings together or report adopted work as a new gain.

Preserve direct Bool/U64/I64/F64 native emission, checked errors/origins, ownership and finite admission. Ordinary resource-accounting limitations remain as documented in QUALIFICATION.md. No new CI-green claim is made; the earlier archived M3 ABI oracle static failures are outside this change. Current M1 code/build evidence must not be replaced with historical M3/LLVM figures.
''';(P/'REVIEW.md').write_text(review)
s=read(P/'scorecard.json');s['previous_latest_candidate_reference']={'commit':source,'path':'benchmarks/mech-deflate/native-profile/native-compact-20261116/scorecard.json','key':'latest_candidate'};s['latest_candidate']={'name':'certified immutable current-input reads','encoder_text_bytes':63532,'current_implementation_candidate':True,'opt_in':True,'decision':'Retain','runtime':r,'evidence':E+'/README.md'};s['m1_recipe_selection']['previous_peer_qualified_opt_in']=s['m1_recipe_selection']['peer_qualified_opt_in'];s['m1_recipe_selection']['peer_qualified_opt_in']={'recipe':'m1-immutable-current-reads','source':source,'encoder_text_bytes':63532,'worker':r['worker_sha256'],'compiler_sha256':r['compiler_sha256'],'runtime':{'decision':'Retain','wins':{'roc':[81,81],'libdeflate':[62,62]},'precision_supported_roc':[81,81],'precision_supported_libdeflate':[62,62]},'runtime_tradeoff':'13 established slowdowns above0.5%,15 uncertain comparisons; all individual peer wins preserved. Large random1MiB costs0.54%/0.77%.','latest_default_build':{'source_through_final_link_seconds':b['medians']['mech-immutable-current-read']['source_through_link'],'paired_ratios':b['paired_ratios']['mech-immutable-current-read'],'same_compiler_change':b['changes']['mech-immutable-current-read_versus_mech-current-disabled']},'evidence':E+'/README.md'};s['current_matched_build']='immutable_current_reads';s['immutable_current_reads']={'source_commit':source,'evidence':E+'/README.md','runtime':r,'build':b,'profiles':profiles};s['review'].update(compiler_source_commit=source,compiler_sha256=r['compiler_sha256'],latest_deployment_evidence=E+'/README.md');s['next_runtime_target']={'case':'prefix-natural-4096','paired_mech_c':[1.0637572685977872,1.0689458677488346],'required_time_reduction':r['next_c_target']['needed_time_reduction'],'primary_investigation':'Shared large-random collector; closest-next-win scorer remains a required canary.','evidence':E+'/placement/runtime-case-ratios.json'};(P/'scorecard.json').write_text(json.dumps(s,indent=2)+'\n')
