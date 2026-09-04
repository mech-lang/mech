#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ImplementationMemoryClass {
    NoAdditionalScratch,
    CloneInput { input: u16 },
    MatrixSolve,
    CanonicalFinalize,
    CanonicalSortUnique,
}
