//! Bounded selected layout for one Map plus flat U64 result fields. This is
//! only a publication recipe; the caller validates the current owner token.
use super::*;

pub(super) struct ResultLayout<'a> {
    body: &'a SchemaBody,
    map: &'a SchemaBody,
    fields: usize,
    map_ordinal: usize,
}

impl<'a> ResultLayout<'a> {
    pub(super) fn new(
        schemas: &'a SchemaTable,
        schema: SchemaId,
        words: &[u64],
    ) -> Result<Self, Error> {
        let definition = schemas
            .get(schema)
            .ok_or(Error::Publication("missing schema"))?;
        if !definition.dimension_parameters().is_empty() {
            return Err(Error::Publication("parameterized output schema"));
        }
        let body = definition.body();
        if matches!(body, SchemaBody::Map { .. }) {
            if words.len() > 1 {
                return Err(Error::Publication("map result word count"));
            }
            return Ok(Self {
                body,
                map: body,
                fields: 0,
                map_ordinal: 0,
            });
        }
        let fields = match body {
            SchemaBody::Tuple(items) => items.len(),
            SchemaBody::Record(fields) => fields.len(),
            _ => return Err(Error::Publication("unsupported result schema")),
        };
        if fields == 0 || fields > 32 || words.len() != fields {
            return Err(Error::Publication("composite result word count"));
        }
        let mut selected = None;
        for ordinal in 0..fields {
            let field = match body {
                SchemaBody::Tuple(items) => &items[ordinal],
                SchemaBody::Record(fields) => &fields[ordinal].schema,
                _ => unreachable!(),
            };
            match field {
                SchemaBody::Map { .. } if selected.is_none() => selected = Some((ordinal, field)),
                SchemaBody::UnsignedInteger(IntegerWidth::W64) => {}
                _ => return Err(Error::Publication("result field kind/multiple maps")),
            }
        }
        let (map_ordinal, map) = selected.ok_or(Error::Publication("missing result map"))?;
        Ok(Self {
            body,
            map,
            fields,
            map_ordinal,
        })
    }

    pub(super) fn validate_map(&self, count: u64) -> Result<(), Error> {
        string_u64_body(self.map, count)
    }

    // Core retained_data_footprint counts one ValueData/node for each scalar
    // and aggregate. Replacing the Map root adds one aggregate and fields-1
    // scalars, hence exactly `fields` additional ValueDatas/nodes.
    pub(super) fn extra_nodes(&self) -> u64 {
        self.fields as u64
    }
    pub(super) fn extra_value_bytes(&self) -> Result<u64, Error> {
        mul(self.extra_nodes(), size_of::<ValueData>() as u64)
    }
    pub(super) fn extra_draft_bytes(&self) -> Result<u64, Error> {
        let slots = mul(self.extra_nodes(), size_of::<ValueDataDraft>() as u64)?;
        if let SchemaBody::Record(fields) = self.body {
            let names = fields
                .iter()
                .try_fold(0u64, |n, field| add(n, field.name.len() as u64))?;
            // Named input, pending Option slots and the ordered draft vector
            // coexist in Core's maintained order_named_values finalizer.
            add(
                add(
                    mul(slots, 3)?,
                    mul(
                        self.extra_nodes(),
                        (size_of::<mech_core::snapshot::NamedValueDraft>()
                            + size_of::<Option<mech_core::snapshot::NamedValueDraft>>())
                            as u64,
                    )?,
                )?,
                names,
            )
        } else {
            Ok(slots)
        }
    }
    pub(super) fn extra_finalization(&self) -> Result<u64, Error> {
        let nodes = mul(self.extra_nodes(), 8)?;
        if let SchemaBody::Record(fields) = self.body {
            let names = fields
                .iter()
                .try_fold(0u64, |n, field| add(n, field.name.len() as u64))?;
            // Core scans every remaining named candidate for every schema
            // field. Charge the complete bounded scan before finalization.
            add(
                nodes,
                mul(add(names, self.extra_nodes())?, self.extra_nodes())?,
            )
        } else {
            Ok(nodes)
        }
    }

    pub(super) fn wrap(&self, map: ValueDataDraft, words: &[u64]) -> Result<ValueDataDraft, Error> {
        if self.fields == 0 {
            return Ok(map);
        }
        let mut map = Some(map);
        let mut child = |ordinal, word| -> Result<ValueDataDraft, Error> {
            Ok(if ordinal == self.map_ordinal {
                map.take()
                    .ok_or(Error::Publication("duplicate result map"))?
            } else {
                ValueDataDraft::U64(word)
            })
        };
        Ok(match self.body {
            SchemaBody::Tuple(_) => {
                let mut values = exact_vec(self.fields)?;
                for (ordinal, &word) in words.iter().enumerate() {
                    values.push(child(ordinal, word)?);
                }
                ValueDataDraft::Tuple(values.into_boxed_slice())
            }
            SchemaBody::Record(fields) => {
                let mut values = exact_vec(self.fields)?;
                for (ordinal, (&word, field)) in words.iter().zip(fields.iter()).enumerate() {
                    let mut name = String::new();
                    name.try_reserve_exact(field.name.len())
                        .map_err(|_| Error::Allocation)?;
                    if name.capacity() != field.name.len() {
                        return Err(Error::Allocation);
                    }
                    name.push_str(&field.name);
                    values.push(mech_core::snapshot::NamedValueDraft {
                        name,
                        value: child(ordinal, word)?,
                    });
                }
                ValueDataDraft::Record(values.into_boxed_slice())
            }
            _ => return Err(Error::Publication("result wrapper kind")),
        })
    }
}
