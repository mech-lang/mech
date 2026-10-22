//! Existing value gates required by the complete declared schema closure.
use mech_core::{FloatWidth, IntegerInterval, IntegerWidth, SchemaBody, SchemaTable};
use std::collections::BTreeSet;

pub(super) struct Requirements {
    pub features: BTreeSet<&'static str>,
    pub open_types: bool,
}

pub(super) fn required(schemas: &SchemaTable) -> Requirements {
    from_bodies(schemas.entries().map(|entry| entry.schema().body()))
}

fn integer(width: IntegerWidth, signed: bool) -> &'static str {
    use IntegerWidth::*;
    match (signed, width) {
        (false, W8) => "u8",
        (false, W16) => "u16",
        (false, W32) => "u32",
        (false, W64) => "u64",
        (false, W128) => "u128",
        (true, W8) => "i8",
        (true, W16) => "i16",
        (true, W32) => "i32",
        (true, W64) => "i64",
        (true, W128) => "i128",
    }
}

fn from_bodies<'a>(bodies: impl Iterator<Item = &'a SchemaBody>) -> Requirements {
    let mut pending: Vec<_> = bodies.collect();
    let mut features = BTreeSet::new();
    let mut open_types = false;
    while let Some(body) = pending.pop() {
        let feature = match body {
            SchemaBody::Bool => "bool",
            SchemaBody::UnsignedInteger(width)
            | SchemaBody::IntegerInterval(IntegerInterval::Unsigned { width, .. }) => {
                integer(*width, false)
            }
            SchemaBody::SignedInteger(width)
            | SchemaBody::IntegerInterval(IntegerInterval::Signed { width, .. }) => {
                integer(*width, true)
            }
            SchemaBody::FloatingPoint(FloatWidth::W32) => "f32",
            SchemaBody::FloatingPoint(FloatWidth::W64) => "f64",
            SchemaBody::Complex(width) => {
                features.insert(match width {
                    FloatWidth::W32 => "f32",
                    FloatWidth::W64 => "f64",
                });
                "complex"
            }
            SchemaBody::Rational64 => "r64",
            SchemaBody::String => "string",
            SchemaBody::Atom(_) => "atom",
            SchemaBody::Enum { variants, .. } => {
                pending.extend(
                    variants
                        .iter()
                        .filter_map(|variant| variant.payload.as_ref()),
                );
                "enum"
            }
            SchemaBody::Option(payload) => {
                pending.push(payload);
                continue;
            }
            SchemaBody::Tuple(fields) => {
                pending.extend(fields.iter());
                "tuple"
            }
            SchemaBody::Record(fields) => {
                pending.extend(fields.iter().map(|field| &field.schema));
                "record"
            }
            SchemaBody::Matrix { element, .. } => {
                pending.push(element);
                "matrixd"
            }
            SchemaBody::Table { columns, .. } => {
                pending.extend(columns.iter().map(|column| &column.schema));
                "table"
            }
            SchemaBody::Set { element, .. } => {
                pending.push(element);
                "set"
            }
            SchemaBody::Map { key, value, .. } => {
                pending.extend([key.as_ref(), value.as_ref()]);
                "map"
            }
            SchemaBody::Dynamic | SchemaBody::ReifiedType => {
                open_types = true;
                continue;
            }
            // These canonical representations have no existing Cargo gate.
            SchemaBody::Id | SchemaBody::Index => {
                continue;
            }
        };
        features.insert(feature);
    }
    Requirements {
        features,
        open_types,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_payloads_supply_their_own_gates() {
        let body = SchemaBody::Tuple(
            vec![
                SchemaBody::UnsignedInteger(IntegerWidth::W128),
                SchemaBody::Option(Box::new(SchemaBody::Complex(FloatWidth::W32))),
                SchemaBody::Rational64,
            ]
            .into_boxed_slice(),
        );
        assert_eq!(
            from_bodies(std::iter::once(&body)).features,
            BTreeSet::from(["complex", "f32", "r64", "tuple", "u128"])
        );
    }

    #[test]
    fn signed_cells_are_selected_only_by_signed_schema_requirements() {
        let unsigned = SchemaBody::UnsignedInteger(IntegerWidth::W64);
        assert_eq!(
            from_bodies(std::iter::once(&unsigned)).features,
            BTreeSet::from(["u64"]),
        );
        let signed = SchemaBody::Option(Box::new(SchemaBody::SignedInteger(IntegerWidth::W64)));
        assert_eq!(
            from_bodies(std::iter::once(&signed)).features,
            BTreeSet::from(["i64"]),
        );
    }

    #[test]
    fn dynamic_nested_payloads_keep_complete_conversion_support() {
        let body = SchemaBody::Option(Box::new(SchemaBody::Dynamic));
        assert!(from_bodies(std::iter::once(&body)).open_types);
        assert!(from_bodies(std::iter::once(&SchemaBody::ReifiedType)).open_types);
    }
}
