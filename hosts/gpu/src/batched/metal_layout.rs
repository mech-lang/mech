//! Component-major storage for direct Metal execution.
//!
//! Scalar instructions, state update operands, and integrity predicates are
//! emitted from the same lowered kernel as the CPU and portable GPU paths.

use std::collections::{BTreeMap, BTreeSet};

use mech_core::CellSlotId;

use super::{
    BatchedConstraint, BatchedInput, BatchedState, FixedShapeKernel, ScalarComputation,
    ScalarInstruction, ScalarOperand, scalar_computation_wgsl, scalar_operand_wgsl,
    scalar_predicate_wgsl,
};
use crate::WORKGROUP_SIZE;

impl FixedShapeKernel {
    pub(super) fn component_major_wgsl(&self, broadcast_inputs: &BTreeSet<CellSlotId>) -> String {
        generate_component_major_wgsl(
            self.instances,
            &self.register_offsets,
            &self.fixed_ir().instructions,
            &self.inputs,
            &self.states,
            &self.constraints,
            true,
            Some(broadcast_inputs),
        )
    }
}

/// Broadcast ports are compact, since their generated indices are constants.
/// Array ports are transposed to one contiguous plane per component.
pub(super) fn pack_input_values(
    original: &[f32],
    expanded: &[f32],
    elements: usize,
    instances: usize,
) -> Vec<f32> {
    if original.len() == elements {
        original.to_vec()
    } else {
        component_major_values(expanded, elements, instances)
    }
}

fn metal_computation_wgsl(computation: &ScalarComputation) -> String {
    match computation {
        ScalarComputation::SumProducts(terms) => metal_sum_products_wgsl(terms),
        _ => scalar_computation_wgsl(computation),
    }
}

/// Preserve the direct Metal lowering used by benchmark revision 45a21a62d:
/// accumulate products with explicit FMA and remove factors of 1 and -1.
/// Zero factors remain, since eliminating 0 * NaN or 0 * infinity would hide
/// non-finite candidates from source integrity predicates. This arithmetic
/// policy is identical for checked and unchecked kernels.
fn metal_sum_products_wgsl(terms: &[(ScalarOperand, ScalarOperand)]) -> String {
    let mut expression = "0.0".to_owned();
    for (left, right) in terms {
        if matches!(left, ScalarOperand::Constant(value) if *value == 1.0) {
            expression = format!("({expression}) + ({})", scalar_operand_wgsl(*right));
        } else if matches!(right, ScalarOperand::Constant(value) if *value == 1.0) {
            expression = format!("({expression}) + ({})", scalar_operand_wgsl(*left));
        } else if matches!(left, ScalarOperand::Constant(value) if *value == -1.0) {
            expression = format!("({expression}) - ({})", scalar_operand_wgsl(*right));
        } else if matches!(right, ScalarOperand::Constant(value) if *value == -1.0) {
            expression = format!("({expression}) - ({})", scalar_operand_wgsl(*left));
        } else {
            expression = format!(
                "fma({}, {}, {expression})",
                scalar_operand_wgsl(*left),
                scalar_operand_wgsl(*right),
            );
        }
    }
    expression
}

fn generate_component_major_wgsl(
    instances: u32,
    register_offsets: &BTreeMap<CellSlotId, usize>,
    instructions: &[ScalarInstruction],
    inputs: &[BatchedInput],
    states: &[BatchedState],
    constraints: &[BatchedConstraint],
    component_major: bool,
    broadcast_inputs: Option<&BTreeSet<CellSlotId>>,
) -> String {
    let mut shader = String::from("// Generic fixed-shape Mech batch kernel.\n");
    for input in inputs {
        shader.push_str(&format!(
            "@group(0) @binding({}) var<storage, read> input_{}: array<f32>;\n",
            input.binding,
            input.slot.get()
        ));
    }
    for state in states {
        shader.push_str(&format!(
            "@group(0) @binding({}) var<storage, read> state_read_{}: array<f32>;\n",
            state.read_binding,
            state.slot.get()
        ));
        shader.push_str(&format!(
            "@group(0) @binding({}) var<storage, read_write> state_write_{}: array<f32>;\n",
            state.write_binding,
            state.slot.get()
        ));
    }
    if !constraints.is_empty() {
        let binding = inputs.len() as u32 + states.len() as u32 * 2;
        shader.push_str(&format!(
            "@group(0) @binding({binding}) var<storage, read_write> integrity_fault: array<atomic<u32>>;\n\n\
             fn record_integrity_fault(code: u32, instance: u32) {{\n\
               atomicAdd(&integrity_fault[0], 1u);\n\
               atomicMin(&integrity_fault[1], (instance << 8u) | code);\n\
             }}\n"
        ));
    }
    shader.push_str(&format!(
        "\n@compute @workgroup_size({WORKGROUP_SIZE})\nfn main(@builtin(global_invocation_id) gid: vec3<u32>) {{\n  let index = gid.x;\n  if (index >= {instances}u) {{ return; }}\n"
    ));
    for input in inputs {
        let offset = register_offsets[&input.slot];
        for component in 0..input.shape.elements() {
            let index = if component_major
                && broadcast_inputs.is_some_and(|inputs| inputs.contains(&input.slot))
            {
                format!("{component}u")
            } else if component_major {
                format!("{component}u * {instances}u + index")
            } else {
                format!("index * {}u + {component}u", input.shape.elements())
            };
            shader.push_str(&format!(
                "  let r{} = input_{}[{index}];\n",
                offset + component,
                input.slot.get(),
            ));
        }
    }
    for state in states {
        let offset = register_offsets[&state.slot];
        for component in 0..state.shape.elements() {
            let index = if component_major {
                format!("{component}u * {instances}u + index")
            } else {
                format!("index * {}u + {component}u", state.shape.elements())
            };
            shader.push_str(&format!(
                "  let r{} = state_read_{}[{index}];\n",
                offset + component,
                state.slot.get(),
            ));
        }
    }
    for instruction in instructions {
        shader.push_str(&format!(
            "  let r{} = {};\n",
            instruction.output,
            metal_computation_wgsl(&instruction.computation)
        ));
    }
    if !constraints.is_empty() {
        shader.push_str("  var integrity_code = 0u;\n");
        for (index, constraint) in constraints.iter().enumerate() {
            let code = index + 1;
            shader.push_str(&format!(
                "  if (integrity_code == 0u && !{}) {{ integrity_code = {code}u; }}\n",
                scalar_predicate_wgsl(&constraint.predicate)
            ));
        }
        shader.push_str(
            "  if (integrity_code != 0u) { record_integrity_fault(integrity_code, index); }\n",
        );
    }
    for state in states {
        for (component, source) in state.update.iter().enumerate() {
            let index = if component_major {
                format!("{component}u * {instances}u + index")
            } else {
                format!("index * {}u + {component}u", state.shape.elements())
            };
            shader.push_str(&format!(
                "  state_write_{}[{index}] = {};\n",
                state.slot.get(),
                scalar_operand_wgsl(*source)
            ));
        }
    }
    shader.push_str("}\n");
    shader
}

pub(super) fn component_major_values(
    values: &[f32],
    elements: usize,
    instances: usize,
) -> Vec<f32> {
    assert_eq!(values.len(), elements * instances);
    let mut result = vec![0.0; values.len()];
    for instance in 0..instances {
        for component in 0..elements {
            result[component * instances + instance] = values[instance * elements + component];
        }
    }
    result
}

pub(super) fn instance_major_values(values: &[f32], elements: usize, instances: usize) -> Vec<f32> {
    assert_eq!(values.len(), elements * instances);
    let mut result = vec![0.0; values.len()];
    for instance in 0..instances {
        for component in 0..elements {
            result[instance * elements + component] = values[component * instances + instance];
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::super::{ComparisonOperation, FixedShape, ScalarOperand, ScalarPredicate};
    use super::*;
    use mech_core::IntegrityConstraintId;

    #[test]
    fn component_major_layout_round_trips_multicomponent_instances() {
        let logical = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let packed = component_major_values(&logical, 2, 3);
        assert_eq!(packed, [1.0, 3.0, 5.0, 2.0, 4.0, 6.0]);
        assert_eq!(instance_major_values(&packed, 2, 3), logical);
    }

    #[test]
    fn broadcast_vectors_remain_compact_and_array_vectors_use_component_planes() {
        let broadcast = [10.0, 20.0];
        let expanded = [10.0, 20.0, 10.0, 20.0, 10.0, 20.0];
        assert_eq!(pack_input_values(&broadcast, &expanded, 2, 3), broadcast);
        let array = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        assert_eq!(
            pack_input_values(&array, &array, 2, 3),
            [1.0, 3.0, 5.0, 2.0, 4.0, 6.0],
        );
    }

    #[test]
    fn checked_and_unchecked_shaders_keep_the_same_state_layout() {
        let input_slot = CellSlotId::new(0);
        let state_slot = CellSlotId::new(1);
        let offsets = BTreeMap::from([(input_slot, 0), (state_slot, 2)]);
        let inputs = [BatchedInput {
            slot: input_slot,
            name: "input".to_owned(),
            shape: FixedShape {
                rows: 2,
                columns: 1,
            },
            binding: 0,
        }];
        let states = [BatchedState {
            slot: state_slot,
            shape: FixedShape {
                rows: 2,
                columns: 1,
            },
            initializer: vec![0.0, 0.0],
            update: vec![ScalarOperand::Register(4), ScalarOperand::Register(1)],
            read_binding: 1,
            write_binding: 2,
        }];
        let constraint = BatchedConstraint {
            id: IntegrityConstraintId::new(0),
            name: "positive".into(),
            predicate: ScalarPredicate::Compare {
                operation: ComparisonOperation::Greater,
                left: ScalarOperand::Register(0),
                right: ScalarOperand::Constant(0.0),
            },
        };
        let instructions = [ScalarInstruction {
            output: 4,
            computation: ScalarComputation::SumProducts(vec![
                (ScalarOperand::Constant(1.0), ScalarOperand::Register(0)),
                (ScalarOperand::Register(1), ScalarOperand::Constant(1.0)),
                (ScalarOperand::Constant(0.0), ScalarOperand::Register(0)),
                (ScalarOperand::Register(0), ScalarOperand::Register(1)),
            ]),
        }];
        let broadcast = BTreeSet::from([input_slot]);
        let checked = generate_component_major_wgsl(
            3,
            &offsets,
            &instructions,
            &inputs,
            &states,
            &[constraint],
            true,
            Some(&broadcast),
        );
        let unchecked = generate_component_major_wgsl(
            3,
            &offsets,
            &instructions,
            &inputs,
            &states,
            &[],
            true,
            Some(&broadcast),
        );
        for source in [&checked, &unchecked] {
            assert!(source.contains("input_0[0u]"));
            assert!(source.contains("input_0[1u]"));
            assert!(source.contains("state_read_1[1u * 3u + index]"));
            assert!(source.contains("state_write_1[1u * 3u + index]"));
            // Both modes use FMA and identity-factor elimination, while a
            // zero product remains present for non-finite propagation.
            assert!(source.contains("let r4 = fma(r0, r1, fma(0.0, r0, ((0.0) + (r0)) + (r1)));"));
            let module = naga::front::wgsl::parse_str(source).unwrap();
            naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::all(),
            )
            .validate(&module)
            .unwrap();
        }
        assert!(checked.contains("record_integrity_fault"));
        assert!(!unchecked.contains("integrity_fault"));
    }

    #[test]
    fn metal_sum_products_elides_signed_identity_but_retains_zero() {
        let expression = metal_sum_products_wgsl(&[
            (ScalarOperand::Constant(-1.0), ScalarOperand::Register(0)),
            (ScalarOperand::Register(1), ScalarOperand::Constant(-1.0)),
            (ScalarOperand::Register(2), ScalarOperand::Constant(0.0)),
        ]);
        assert_eq!(expression, "fma(r2, 0.0, ((0.0) - (r0)) - (r1))");
        assert_eq!(metal_sum_products_wgsl(&[]), "0.0");
    }
}
