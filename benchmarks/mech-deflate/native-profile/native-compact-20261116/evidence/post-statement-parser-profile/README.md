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
