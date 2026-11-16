//! Compare finalized relocations using the identities from the same module.
use cranelift_module::{FuncId, ModuleReloc, ModuleRelocTarget};

pub(super) fn matching_relocations(
    left_function: FuncId,
    left: &[ModuleReloc],
    right_function: FuncId,
    right: &[ModuleReloc],
) -> bool {
    use ModuleRelocTarget::{FunctionOffset, KnownSymbol, LibCall, User};
    left.len() == right.len()
        && left.iter().zip(right).all(|(a, b)| {
            a.offset == b.offset
                && a.kind == b.kind
                && a.addend == b.addend
                && match (&a.name, &b.name) {
                    (FunctionOffset(a, x), FunctionOffset(b, y)) => {
                        x == y
                            && (*a != left_function).then_some(a)
                                == (*b != right_function).then_some(b)
                    }
                    (
                        User {
                            namespace: a,
                            index: x,
                        },
                        User {
                            namespace: b,
                            index: y,
                        },
                    ) => a == b && x == y,
                    (KnownSymbol(a), KnownSymbol(b)) => a == b,
                    (LibCall(a), LibCall(b)) => a == b,
                    _ => false,
                }
        })
}
