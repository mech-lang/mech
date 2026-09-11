//! Pure type-memory contract derivation.

use crate::{
    CardinalitySpec, DimensionExpr, DimensionLifetime, DimensionParameter, ExtentEvolution,
    FloatWidth, IntegerWidth, SchemaBody, SemanticModelError, ShapeInstance,
};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, vec::Vec};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScalarMemoryKind {
    Bool,
    Unsigned(IntegerWidth),
    Signed(IntegerWidth),
    Floating(FloatWidth),
    Complex(FloatWidth),
    Rational64,
    String,
    Id,
    Index,
    Atom,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryTopology {
    Dynamic,
    Scalar(ScalarMemoryKind),
    Tagged { variants: u64 },
    Product { members: u64, named: bool },
    DenseSequence { rank: u64 },
    Columnar { columns: u64 },
    OrderedSet,
    OrderedMap,
    ReifiedType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MemoryExtent {
    Single,
    FixedArity(u64),
    Dimensions(Box<[DimensionExpr]>),
    Cardinality(CardinalitySpec),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResolvedAxisExtent {
    pub value: u64,
    pub evolution: ExtentEvolution,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResolvedMemoryExtent {
    Single,
    FixedArity(u64),
    Dimensions(Box<[ResolvedAxisExtent]>),
    ExactCardinality(u64),
    DynamicCardinality { upper_bound: Option<u64> },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AddressingContract {
    pub positional_rank: Option<u64>,
    pub named_members: bool,
    pub keyed_members: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CanonicalizationContract {
    pub self_describing: bool,
    pub recursive: bool,
    pub tagged: bool,
    pub ordered_keys: bool,
    pub unique_keys: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PayloadAccounting {
    FixedWidth,
    VariableWidth,
    Recursive,
    SelfDescribing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PopulationAccounting {
    Single,
    FixedArity,
    ShapeResolved,
    ExactCardinality,
    ValueCardinality,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuxiliaryAccounting {
    pub tag: bool,
    pub ordered_index: bool,
    pub column_directory: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AccountingContract {
    pub payload: PayloadAccounting,
    pub population: PopulationAccounting,
    pub auxiliary: AuxiliaryAccounting,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeMemoryContract {
    pub topology: MemoryTopology,
    pub extent: MemoryExtent,
    pub extent_evolution: ExtentEvolution,
    pub addressing: AddressingContract,
    pub canonicalization: CanonicalizationContract,
    pub accounting: AccountingContract,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedTypeMemoryContract {
    pub topology: MemoryTopology,
    pub extent: ResolvedMemoryExtent,
    pub extent_evolution: ExtentEvolution,
    pub addressing: AddressingContract,
    pub canonicalization: CanonicalizationContract,
    pub accounting: AccountingContract,
}

const NO_ADDRESSING: AddressingContract = AddressingContract {
    positional_rank: None,
    named_members: false,
    keyed_members: false,
};

const NO_CANONICALIZATION: CanonicalizationContract = CanonicalizationContract {
    self_describing: false,
    recursive: false,
    tagged: false,
    ordered_keys: false,
    unique_keys: false,
};

const NO_AUXILIARY: AuxiliaryAccounting = AuxiliaryAccounting {
    tag: false,
    ordered_index: false,
    column_directory: false,
};

fn checked_len(length: usize) -> Result<u64, SemanticModelError> {
    u64::try_from(length).map_err(|_| SemanticModelError::DimensionOverflowV1)
}

fn join_evolution(left: ExtentEvolution, right: ExtentEvolution) -> ExtentEvolution {
    use ExtentEvolution::{ActivationFixed, Fixed, TurnBounded, TurnUnbounded};

    match (left, right) {
        (TurnUnbounded, _) | (_, TurnUnbounded) => TurnUnbounded,
        (TurnBounded, _) | (_, TurnBounded) => TurnBounded,
        (ActivationFixed, _) | (_, ActivationFixed) => ActivationFixed,
        (Fixed, Fixed) => Fixed,
    }
}

fn dimension_evolution(
    expression: &DimensionExpr,
    parameters: &[DimensionParameter],
) -> Result<ExtentEvolution, SemanticModelError> {
    match expression {
        DimensionExpr::Hole => {
            debug_assert!(false, "finalized schemas cannot retain dimension holes");
            Err(SemanticModelError::UnresolvedDimensionHole)
        }
        DimensionExpr::Constant(_) => Ok(ExtentEvolution::Fixed),
        DimensionExpr::Parameter(id) => {
            let parameter = parameters
                .get(id.get() as usize)
                .ok_or(SemanticModelError::UnknownDimensionParameterV1 { id: *id })?;
            match parameter.lifetime() {
                DimensionLifetime::CompileTime => {
                    debug_assert!(
                        false,
                        "finalized schemas cannot retain compile-time parameters"
                    );
                    Err(SemanticModelError::CompileTimeDimensionParameterV1)
                }
                DimensionLifetime::Activation => Ok(ExtentEvolution::ActivationFixed),
                DimensionLifetime::Turn if parameter.upper_bound().is_some() => {
                    Ok(ExtentEvolution::TurnBounded)
                }
                DimensionLifetime::Turn => Ok(ExtentEvolution::TurnUnbounded),
            }
        }
        DimensionExpr::Add(operands)
        | DimensionExpr::Multiply(operands)
        | DimensionExpr::Min(operands)
        | DimensionExpr::Max(operands) => {
            operands
                .iter()
                .try_fold(ExtentEvolution::Fixed, |evolution, operand| {
                    Ok(join_evolution(
                        evolution,
                        dimension_evolution(operand, parameters)?,
                    ))
                })
        }
    }
}

fn cardinality_evolution(
    cardinality: &CardinalitySpec,
    parameters: &[DimensionParameter],
) -> Result<ExtentEvolution, SemanticModelError> {
    match cardinality {
        CardinalitySpec::Exact(expression) => dimension_evolution(expression, parameters),
        CardinalitySpec::Dynamic {
            upper_bound: Some(expression),
        } => Ok(join_evolution(
            ExtentEvolution::TurnBounded,
            dimension_evolution(expression, parameters)?,
        )),
        CardinalitySpec::Dynamic { upper_bound: None } => Ok(ExtentEvolution::TurnUnbounded),
    }
}

fn body_evolution(
    body: &SchemaBody,
    parameters: &[DimensionParameter],
) -> Result<ExtentEvolution, SemanticModelError> {
    match body {
        SchemaBody::Dynamic => Ok(ExtentEvolution::TurnUnbounded),
        SchemaBody::Bool
        | SchemaBody::UnsignedInteger(_)
        | SchemaBody::SignedInteger(_)
        | SchemaBody::FloatingPoint(_)
        | SchemaBody::Complex(_)
        | SchemaBody::Rational64
        | SchemaBody::String
        | SchemaBody::Id
        | SchemaBody::Index
        | SchemaBody::Atom(_)
        | SchemaBody::ReifiedType => Ok(ExtentEvolution::Fixed),
        SchemaBody::Enum { variants, .. } => {
            variants
                .iter()
                .try_fold(
                    ExtentEvolution::Fixed,
                    |evolution, variant| match &variant.payload {
                        Some(payload) => Ok(join_evolution(
                            evolution,
                            body_evolution(payload, parameters)?,
                        )),
                        None => Ok(evolution),
                    },
                )
        }
        SchemaBody::Option(element) => body_evolution(element, parameters),
        SchemaBody::Tuple(elements) => {
            elements
                .iter()
                .try_fold(ExtentEvolution::Fixed, |evolution, element| {
                    Ok(join_evolution(
                        evolution,
                        body_evolution(element, parameters)?,
                    ))
                })
        }
        SchemaBody::Record(fields) => {
            fields
                .iter()
                .try_fold(ExtentEvolution::Fixed, |evolution, field| {
                    Ok(join_evolution(
                        evolution,
                        body_evolution(&field.schema, parameters)?,
                    ))
                })
        }
        SchemaBody::Matrix {
            element,
            dimensions,
        } => {
            let dimension_evolution =
                dimensions
                    .iter()
                    .try_fold(ExtentEvolution::Fixed, |evolution, dimension| {
                        Ok(join_evolution(
                            evolution,
                            dimension_evolution(dimension, parameters)?,
                        ))
                    })?;
            Ok(join_evolution(
                dimension_evolution,
                body_evolution(element, parameters)?,
            ))
        }
        SchemaBody::Table { columns, rows } => columns.iter().try_fold(
            cardinality_evolution(rows, parameters)?,
            |evolution, column| {
                Ok(join_evolution(
                    evolution,
                    body_evolution(&column.schema, parameters)?,
                ))
            },
        ),
        SchemaBody::Set {
            element,
            cardinality,
        } => Ok(join_evolution(
            cardinality_evolution(cardinality, parameters)?,
            body_evolution(element, parameters)?,
        )),
        SchemaBody::Map {
            key,
            value,
            cardinality,
        } => Ok(join_evolution(
            cardinality_evolution(cardinality, parameters)?,
            join_evolution(
                body_evolution(key, parameters)?,
                body_evolution(value, parameters)?,
            ),
        )),
    }
}

fn accounting(
    payload: PayloadAccounting,
    population: PopulationAccounting,
    auxiliary: AuxiliaryAccounting,
) -> AccountingContract {
    AccountingContract {
        payload,
        population,
        auxiliary,
    }
}

fn fixed_scalar(kind: ScalarMemoryKind) -> TypeMemoryContract {
    TypeMemoryContract {
        topology: MemoryTopology::Scalar(kind),
        extent: MemoryExtent::Single,
        extent_evolution: ExtentEvolution::Fixed,
        addressing: NO_ADDRESSING,
        canonicalization: NO_CANONICALIZATION,
        accounting: accounting(
            PayloadAccounting::FixedWidth,
            PopulationAccounting::Single,
            NO_AUXILIARY,
        ),
    }
}

pub(crate) fn derive_type_memory_contract(
    body: &SchemaBody,
    parameters: &[DimensionParameter],
) -> Result<TypeMemoryContract, SemanticModelError> {
    let contract = match body {
        SchemaBody::Dynamic => TypeMemoryContract {
            topology: MemoryTopology::Dynamic,
            extent: MemoryExtent::Single,
            extent_evolution: ExtentEvolution::TurnUnbounded,
            addressing: NO_ADDRESSING,
            canonicalization: CanonicalizationContract {
                self_describing: true,
                recursive: true,
                ..NO_CANONICALIZATION
            },
            accounting: accounting(
                PayloadAccounting::SelfDescribing,
                PopulationAccounting::Single,
                NO_AUXILIARY,
            ),
        },
        SchemaBody::Bool => fixed_scalar(ScalarMemoryKind::Bool),
        SchemaBody::UnsignedInteger(width) => fixed_scalar(ScalarMemoryKind::Unsigned(*width)),
        SchemaBody::SignedInteger(width) => fixed_scalar(ScalarMemoryKind::Signed(*width)),
        SchemaBody::FloatingPoint(width) => fixed_scalar(ScalarMemoryKind::Floating(*width)),
        SchemaBody::Complex(width) => fixed_scalar(ScalarMemoryKind::Complex(*width)),
        SchemaBody::Rational64 => fixed_scalar(ScalarMemoryKind::Rational64),
        SchemaBody::String => TypeMemoryContract {
            addressing: AddressingContract {
                positional_rank: Some(1),
                ..NO_ADDRESSING
            },
            accounting: accounting(
                PayloadAccounting::VariableWidth,
                PopulationAccounting::Single,
                NO_AUXILIARY,
            ),
            ..fixed_scalar(ScalarMemoryKind::String)
        },
        SchemaBody::Id => fixed_scalar(ScalarMemoryKind::Id),
        SchemaBody::Index => fixed_scalar(ScalarMemoryKind::Index),
        SchemaBody::Atom(_) => fixed_scalar(ScalarMemoryKind::Atom),
        SchemaBody::Enum { variants, .. } => TypeMemoryContract {
            topology: MemoryTopology::Tagged {
                variants: checked_len(variants.len())?,
            },
            extent: MemoryExtent::Single,
            extent_evolution: body_evolution(body, parameters)?,
            addressing: NO_ADDRESSING,
            canonicalization: CanonicalizationContract {
                recursive: true,
                tagged: true,
                ..NO_CANONICALIZATION
            },
            accounting: accounting(
                PayloadAccounting::Recursive,
                PopulationAccounting::Single,
                AuxiliaryAccounting {
                    tag: true,
                    ..NO_AUXILIARY
                },
            ),
        },
        SchemaBody::Option(_) => TypeMemoryContract {
            topology: MemoryTopology::Tagged { variants: 2 },
            extent: MemoryExtent::Single,
            extent_evolution: body_evolution(body, parameters)?,
            addressing: NO_ADDRESSING,
            canonicalization: CanonicalizationContract {
                recursive: true,
                tagged: true,
                ..NO_CANONICALIZATION
            },
            accounting: accounting(
                PayloadAccounting::Recursive,
                PopulationAccounting::Single,
                AuxiliaryAccounting {
                    tag: true,
                    ..NO_AUXILIARY
                },
            ),
        },
        SchemaBody::Tuple(elements) => {
            let members = checked_len(elements.len())?;
            TypeMemoryContract {
                topology: MemoryTopology::Product {
                    members,
                    named: false,
                },
                extent: MemoryExtent::FixedArity(members),
                extent_evolution: body_evolution(body, parameters)?,
                addressing: AddressingContract {
                    positional_rank: Some(1),
                    ..NO_ADDRESSING
                },
                canonicalization: CanonicalizationContract {
                    recursive: true,
                    ..NO_CANONICALIZATION
                },
                accounting: accounting(
                    PayloadAccounting::Recursive,
                    PopulationAccounting::FixedArity,
                    NO_AUXILIARY,
                ),
            }
        }
        SchemaBody::Record(fields) => {
            let members = checked_len(fields.len())?;
            TypeMemoryContract {
                topology: MemoryTopology::Product {
                    members,
                    named: true,
                },
                extent: MemoryExtent::FixedArity(members),
                extent_evolution: body_evolution(body, parameters)?,
                addressing: AddressingContract {
                    named_members: true,
                    ..NO_ADDRESSING
                },
                canonicalization: CanonicalizationContract {
                    recursive: true,
                    ..NO_CANONICALIZATION
                },
                accounting: accounting(
                    PayloadAccounting::Recursive,
                    PopulationAccounting::FixedArity,
                    NO_AUXILIARY,
                ),
            }
        }
        SchemaBody::Matrix { dimensions, .. } => {
            let rank = checked_len(dimensions.len())?;
            TypeMemoryContract {
                topology: MemoryTopology::DenseSequence { rank },
                extent: MemoryExtent::Dimensions(dimensions.clone()),
                extent_evolution: body_evolution(body, parameters)?,
                addressing: AddressingContract {
                    positional_rank: Some(rank),
                    ..NO_ADDRESSING
                },
                canonicalization: CanonicalizationContract {
                    recursive: true,
                    ..NO_CANONICALIZATION
                },
                accounting: accounting(
                    PayloadAccounting::Recursive,
                    PopulationAccounting::ShapeResolved,
                    NO_AUXILIARY,
                ),
            }
        }
        SchemaBody::Table { columns, rows } => TypeMemoryContract {
            topology: MemoryTopology::Columnar {
                columns: checked_len(columns.len())?,
            },
            extent: MemoryExtent::Cardinality(rows.clone()),
            extent_evolution: body_evolution(body, parameters)?,
            addressing: AddressingContract {
                positional_rank: Some(2),
                named_members: true,
                ..NO_ADDRESSING
            },
            canonicalization: CanonicalizationContract {
                recursive: true,
                ..NO_CANONICALIZATION
            },
            accounting: accounting(
                PayloadAccounting::Recursive,
                match rows {
                    CardinalitySpec::Exact(_) => PopulationAccounting::ExactCardinality,
                    CardinalitySpec::Dynamic { .. } => PopulationAccounting::ValueCardinality,
                },
                AuxiliaryAccounting {
                    column_directory: true,
                    ..NO_AUXILIARY
                },
            ),
        },
        SchemaBody::Set { cardinality, .. } => TypeMemoryContract {
            topology: MemoryTopology::OrderedSet,
            extent: MemoryExtent::Cardinality(cardinality.clone()),
            extent_evolution: body_evolution(body, parameters)?,
            addressing: AddressingContract {
                keyed_members: true,
                ..NO_ADDRESSING
            },
            canonicalization: CanonicalizationContract {
                recursive: true,
                ordered_keys: true,
                unique_keys: true,
                ..NO_CANONICALIZATION
            },
            accounting: accounting(
                PayloadAccounting::Recursive,
                match cardinality {
                    CardinalitySpec::Exact(_) => PopulationAccounting::ExactCardinality,
                    CardinalitySpec::Dynamic { .. } => PopulationAccounting::ValueCardinality,
                },
                AuxiliaryAccounting {
                    ordered_index: true,
                    ..NO_AUXILIARY
                },
            ),
        },
        SchemaBody::Map { cardinality, .. } => TypeMemoryContract {
            topology: MemoryTopology::OrderedMap,
            extent: MemoryExtent::Cardinality(cardinality.clone()),
            extent_evolution: body_evolution(body, parameters)?,
            addressing: AddressingContract {
                keyed_members: true,
                ..NO_ADDRESSING
            },
            canonicalization: CanonicalizationContract {
                recursive: true,
                ordered_keys: true,
                unique_keys: true,
                ..NO_CANONICALIZATION
            },
            accounting: accounting(
                PayloadAccounting::Recursive,
                match cardinality {
                    CardinalitySpec::Exact(_) => PopulationAccounting::ExactCardinality,
                    CardinalitySpec::Dynamic { .. } => PopulationAccounting::ValueCardinality,
                },
                AuxiliaryAccounting {
                    ordered_index: true,
                    ..NO_AUXILIARY
                },
            ),
        },
        SchemaBody::ReifiedType => TypeMemoryContract {
            topology: MemoryTopology::ReifiedType,
            extent: MemoryExtent::Single,
            extent_evolution: ExtentEvolution::Fixed,
            addressing: NO_ADDRESSING,
            canonicalization: CanonicalizationContract {
                self_describing: true,
                ..NO_CANONICALIZATION
            },
            accounting: accounting(
                PayloadAccounting::SelfDescribing,
                PopulationAccounting::Single,
                NO_AUXILIARY,
            ),
        },
    };

    Ok(contract)
}

fn resolve_extent(
    extent: &MemoryExtent,
    parameters: &[DimensionParameter],
    shape: &ShapeInstance,
) -> Result<ResolvedMemoryExtent, SemanticModelError> {
    match extent {
        MemoryExtent::Single => Ok(ResolvedMemoryExtent::Single),
        MemoryExtent::FixedArity(arity) => Ok(ResolvedMemoryExtent::FixedArity(*arity)),
        MemoryExtent::Dimensions(dimensions) => {
            let mut resolved = Vec::with_capacity(dimensions.len());
            for dimension in dimensions {
                resolved.push(ResolvedAxisExtent {
                    value: shape.resolve_dimension(dimension)?,
                    evolution: dimension_evolution(dimension, parameters)?,
                });
            }
            Ok(ResolvedMemoryExtent::Dimensions(
                resolved.into_boxed_slice(),
            ))
        }
        MemoryExtent::Cardinality(CardinalitySpec::Exact(expression)) => Ok(
            ResolvedMemoryExtent::ExactCardinality(shape.resolve_dimension(expression)?),
        ),
        MemoryExtent::Cardinality(CardinalitySpec::Dynamic { upper_bound }) => {
            Ok(ResolvedMemoryExtent::DynamicCardinality {
                upper_bound: upper_bound
                    .as_ref()
                    .map(|bound| shape.resolve_dimension(bound))
                    .transpose()?,
            })
        }
    }
}

pub(crate) fn resolve_type_memory_contract(
    contract: TypeMemoryContract,
    parameters: &[DimensionParameter],
    shape: &ShapeInstance,
) -> Result<ResolvedTypeMemoryContract, SemanticModelError> {
    Ok(ResolvedTypeMemoryContract {
        extent: resolve_extent(&contract.extent, parameters, shape)?,
        topology: contract.topology,
        extent_evolution: contract.extent_evolution,
        addressing: contract.addressing,
        canonicalization: contract.canonicalization,
        accounting: contract.accounting,
    })
}
