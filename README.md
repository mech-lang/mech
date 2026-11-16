# Mech Level 12 at 1.021× Roc

This handoff contains the exact Apple M1 diagnostic object from the local level-12 experiment. The comparison uses the generated object's Mach-O __text section and the pinned Roc archive's text. All 162 linked exact-output checks passed.

The six-case screen won 5/6 and 4/6 against Roc in the two execution orders. It won 3/6 against libdeflate in each order, matching the control. Full-suite qualification and M3 timing are pending.

## Files

- [program.o](program.o) is the measured linkable arm64 Mach-O object.
- [metadata.json](metadata.json) contains the native packet layout and the source-origin table.
- [source.mec](source.mec) is the exact level-12 benchmark source. [program.mecb](program.mecb) is its canonical bytecode.
- [receipt.json](receipt.json) records the code-size ratios and content hashes.
- [screening.json](screening.json) has both measured orders, per-case ratios, and reference binary hashes.
- [llvm](llvm) has the optimized LLVM input for the checked root and its private definitions, plus the selected matcher. Module labels in the LLVM files use relative names. The prepared compact object also supplies retained native and runtime symbols.

From the handoff directory, verify the files and inspect the Object with LLVM.

    shasum -a 256 -c SHA256SUMS
    llvm-size -m program.o
    llvm-objdump -d program.o

The root used LLVM 22.1.8 and Apple M1 code generation with the following options.

    -O=2 -global-isel=0 -relocation-model=pic -enable-machine-outliner=always -outliner-benefit-threshold=8 -machine-outliner-reruns=2

The selected matcher was compiled separately with size optimization and machine outlining disabled. See [the bundled attribution and notices](deflate-huffman-LICENSES.md).
