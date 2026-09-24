# Parallel EKF: Interpreted baseline

Interpreter-driven controls. Workload: **10,000 filters x 20 turns where available**. Checked and unchecked are separate columns; source edits are measured against each language's baseline source.

**Scope note:** NumPy is included here because this row is a Python loop invoking one-filter NumPy operations; its array kernels are native, but the outer execution remains interpreter-driven.

| Language | Representative source | Code L/C | Edit vs baseline L/C | Checked M/s | Unchecked M/s | Result |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| Mech | resident scalar interpreter | 42 / 1,513 | 0 / 0 | 0.919 | 1.029 | measured |
| Lua | PUC Lua flat fixed-shape arrays | 153 / 7,031 | 0 / 0 | 0.564 | 0.836 | measured |
| Python | standard-library lists and math | 158 / 5,118 | 0 / 0 | 0.246 | 0.356 | measured |
| NumPy | Python loop over scalar NumPy operations | 66 / 1,819 | 0 / 0 | 0.040 | 0.053 | measured |

`partial` means the source exists but one checked/unchecked measurement is not recorded yet; it is not treated as zero.
