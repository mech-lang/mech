use crate::LegacyValue;
#[cfg(feature = "enum")]
use crate::{EnumDefine, InterpreterExecution, MResult, MechEnum, Ref, kind_annotation};
#[cfg(all(feature = "enum", feature = "atom"))]
use crate::{OperationId, ValueKind};

#[cfg(feature = "enum")]
pub fn enum_define(enm_def: &EnumDefine, p: &InterpreterExecution<'_>) -> MResult<()> {
    let id = enm_def.name.hash();
    let mut variants: Vec<(u64, Option<LegacyValue>)> = Vec::new();
    {
        let mut state_brrw = p.state.borrow_mut();
        for v in &enm_def.variants {
            let payload = match &v.value {
                Some(kind_annotation_node) => {
                    let knd = kind_annotation(&kind_annotation_node.kind, p)?;
                    let vk = knd.to_value_kind(&mut state_brrw.kinds)?;
                    Some(LegacyValue::Kind(vk))
                }
                None => None,
            };
            variants.push((v.name.hash(), payload));
        }
    }
    let state = &p.state;
    let mut state_brrw = state.borrow_mut();
    let dictionary = state_brrw.dictionary.clone();
    {
        let mut dictionary_brrw = dictionary.borrow_mut();
        dictionary_brrw.insert(enm_def.name.hash(), enm_def.name.to_string());
        for variant in &enm_def.variants {
            dictionary_brrw.insert(variant.name.hash(), variant.name.to_string());
        }
    }
    let enm = MechEnum {
        id,
        variants,
        names: dictionary,
    };
    let val = LegacyValue::Enum(Ref::new(enm.clone()));
    state_brrw.enums.insert(id, enm.clone());
    state_brrw.kinds.insert(id, val.kind());
    Ok(())
}

#[cfg(all(feature = "enum", feature = "atom"))]
pub(super) fn value_matches_enum_variant(
    value: &LegacyValue,
    enum_id: u64,
    p: &InterpreterExecution<'_>,
) -> bool {
    let my_enum = match p.state.borrow().enums.get(&enum_id).cloned() {
        Some(enm) => enm,
        None => return false,
    };
    let names_brrw = my_enum.names.borrow();
    let atom_matches_variant = |variant_id: u64, atom_id: u64, atom_name: &str| {
        if variant_id == atom_id {
            return true;
        }
        let variant_name = match names_brrw.get(&variant_id) {
            Some(name) => name.as_str(),
            None => return false,
        };
        let short_variant = variant_name.rsplit('/').next().unwrap_or(variant_name);
        let short_atom = atom_name.rsplit('/').next().unwrap_or(atom_name);
        short_variant == short_atom
    };
    match value {
        LegacyValue::Enum(enum_value) => {
            let enum_value_brrw = enum_value.borrow();
            if enum_value_brrw.id != enum_id {
                return false;
            }
            if enum_value_brrw.variants.len() != 1 {
                return false;
            }
            let (variant_id, payload) = &enum_value_brrw.variants[0];
            let (_, declared_payload_kind) = match my_enum
                .variants
                .iter()
                .find(|(known_variant, _)| *known_variant == *variant_id)
            {
                Some(v) => v,
                None => return false,
            };
            match (payload, declared_payload_kind) {
                (None, None) => true,
                (Some(payload_value), Some(LegacyValue::Kind(expected_kind))) => {
                    match expected_kind {
                        ValueKind::Enum(inner_enum_id, _) => {
                            value_matches_enum_variant(payload_value, *inner_enum_id, p)
                        }
                        _ => {
                            payload_value.kind() == expected_kind.clone()
                                || p.specialize_visible_operation_named(
                                    OperationId::from_name("convert/kind"),
                                    Some("convert/kind"),
                                    &[
                                        payload_value.clone(),
                                        LegacyValue::Kind(expected_kind.clone()),
                                    ],
                                )
                                .is_ok()
                        }
                    }
                }
                _ => false,
            }
        }
        LegacyValue::Atom(atom_variant) => {
            let atom_brrw = atom_variant.borrow();
            let variant_id = atom_brrw.id();
            let atom_name = atom_brrw.name();
            my_enum
                .variants
                .iter()
                .any(|(known_variant, payload_kind)| {
                    atom_matches_variant(*known_variant, variant_id, &atom_name)
                        && payload_kind.is_none()
                })
        }
        #[cfg(feature = "tuple")]
        LegacyValue::Tuple(tuple_val) => {
            let tuple_brrw = tuple_val.borrow();
            if tuple_brrw.elements.len() != 2 {
                return false;
            }
            let variant_atom = match tuple_brrw.elements[0].as_ref() {
                LegacyValue::Atom(atom) => atom.borrow(),
                _ => return false,
            };
            let variant_id = variant_atom.id();
            let atom_name = variant_atom.name();
            let payload = tuple_brrw.elements[1].as_ref();
            let (_, declared_payload_kind) =
                match my_enum.variants.iter().find(|(known_variant, _)| {
                    atom_matches_variant(*known_variant, variant_id, &atom_name)
                }) {
                    Some(v) => v,
                    None => return false,
                };
            match declared_payload_kind {
                Some(LegacyValue::Kind(expected_kind)) => match expected_kind {
                    ValueKind::Enum(inner_enum_id, _) => {
                        value_matches_enum_variant(payload, *inner_enum_id, p)
                    }
                    _ => {
                        payload.kind() == expected_kind.clone()
                            || p.specialize_visible_operation_named(
                                OperationId::from_name("convert/kind"),
                                Some("convert/kind"),
                                &[payload.clone(), LegacyValue::Kind(expected_kind.clone())],
                            )
                            .is_ok()
                    }
                },
                _ => false,
            }
        }
        _ => false,
    }
}
