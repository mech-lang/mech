<p align="center">
  <img width="400px" src="https://mech-lang.org/img/logo.png">
</p>

Mech is a language for developing **data-driven**, **reactive** systems like robots, games, and animations. It makes **composing**, **transforming**, and **distributing** data easy, allowing you to focus on the essential complexity of your project. 

You can try Mech online at [https://try.mech-lang.org](https://try.mech-lang.org).

Usage and installation instructions can be found in the [documentation](https://docs.mech-lang.org) or the [main Mech repository](https://github.com/mech-lang/mech).

Be sure to follow our [blog](https://mech-lang.org/blog/)([RSS](https://mech-lang.org/feed.xml))!

## Feature layers

`mech-stats` separates concrete execution (`runtime`), source specializers
(`source`), and bytecode lowering (`compiler`). `source` and `compiler` each
require `runtime`, but `compiler` does not enable `source`. Enable both when a
consumer needs source elaboration and lowering.

Every leaf operation enables `runtime`. For example, a minimal sum runtime
build uses:

```text
--no-default-features --features "runtime,f64,matrixd,vectord,sum"
```

`runtime_default`, `source_default`, and `compiler_default` select the complete
standalone profiles; the crate default is `compiler_default`.

Use `install_runtime` to add selected factories to a catalog builder and, with
`source` enabled, `install_source` to add specializers and exports. Concrete
Mech distributions normally route these features and installers through
`mech-stdlib`.

## License

Apache 2.0
