#[cfg(feature = "enum")]
use crate::{
    CanonicalEnumDefinition, CanonicalEnumVariant, EnumDefine, InterpreterExecution, MResult,
};
#[cfg(all(feature = "enum", not(feature = "kind_annotation")))]
use crate::{FeatureNotEnabledError, MechError};

#[cfg(feature = "enum")]
pub fn enum_define(enm_def: &EnumDefine, p: &InterpreterExecution<'_>) -> MResult<()> {
    let id = enm_def.name.hash();
    let variants = enm_def
        .variants
        .iter()
        .map(|variant| {
            let payload = match &variant.value {
                #[cfg(feature = "kind_annotation")]
                Some(annotation) => Some(crate::structures::schema_body_from_kind(
                    &annotation.kind,
                    p,
                )?),
                #[cfg(not(feature = "kind_annotation"))]
                Some(_) => {
                    return Err(MechError::new(FeatureNotEnabledError, None).with_compiler_loc());
                }
                None => None,
            };
            Ok(CanonicalEnumVariant {
                id: variant.name.hash(),
                name: variant.name.to_string(),
                payload,
            })
        })
        .collect::<MResult<Vec<_>>>()?
        .into_boxed_slice();

    let name = enm_def.name.to_string();
    let definition = CanonicalEnumDefinition {
        id,
        name: name.clone(),
        variants,
    };
    let schema = crate::structures::enum_schema(&definition)?;
    let mut state = p.state.borrow_mut();
    {
        let mut dictionary = state.dictionary.borrow_mut();
        dictionary.insert(id, name);
        for variant in &definition.variants {
            dictionary.insert(variant.id, variant.name.clone());
        }
    }
    state.enums.insert(id, definition);
    state.kinds.insert(id, schema);
    Ok(())
}
