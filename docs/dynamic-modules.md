# Dynamic module smoke path

The dynamic module prototype can be validated with:

```bash
./scripts/test-dynamic-modules.sh
```

The script performs the full local smoke path:

1. Checks the interpreter with `base dynamic-modules f64`.
2. Tests and builds the math dynamic provider.
3. Tests and builds the combinatorics dynamic provider.
4. Stages provider artifacts under `target/mech-modules`.
5. Runs dynamic math integration tests with `MECH_MODULE_PATH`.
6. Runs dynamic combinatorics integration tests with `MECH_MODULE_PATH`.
7. Runs dynamic module smoke tests with `MECH_MODULE_PATH`.

The script does not install modules globally. It uses only repository-local build output.

The staged module directory is:

```text
target/mech-modules
```

The provider build directory is:

```text
target/dynamic-modules
```

The current smoke path validates:

```mech
+> math
+> math/sin
+> math/*
+> combinatorics/n-choose-k
```

It also validates missing module and missing item failures.

This is a prototype validation path for the current narrow dynamic ABI. It is not a package manager, module installer, or dynamic dependency resolver.

Do not document unimplemented behavior.

Do not mention future features as working.
