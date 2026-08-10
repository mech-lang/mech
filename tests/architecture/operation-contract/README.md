# C4 operation and port contract boundary

C4 makes every `ProgramArtifact` node name an artifact-local, resolved semantic
contract. Declarations remain reusable compiler/catalog metadata; bytecode v1
stores only concrete schemas and portable access, delivery, construction,
alias, change-detection, observation, effect, and transaction semantics.

`LegacyOpaque` remains a migration escape hatch for ordinary pre-launch source
compilation. It grants no purity or effect guarantees, is forbidden for
integrity constraints, and the synthetic EKF-shaped fixture injects complete
declarations and proves zero `LegacyOpaque` rows under complete metadata plus
stable bytecode round-tripping.

D1 must compile ordinary Mech EKF source without injected declarations,
require zero `LegacyOpaque` contracts, and then activate the resulting
`ProgramArtifact` through the resident executor. Current execution continues
to use `RuntimeFunctionContract`; C4 does not route `ProgramArtifact` or
resolved contracts into the hot turn.

Bytecode v1 evolves in place to 18 sections with
`ArtifactOperationContracts`. There is no pre-launch compatibility reader and
no bytecode-v2 work.
