<img width="40%" height="40%" src="http://mech-lang.org/img/logo.png">

Mech is a language for developing **data-driven**, **reactive** systems like animations, games, and robots. It makes **composing**, **transforming**, and **distributing** data easy, allowing you to focus on the essential complexity of your project. 

Read about progress on our [blog](http://mech-lang.org/blog/), follow us on Twitter [@MechLang](https://twitter.com/MechLang), or join the mailing list: [talk@mech-lang.org](http://mech-lang.org/page/community/).

## Provided Functions

- `set/any(table)`
- `set/all(table)`
- `set/none(table)`

## Feature layers

`mech-set` separates concrete execution (`runtime`), source specializers
(`source`), and bytecode lowering (`compiler`). `source` and `compiler` each
require `runtime`, but `compiler` does not enable `source`. Enable both when a
consumer needs source elaboration and lowering.

Every leaf operation enables `runtime`. For example, a minimal union runtime
build uses `--no-default-features --features "runtime,set,f64,union"`.
`runtime_default`, `source_default`, and `compiler_default` select the complete
standalone profiles; the crate default is `compiler_default`.

Use `install_runtime` to add selected factories to a catalog builder and, with
`source` enabled, `install_source` to add specializers and exports. Concrete
Mech distributions normally route these features and installers through
`mech-stdlib`.

## License

Apache 2.0
