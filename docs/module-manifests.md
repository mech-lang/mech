# Module manifests

A module manifest is a restricted `.mcfg` config document with a top-level `module` field.

This PR implements only context exports. The first manifest-backed module is `browser`, and its first export is `browser/dom`.

```mech
config := {
  module: {
    name: "browser"
    exports: [
      {
        name: "dom"
        kind: "context"
        base-uri: "browser://dom"
        operations: ["read", "write"]
      }
    ]
  }
}
```

Context exports must be imported with an `@` alias:

```mech
+> @ui := browser/dom
```

This binds `@ui` to the `browser://dom` context base URI.

Direct resource context binding remains available and is not a module import:

```mech
@ui := browser://dom
```

The legacy suffix context form is removed and unsupported:

```mech
foo@bar
```
