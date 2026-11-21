//! Specialize a byte histogram only under the existing numeric caller's
//! ownership/current-view gates and a new whole-span guard before source work.
use super::*;

pub(super) fn prepare<M: Module>(
    module: &mut M,
    context: &mut Context,
    proof: &Proof,
    region: &Region,
    header: Block,
    input_words: usize,
    symbol: &str,
    signature: &Signature,
    bodies: Option<&mut PhaseBodies>,
) -> Result<Option<serde_json::Value>, NativeCompileError> {
    let Some(loads) = phase_storage::input_word_loads(&context.func, input_words) else {
        return Ok(None);
    };
    let packet = numeric_minimal::input_words(proof, region.tail.carrier().get())?;
    let word_at = |word| packet.iter().position(|&at| at == word);
    let mut inputs = Vec::new();
    let mut bins = Vec::new();
    for (array, definition) in proof.arrays.iter().enumerate() {
        let destination = match (definition.width, definition.writable) {
            (1, false) => &mut inputs,
            (4, true) => &mut bins,
            _ => continue,
        };
        if let (Some(pointer_word), Some(length_word)) =
            (word_at(proof.view(array)), word_at(proof.view(array) + 1))
        {
            destination.push(View {
                array,
                pointer_word,
                length_word,
                width: definition.width,
                literal: None,
            });
        }
    }
    if inputs.is_empty() || bins.is_empty() {
        return Ok(None);
    }
    let mut specialized = module.make_context();
    specialized.func = context.func.clone();
    let grouped = std::env::var_os("MECH_HISTOGRAM_GROUP_BYTE_VISITS").is_some();
    let vector_scan = std::env::var_os("MECH_HISTOGRAM_VECTOR_COUNTER_SCAN").is_some();
    let counter_capacity =
        std::env::var_os("MECH_HISTOGRAM_ENTRY_COUNTER_CAPACITY").and_then(|_| {
            if grouped {
                cfg_cleanup::bounded_byte_histogram::optimize_grouped_counter_capacity(
                    &mut specialized,
                    header,
                    &loads,
                    &inputs,
                    &bins,
                )
            } else {
                cfg_cleanup::bounded_byte_histogram::optimize_counter_capacity(
                    &mut specialized,
                    header,
                    &loads,
                    &inputs,
                    &bins,
                )
            }
        });
    let bounded_counters = counter_capacity.is_some();
    let Some(admission) = counter_capacity.or_else(|| {
        cfg_cleanup::bounded_byte_histogram::optimize(
            &mut specialized,
            header,
            &loads,
            &inputs,
            &bins,
        )
    }) else {
        return Ok(None);
    };
    specialized.verify(module.isa()).map_err(backend)?;
    let name = format!("{symbol}_bounded_byte_histogram");
    let id = module
        .declare_function(&name, Linkage::Local, signature)
        .map_err(backend)?;
    let inspection = std::env::var_os("MECH_INSPECT_PHASE_CLIF")
        .map(std::path::PathBuf::from)
        .filter(|path| path.file_stem().is_some_and(|stem| stem == name.as_str()));
    if let Some(path) = &inspection {
        std::fs::write(path, specialized.func.display().to_string()).map_err(backend)?;
        specialized.set_disasm(true);
    }
    let (id, target) = define_body(module, id, &name, &mut specialized, bodies)?;
    let machine = specialized
        .compiled_code()
        .ok_or_else(|| backend("bounded histogram lacks code"))?;
    if let Some(path) = inspection {
        let vcode = machine
            .vcode
            .as_ref()
            .ok_or_else(|| backend("bounded histogram lacks VCode"))?;
        std::fs::write(path.with_extension("vcode"), vcode).map_err(backend)?;
    }
    let mut result = serde_json::json!({
        "symbol": target,
        "admission": admission,
        "removed_bounds_per_visit": 4,
        "counter_overflow_retained": !bounded_counters,
        "code_bytes": machine.code_info().total_size,
        "stack_bytes": compiled_stack_bytes(&machine.buffer)?,
    });
    if bounded_counters && grouped {
        result["grouped_byte_visits"] = serde_json::json!(4);
    }
    if bounded_counters && vector_scan {
        result["vector_counter_scan"] = serde_json::json!(true);
    }
    let fast = module.declare_func_in_func(id, &mut context.func);
    if bounded_counters && vector_scan {
        cfg_cleanup::bounded_byte_histogram::guard_vector_counter_capacity(
            context, admission, fast,
        );
    } else if bounded_counters {
        cfg_cleanup::bounded_byte_histogram::guard_counter_capacity(context, admission, fast);
    } else {
        cfg_cleanup::bounded_byte_histogram::guard_entry(context, admission, fast);
    }
    context.verify(module.isa()).map_err(backend)?;
    Ok(Some(result))
}
