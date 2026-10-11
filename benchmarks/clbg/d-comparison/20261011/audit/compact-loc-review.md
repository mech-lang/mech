# Why three compact D sources are shorter than Mech

This review compares measured compact endpoints, not fastest-source LOC. None of the ten fastest D sources is shorter than its fastest Mech counterpart. The original Mech sources, hashes and counts are preserved in `evidence/site-before.json`; the three exact shorter-source pairs are retained in `shorter-d-cases.json`.

| Benchmark | D LOC | Mech LOC | D / Mech LOC | D seconds | D time / matched Mech AOT |
|---|---:|---:|---:|---:|---:|
| Regex redux | 12 | 24 | 0.500× | 1.757904 | 3.596× |
| FASTA | 29 | 32 | 0.906× | 2.759285 | 5.028× |
| Pi digits | 29 | 35 | 0.829× | 3.136244 | 10.902× |

All three execute the complete required workload. None calls a native benchmark executor. Ordinary regex, arbitrary-precision arithmetic, ranges, search and text repetition are library operations allowed uniformly. Application functions, bindings and algorithms are visible and counted.

## LOC policy check

D regex has twelve nonblank, noncomment lines and no packed-statement expansion. Its pattern vectors contain literal data, not independent executable assignments. Mech already uses the same one-line treatment for probability vectors and ALU text. Requiring one literal item per line only for D would change the metric and would not be a fair correction.

D FASTA has twenty-eight physical code lines. Its `p = sum += p` line performs two updates; the existing normalization expands it to twenty-nine LOC. No packed-assignment discount is awarded. D pi digits has twenty-nine code lines with no packed-statement expansion. Loop headers count once in all three.

The reader also masks D nested comments, strings and loop-header separators. Structural braces are counted consistently with the existing native-language cohorts. Source and full-output hashes match the frozen timing audit.

## Regex redux

Mech's `Count` helper takes three lines, and nine separate calls take another nine. D expresses the same nine searches with a two-line loop over nine literal patterns. Mech's five separate replacement bindings take five lines; D uses a two-line ordered replacement loop over the five explicit pattern/replacement pairs. Mech writes three footer fields separately; D uses one general formatted write. D's three boilerplate lines offset part of this difference.

These are genuine differences in the submitted compact programs. Mech's native sequence path currently lacks general string-vector literals with captured map/comprehension and ordered fold, and its `io/print` path does not admit mixed string/numeric fields. A generic collection/formatting pathway would address the gap without a regex-redux executor. Existing expression definitions and function-application pipelines may already remove six Mech lines by composing `Count` and the replacement sequence; that proposal has not been compiled or measured here.

## FASTA

Mech spends eleven lines expressing `More`, `Search` and `Choose`, including the low/high loop state. D uses the ordinary `countUntil` range operation to locate a cumulative threshold. D also shares one generator across IUB and human alphabets; Mech repeats the LCG and selection callbacks because its output scan consumes a unary callback.

Both sources perform every LCG step and the direct binary64 division and threshold comparisons. Linear and binary cumulative-distribution searches are legitimate alternative algorithms; no input period, integer thresholds or prerecorded output is used. A general upper-bound/first-match operation, or the already supported filtered comprehension reduction, could shorten Mech's search machinery substantially. A roughly twenty-two-line Mech source is a proposal requiring compilation, complete output validation and timing before admission.

## Pi digits

D uses ordinary `BigInt` arithmetic operators and one imperative loop. Mech uses explicit reusable bigint cells and passes a typed state through predicate, step, produce and consume callbacks. Its `Extract` helper takes four lines plus two call sites; D spells the two divisions in two arithmetic expressions. Mech's padding predicate, step and finish callbacks plus final conditional occupy eight lines; D uses general string repetition and a formatted write in two.

The D program retains both stable-digit divisions and the canonical term transformations. Its early impossible-candidate guard is the same general guard admitted in Mech's optimized source. There is no hidden pi kernel. Existing pipelines could compose the bigint extraction and two update intermediates, potentially removing five Mech lines. Value-level bigint operators with safe storage reuse and general text repetition would remove further boilerplate; they are language/stdlib features, not a pi function. No prototype or changed Mech count is admitted by this review.

## Conclusion

The three smaller D counts are valid under the current normalization. They expose avoidable explicit sequencing and gaps in Mech's general compiled library paths. The measured Mech programs remain substantially faster than these particular compact D forms. Counts and timings should remain unchanged until any shorter Mech programs pass the same build, required-work and complete-output checks.
