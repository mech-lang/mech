<p align="center">
  <img width="400px" src="https://mech-lang.org/img/logo.png">
</p>

Mech is a language for developing **data-driven**, **reactive** systems like robots, games, and animations. It makes **composing**, **transforming**, and **distributing** data easy, allowing you to focus on the essential complexity of your project. 

You can try Mech online at [https://try.mech-lang.org](https://try.mech-lang.org).

Usage and installation instructions can be found in the [documentation](https://docs.mech-lang.org) or the [main Mech repository](https://github.com/mech-lang/mech).

Be sure to follow our [blog](https://mech-lang.org/blog/)([RSS](https://mech-lang.org/feed.xml))!

## Feature layers

`mech-logic` separates concrete execution (`runtime`), source specializers
(`source`), and bytecode lowering (`compiler`). `source` and `compiler` each
require `runtime`, but `compiler` does not enable `source`. Enable both when a
consumer needs source elaboration and lowering.

Every leaf operation enables `runtime`. For example, a minimal logical-and
runtime build uses `--no-default-features --features "runtime,bool,and"`.
`standard_runtime`, `standard_source`, and `standard_compiler` select the
Boolean scalar profiles. The corresponding `full_*` profiles also enable
matrix storage shapes. No features are enabled by default.

Use `install_runtime` to add selected factories to a catalog builder and, with
`source` enabled, `install_source` to add specializers and exports. Concrete
Mech distributions normally route these features and installers through
`mech-stdlib`.

## Boolean reduction

Import `logic/all` to reduce a Boolean scalar, row or column vector, or matrix
to one Boolean value:

```mech
+> logic/all
valid! := all([true true; true true])
```

`all` is true exactly when every element is true. It returns true for an empty
Boolean matrix and rejects non-Boolean inputs. Comparisons can supply its input,
for example `all([a b c] > 0)` checks three scalar values. Matrix comparison is
elementwise; applying it to an entire covariance matrix is not a test of positive
semidefiniteness. The leaf Cargo feature is `all`, exposed as `logic_all` through
`mech-stdlib` and `mech`; complete logic operation profiles include it.

## License

Apache 2.0
