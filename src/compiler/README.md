# mech-compiler

`mech-compiler` is the reusable compiler component for Mech.

It parses Mech source with `mech-syntax`, lowers through the existing AST -> bytecode
pipeline in `mech-interpreter`, and provides an LLVM-targeting IR lowering surface for
backend integration.
