# Futhark strict ISPC control

Apple M1, Futhark 0.27.1, ISPC 1.31.0. The boundary is one ISPC worker,
10,000 independent filters, 20 resident turns, and three steady-state samples.
The build forces `ISPCFLAGS='-O3 --woff --opt=disable-fma'`, so default ISPC
FMA contraction cannot enter the strict comparison.

| Source | Checked (M turns/s) | Unchecked (M turns/s) |
| --- | ---: | ---: |
| `futhark_scalar_ekf.fut` (scalar-expanded covariance) | 30.55 | 43.34 |

The older 52.67M unchecked result used default ISPC contraction and is retained
only as historical evidence; it is not a strict arithmetic result and is not
used in current comparisons.
