# Parser work after statement replay

Four balanced diagnostic pairs preserve the exact identity-bearing tree,
718,353 final events, and 3,967,302 parser steps. The uninstrumented parser
takes **0.917 s**. The phase diagnostics put
**883.5 ms** in grammar execution, **34.0 ms**
in tree indexing and **2.2 ms** in restart indexing.
The replay families account for **162.4 ms**
of exclusive grammar time. The statement recipe applies for 117 of 123
winners. Six exceed the established recipe bounds.
[Phase medians and family rows](report.json) · [All diagnostic pairs](phase-observations.json.gz)

Four separate stack captures measure where the unmodified parser spends its time.

- base-continuation accounts for 19.8% of canonical parser samples.
- literal-scan accounts for 16.1% of canonical parser samples.
- precedence-continuation accounts for 9.6% of canonical parser samples.
- grammar-journal accounts for 9.3% of canonical parser samples.

[Stack sample attribution](stack-samples/report.json)

## Fixed literal scans

Separate counting shows **1,652,988** Unicode cursor calls on the expected
side and **2,614,359** on the source side. Expected-side work accounts for
**38.7%** of the counted Unicode calls and comes from **164** static
literals. ASCII literals account for 1,360,746 of the expected-side calls.
There are 166,988 cold scans for keys that completed earlier. Those repeated
cold scans account for 538,049 Unicode calls across both sides.

The count covers completed final-input scans of fixed literals. The
instrumented parser reproduces the exact tree, event count and work
accounting. The data quantify the opportunity to reuse expected-literal
grapheme shapes. Any such reuse must reproduce the original logical
allowance and keep the existing segmented and partial-grant behavior.
[Counts and identity checks](literal-census/report.json)
· [Per-literal observations](literal-census/counts.log.gz)
