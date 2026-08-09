<p align="center">
  <img width="400px" src="https://mech-lang.org/img/logo.png">
</p>

Mech is a language for developing **data-driven**, **reactive** systems like robots, games, and animations. It makes **composing**, **transforming**, and **distributing** data easy, allowing you to focus on the essential complexity of your project. 

You can try Mech online at [https://try.mech-lang.org](https://try.mech-lang.org).

Usage and installation instructions can be found in the [documentation](https://docs.mech-lang.org) or the [main Mech repository](https://github.com/mech-lang/mech).

Be sure to follow our [blog](https://mech-lang.org/blog/)([RSS](https://mech-lang.org/feed.xml))!

## Feature layers

`mech-matrix` separates concrete execution (`runtime`), source specializers
(`source`), and bytecode lowering (`compiler`). `source` and `compiler` each
require `runtime`, but `compiler` does not enable `source`. Enable both when a
consumer needs source elaboration and lowering.

Every leaf operation enables `runtime`. For example, a minimal solve runtime
build uses:

```text
--no-default-features --features "runtime,f64,matrixd,vectord,solve"
```

This profile does not require `transpose`. `runtime_default`, `source_default`,
and `compiler_default` select the complete standalone profiles; the crate
default is `compiler_default`.

Use `install_runtime` to add selected factories to a catalog builder and, with
`source` enabled, `install_source` to add specializers and exports. Concrete
Mech distributions normally route these features and installers through
`mech-stdlib`.

## Benchmarks

- [Matrix steady-state](benchmarks/steady_state/README.md) compares matrix
  multiplication, materialized transpose, and solve across retained Mech,
  Rust, Python, NumPy, Lua, LuaJIT, and Julia.
- [EKF runtime loop](benchmarks/ekf/README.md) compares a persistent
  three-state extended Kalman filter across the same runtime families.
- [N-body](benchmarks/nbody/README.md) provides exact Computer Language
  Benchmarks Game programs plus NumPy and validated retained Mech fixtures.

## License

Apache 2.0
