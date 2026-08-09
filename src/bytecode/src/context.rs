use core::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use mech_core::{
    ApplicationRequirement, BytecodeCompilerContext, BytecodeInstruction, BytecodeProgram,
    BytecodeRegisterIdentity, BytecodeValidationError, EncodedConstant, LegacyValue, MResult,
    MechError, ParsedProgram, Register, compare_application_requirements, compile_value_register,
    hash_str, write_bytecode,
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct CanonicalRequirement(ApplicationRequirement);

impl Ord for CanonicalRequirement {
    fn cmp(&self, other: &Self) -> Ordering {
        compare_application_requirements(&self.0, &other.0)
    }
}

impl PartialOrd for CanonicalRequirement {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Default)]
pub struct CompileCtx {
    reg_map: HashMap<BytecodeRegisterIdentity, Register>,
    symbols: BTreeMap<u64, Register>,
    symbol_ptrs: BTreeMap<u64, usize>,
    dictionary: BTreeMap<u64, String>,
    mutable_symbols: BTreeSet<u64>,
    pending_constants: Vec<EncodedConstant>,
    requirements: BTreeSet<CanonicalRequirement>,
    pending_requirements: Vec<ApplicationRequirement>,
    instructions: Vec<BytecodeInstruction>,
    next_register: Register,
}

impl CompileCtx {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Resolve an interpreted value to the register that carries its identity.
    ///
    /// Planned outputs reuse their producer register. A final value that was
    /// not part of the plan (for example, a trailing literal) is materialized
    /// exactly once so `Return` still represents the source block's result.
    pub fn resolve_value_register(&mut self, value: &LegacyValue) -> MResult<Register> {
        let fallback = std::ptr::from_ref(value).addr();
        compile_value_register(value, fallback, self)
    }

    pub fn finish(&mut self, return_register: Register) -> MResult<Vec<u8>> {
        if return_register >= self.next_register {
            return invalid(format!(
                "return register {return_register} is outside register count {}",
                self.next_register,
            ));
        }

        let requirements = self
            .requirements
            .iter()
            .map(|requirement| requirement.0.clone())
            .collect::<Vec<_>>();
        let requirement_remap = self
            .pending_requirements
            .iter()
            .map(|requirement| {
                requirements
                    .binary_search_by(|candidate| {
                        compare_application_requirements(candidate, requirement)
                    })
                    .map(|index| index as u32)
                    .map_err(|_| {
                        invalid::<()>("pending application requirement was not finalized")
                            .unwrap_err()
                    })
            })
            .collect::<MResult<Vec<_>>>()?;

        let mut instructions = self.instructions.clone();
        for instruction in &mut instructions {
            remap_instruction_requirement(instruction, &requirement_remap)?;
        }
        instructions.push(BytecodeInstruction::Return {
            src: return_register,
        });

        let program = BytecodeProgram {
            register_count: self.next_register,
            constants: self.pending_constants.clone(),
            symbols: self.symbols.clone(),
            mutable_symbols: self.mutable_symbols.clone(),
            instructions,
            dictionary: self.dictionary.clone(),
            requirements,
        };
        let bytes = write_bytecode(&program)?;
        ParsedProgram::from_bytes(&bytes)?;
        Ok(bytes)
    }

    fn register_for_identity(&mut self, identity: BytecodeRegisterIdentity) -> (Register, bool) {
        if let Some(&register) = self.reg_map.get(&identity) {
            return (register, false);
        }
        let register = self.next_register;
        self.next_register = self
            .next_register
            .checked_add(1)
            .expect("bytecode register space exhausted");
        self.reg_map.insert(identity, register);
        (register, true)
    }
}

impl BytecodeCompilerContext for CompileCtx {
    fn register_for_ptr_with_initialization_status(&mut self, pointer: usize) -> (Register, bool) {
        self.register_for_identity(BytecodeRegisterIdentity::Cell(pointer))
    }

    fn register_for_identity_with_initialization_status(
        &mut self,
        identity: &BytecodeRegisterIdentity,
    ) -> (Register, bool) {
        self.register_for_identity(identity.clone())
    }

    fn intern_constant(&mut self, constant: EncodedConstant) -> MResult<u32> {
        if let Some(index) = self
            .pending_constants
            .iter()
            .position(|candidate| candidate == &constant)
        {
            return u32::try_from(index)
                .map_err(|_| invalid::<()>("constant index exceeds u32").unwrap_err());
        }
        let index = u32::try_from(self.pending_constants.len())
            .map_err(|_| invalid::<()>("constant index exceeds u32").unwrap_err())?;
        self.pending_constants.push(constant);
        Ok(index)
    }

    fn define_symbol(
        &mut self,
        pointer: usize,
        register: Register,
        name: &str,
        mutable: bool,
    ) -> MResult<()> {
        if name.is_empty() {
            return invalid("bytecode symbol name must not be empty");
        }
        if register >= self.next_register {
            return invalid(format!(
                "symbol register {register} is outside register count {}",
                self.next_register,
            ));
        }

        let symbol_id = hash_str(name);
        if let Some(existing_name) = self.dictionary.get(&symbol_id) {
            if existing_name != name {
                return invalid(format!(
                    "bytecode symbol hash collision between {existing_name:?} and {name:?}",
                ));
            }
            if self.symbols.get(&symbol_id) != Some(&register)
                || self.symbol_ptrs.get(&symbol_id) != Some(&pointer)
                || self.mutable_symbols.contains(&symbol_id) != mutable
            {
                return invalid(format!(
                    "conflicting bytecode symbol definition for {name:?}",
                ));
            }
            return Ok(());
        }

        self.symbols.insert(symbol_id, register);
        self.symbol_ptrs.insert(symbol_id, pointer);
        self.dictionary.insert(symbol_id, name.to_owned());
        if mutable {
            self.mutable_symbols.insert(symbol_id);
        }
        Ok(())
    }

    fn intern_requirement(&mut self, requirement: ApplicationRequirement) -> MResult<u32> {
        if let Some(index) = self
            .pending_requirements
            .iter()
            .position(|candidate| candidate == &requirement)
        {
            return u32::try_from(index)
                .map_err(|_| invalid::<()>("requirement index exceeds u32").unwrap_err());
        }
        let index = u32::try_from(self.pending_requirements.len())
            .map_err(|_| invalid::<()>("requirement index exceeds u32").unwrap_err())?;
        self.requirements
            .insert(CanonicalRequirement(requirement.clone()));
        self.pending_requirements.push(requirement);
        Ok(index)
    }

    fn emit_const_load(&mut self, destination: Register, constant: u32) {
        self.instructions.push(BytecodeInstruction::ConstLoad {
            dst: destination,
            constant,
        });
    }

    fn emit_composite_pack(
        &mut self,
        destination: Register,
        template: u32,
        children: Vec<Register>,
    ) {
        self.instructions.push(BytecodeInstruction::CompositePack {
            dst: destination,
            template,
            children,
        });
    }

    fn emit_nullop(&mut self, function: u64, destination: Register) {
        self.instructions.push(BytecodeInstruction::RuntimeNullary {
            function,
            dst: destination,
        });
    }

    fn emit_unop(&mut self, function: u64, destination: Register, source: Register) {
        self.instructions.push(BytecodeInstruction::RuntimeUnary {
            function,
            dst: destination,
            src: source,
        });
    }

    fn emit_binop(&mut self, function: u64, destination: Register, lhs: Register, rhs: Register) {
        self.instructions.push(BytecodeInstruction::RuntimeBinary {
            function,
            dst: destination,
            lhs,
            rhs,
        });
    }

    fn emit_ternop(
        &mut self,
        function: u64,
        destination: Register,
        a: Register,
        b: Register,
        c: Register,
    ) {
        self.instructions.push(BytecodeInstruction::RuntimeTernary {
            function,
            dst: destination,
            a,
            b,
            c,
        });
    }

    fn emit_quadop(
        &mut self,
        function: u64,
        destination: Register,
        a: Register,
        b: Register,
        c: Register,
        d: Register,
    ) {
        self.instructions
            .push(BytecodeInstruction::RuntimeQuaternary {
                function,
                dst: destination,
                a,
                b,
                c,
                d,
            });
    }

    fn emit_varop(&mut self, function: u64, destination: Register, arguments: Vec<Register>) {
        self.instructions
            .push(BytecodeInstruction::RuntimeVariadic {
                function,
                dst: destination,
                arguments,
            });
    }

    fn emit_host_call(
        &mut self,
        requirement: u32,
        destination: Register,
        arguments: Vec<Register>,
    ) {
        self.instructions.push(BytecodeInstruction::HostCall {
            requirement,
            dst: destination,
            arguments,
        });
    }

    fn emit_resource_read(&mut self, requirement: u32, destination: Register) {
        self.instructions.push(BytecodeInstruction::ResourceRead {
            requirement,
            dst: destination,
        });
    }

    fn emit_resource_write(&mut self, requirement: u32, destination: Register, source: Register) {
        self.instructions.push(BytecodeInstruction::ResourceWrite {
            requirement,
            dst: destination,
            src: source,
        });
    }

    fn emit_resource_send(&mut self, requirement: u32, destination: Register, source: Register) {
        self.instructions.push(BytecodeInstruction::ResourceSend {
            requirement,
            dst: destination,
            src: source,
        });
    }
}

fn remap_instruction_requirement(
    instruction: &mut BytecodeInstruction,
    remap: &[u32],
) -> MResult<()> {
    let requirement = match instruction {
        BytecodeInstruction::HostCall { requirement, .. }
        | BytecodeInstruction::ResourceRead { requirement, .. }
        | BytecodeInstruction::ResourceWrite { requirement, .. }
        | BytecodeInstruction::ResourceSend { requirement, .. } => requirement,
        _ => return Ok(()),
    };
    *requirement = remap.get(*requirement as usize).copied().ok_or_else(|| {
        invalid::<()>("instruction requirement index is out of range").unwrap_err()
    })?;
    Ok(())
}

fn invalid<T>(reason: impl Into<String>) -> MResult<T> {
    Err(MechError::new(
        BytecodeValidationError {
            reason: reason.into(),
        },
        None,
    )
    .with_compiler_loc())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::{
        ExecutionHostFunctionRequest, ExecutionResourceRequest, ResourceDelivery, ResourceIntent,
        RuntimeType,
    };

    fn f64_constant(value: f64) -> EncodedConstant {
        EncodedConstant {
            runtime_type: RuntimeType::F64,
            alignment: 8,
            bytes: value.to_bits().to_le_bytes().to_vec(),
        }
    }

    fn allocate_registers(context: &mut CompileCtx, count: usize) -> Vec<Register> {
        (0..count)
            .map(|pointer| {
                context
                    .register_for_ptr_with_initialization_status(pointer)
                    .0
            })
            .collect()
    }

    fn initialize_registers(context: &mut CompileCtx, registers: &[Register]) -> u32 {
        let constant = context.intern_constant(f64_constant(0.0)).unwrap();
        for register in registers {
            context.emit_const_load(*register, constant);
        }
        constant
    }

    fn host_requirement(name: &str) -> ApplicationRequirement {
        ApplicationRequirement::HostFunction(ExecutionHostFunctionRequest {
            name: name.to_owned(),
        })
    }

    fn resource_requirement(base_uri: &str) -> ApplicationRequirement {
        ApplicationRequirement::Resource(ExecutionResourceRequest {
            base_uri: base_uri.to_owned(),
            path: "value".to_owned(),
            context_name: "ctx".to_owned(),
            operation: "read".to_owned(),
            intent: ResourceIntent::Read,
            delivery: ResourceDelivery::Snapshot,
        })
    }

    #[test]
    fn pointer_registers_are_initialized_once() {
        let mut context = CompileCtx::new();
        let (first, initializes_first) = context.register_for_ptr_with_initialization_status(100);
        let (same, initializes_same) = context.register_for_ptr_with_initialization_status(100);
        let (second, initializes_second) = context.register_for_ptr_with_initialization_status(200);

        assert_eq!(first, same);
        assert_ne!(first, second);
        assert!(initializes_first);
        assert!(!initializes_same);
        assert!(initializes_second);

        context.clear();
        assert_eq!(
            context.register_for_ptr_with_initialization_status(100),
            (0, true),
        );
    }

    #[test]
    fn typed_wrappers_do_not_share_registers_with_bare_values() {
        for typed_first in [false, true] {
            let scalar = mech_core::Ref::new(7.0);
            let bare = LegacyValue::F64(scalar.clone());
            let typed = LegacyValue::Typed(
                Box::new(LegacyValue::F64(scalar)),
                mech_core::ValueKind::Option(Box::new(mech_core::ValueKind::F64)),
            );
            let typed_clone = typed.clone();
            let mut context = CompileCtx::new();

            let (first, second) = if typed_first {
                (
                    context.resolve_value_register(&typed).unwrap(),
                    context.resolve_value_register(&bare).unwrap(),
                )
            } else {
                (
                    context.resolve_value_register(&bare).unwrap(),
                    context.resolve_value_register(&typed).unwrap(),
                )
            };
            assert_ne!(first, second);
            assert_eq!(
                context.resolve_value_register(&typed_clone).unwrap(),
                if typed_first { first } else { second },
            );

            let parsed = ParsedProgram::from_bytes(&context.finish(second).unwrap()).unwrap();
            assert_eq!(parsed.constants.len(), 2);
            assert!(
                parsed
                    .constants
                    .iter()
                    .any(|constant| parsed.types[constant.type_id as usize] == RuntimeType::F64)
            );
            assert!(parsed.constants.iter().any(|constant| {
                parsed.types[constant.type_id as usize]
                    == RuntimeType::Option(Box::new(RuntimeType::F64))
            }));
            assert_eq!(
                parsed
                    .instructions
                    .iter()
                    .filter(|instruction| matches!(
                        instruction,
                        BytecodeInstruction::ConstLoad { .. }
                    ))
                    .count(),
                1,
            );
            assert_eq!(
                parsed
                    .instructions
                    .iter()
                    .filter(|instruction| matches!(
                        instruction,
                        BytecodeInstruction::CompositePack { .. }
                    ))
                    .count(),
                1,
            );
        }
    }

    #[test]
    fn complete_typed_annotations_are_part_of_register_identity() {
        let scalar = mech_core::Ref::new(7.0);
        let inner = Box::new(BytecodeRegisterIdentity::Cell(scalar.id() as usize));
        let option_f64 = BytecodeRegisterIdentity::Typed {
            inner: inner.clone(),
            annotation: mech_core::ValueKind::Option(Box::new(mech_core::ValueKind::F64)),
        };
        let option_u64 = BytecodeRegisterIdentity::Typed {
            inner,
            annotation: mech_core::ValueKind::Option(Box::new(mech_core::ValueKind::U64)),
        };
        let mut context = CompileCtx::new();

        let f64_register = context
            .register_for_identity_with_initialization_status(&option_f64)
            .0;
        let u64_register = context
            .register_for_identity_with_initialization_status(&option_u64)
            .0;

        assert_ne!(f64_register, u64_register);
        assert_eq!(
            context
                .register_for_identity_with_initialization_status(&option_f64)
                .0,
            f64_register,
        );
    }

    #[test]
    fn mutable_references_reuse_their_producer_register() {
        for mutable_first in [false, true] {
            let scalar = mech_core::Ref::new(7.0);
            let bare = LegacyValue::F64(scalar.clone());
            let mutable =
                LegacyValue::MutableReference(mech_core::Ref::new(LegacyValue::F64(scalar)));
            let mut context = CompileCtx::new();

            let (first, second) = if mutable_first {
                (
                    context.resolve_value_register(&mutable).unwrap(),
                    context.resolve_value_register(&bare).unwrap(),
                )
            } else {
                (
                    context.resolve_value_register(&bare).unwrap(),
                    context.resolve_value_register(&mutable).unwrap(),
                )
            };

            assert_eq!(first, second);
            let parsed = ParsedProgram::from_bytes(&context.finish(second).unwrap()).unwrap();
            assert_eq!(parsed.constants.len(), 1);
        }
    }

    #[test]
    fn composite_values_are_lowered_from_child_registers() {
        let scalar = mech_core::Ref::new(7.0);
        let bare = LegacyValue::F64(scalar.clone());
        let tuple = LegacyValue::Tuple(mech_core::Ref::new(mech_core::MechTuple::from_vec(vec![
            LegacyValue::F64(scalar.clone()),
            LegacyValue::F64(scalar),
        ])));
        let mut context = CompileCtx::new();

        let scalar_register = context.resolve_value_register(&bare).unwrap();
        let tuple_register = context.resolve_value_register(&tuple).unwrap();
        let parsed = ParsedProgram::from_bytes(&context.finish(tuple_register).unwrap()).unwrap();

        assert!(parsed.instructions.iter().any(|instruction| matches!(
            instruction,
            BytecodeInstruction::CompositePack { dst, children, .. }
                if *dst == tuple_register
                    && children == &[scalar_register, scalar_register]
        )));
        assert!(!parsed.instructions.iter().any(|instruction| matches!(
            instruction,
            BytecodeInstruction::ConstLoad { dst, .. } if *dst == tuple_register
        )));
    }

    #[test]
    fn constants_are_interned_and_self_validated() {
        let mut context = CompileCtx::new();
        let register = allocate_registers(&mut context, 1)[0];
        let first = context.intern_constant(f64_constant(3.0)).unwrap();
        let duplicate = context.intern_constant(f64_constant(3.0)).unwrap();
        assert_eq!(first, duplicate);
        context.emit_const_load(register, first);

        let parsed = ParsedProgram::from_bytes(&context.finish(register).unwrap()).unwrap();
        assert_eq!(parsed.constants.len(), 1);
        assert_eq!(parsed.decode_constants().unwrap().len(), 1);
        assert_eq!(
            parsed.instructions,
            vec![
                BytecodeInstruction::ConstLoad {
                    dst: register,
                    constant: first,
                },
                BytecodeInstruction::Return { src: register },
            ],
        );
    }

    #[test]
    fn symbol_and_dictionary_order_is_deterministic() {
        fn compile(reverse: bool) -> Vec<u8> {
            let mut context = CompileCtx::new();
            let registers = allocate_registers(&mut context, 2);
            initialize_registers(&mut context, &registers);
            let definitions = [
                (10usize, registers[0], "alpha", false),
                (20usize, registers[1], "omega", true),
            ];
            for index in if reverse { [1, 0] } else { [0, 1] } {
                let (pointer, register, name, mutable) = definitions[index];
                context
                    .define_symbol(pointer, register, name, mutable)
                    .unwrap();
            }
            context.finish(registers[0]).unwrap()
        }

        assert_eq!(compile(false), compile(true));
    }

    #[test]
    fn conflicting_and_empty_symbols_are_rejected() {
        let mut context = CompileCtx::new();
        let registers = allocate_registers(&mut context, 2);
        assert!(context.define_symbol(1, registers[0], "", false).is_err());
        context
            .define_symbol(1, registers[0], "answer", false)
            .unwrap();
        assert!(
            context
                .define_symbol(2, registers[1], "answer", false)
                .is_err()
        );
    }

    #[test]
    fn requirements_are_canonicalized_and_instruction_indexes_are_remapped() {
        fn compile(resource_first: bool) -> Vec<u8> {
            let mut context = CompileCtx::new();
            let registers = allocate_registers(&mut context, 2);
            initialize_registers(&mut context, &registers);
            let host = host_requirement("cli/stdout");
            let resource = resource_requirement("context://input");
            let (host_id, resource_id) = if resource_first {
                let resource_id = context.intern_requirement(resource).unwrap();
                let host_id = context.intern_requirement(host).unwrap();
                (host_id, resource_id)
            } else {
                let host_id = context.intern_requirement(host).unwrap();
                let resource_id = context.intern_requirement(resource).unwrap();
                (host_id, resource_id)
            };
            context.emit_host_call(host_id, registers[0], Vec::new());
            context.emit_resource_read(resource_id, registers[1]);
            context.finish(registers[1]).unwrap()
        }

        let resource_first = compile(true);
        assert_eq!(resource_first, compile(false));
        let parsed = ParsedProgram::from_bytes(&resource_first).unwrap();
        assert!(matches!(
            parsed.requirements[0],
            ApplicationRequirement::HostFunction(_)
        ));
        assert!(matches!(
            parsed.instructions[2],
            BytecodeInstruction::HostCall { requirement: 0, .. }
        ));
        assert!(matches!(
            parsed.instructions[3],
            BytecodeInstruction::ResourceRead { requirement: 1, .. }
        ));
    }

    #[test]
    fn finish_appends_one_final_return_and_is_repeatable() {
        let mut context = CompileCtx::new();
        let register = allocate_registers(&mut context, 1)[0];
        let constant = initialize_registers(&mut context, &[register]);
        let first = context.finish(register).unwrap();
        let second = context.finish(register).unwrap();
        assert_eq!(first, second);

        let parsed = ParsedProgram::from_bytes(&first).unwrap();
        assert_eq!(
            parsed.instructions,
            vec![
                BytecodeInstruction::ConstLoad {
                    dst: register,
                    constant,
                },
                BytecodeInstruction::Return { src: register },
            ],
        );
    }

    #[test]
    fn all_instruction_shapes_round_trip() {
        let mut context = CompileCtx::new();
        let registers = allocate_registers(&mut context, 7);
        initialize_registers(&mut context, &registers);
        let host = context
            .intern_requirement(host_requirement("cli/stdout"))
            .unwrap();
        let resource = context
            .intern_requirement(resource_requirement("context://input"))
            .unwrap();

        context.emit_nullop(1, registers[0]);
        context.emit_unop(2, registers[1], registers[0]);
        context.emit_binop(3, registers[2], registers[0], registers[1]);
        context.emit_ternop(4, registers[3], registers[0], registers[1], registers[2]);
        context.emit_quadop(
            5,
            registers[4],
            registers[0],
            registers[1],
            registers[2],
            registers[3],
        );
        context.emit_varop(6, registers[5], registers[..5].to_vec());
        context.emit_host_call(host, registers[5], registers[..2].to_vec());
        context.emit_resource_read(resource, registers[5]);
        context.emit_resource_write(resource, registers[5], registers[0]);
        context.emit_resource_send(resource, registers[6], registers[5]);

        let parsed = ParsedProgram::from_bytes(&context.finish(registers[6]).unwrap()).unwrap();
        assert_eq!(parsed.instructions.len(), 18);
        assert_eq!(
            parsed.instructions.last(),
            Some(&BytecodeInstruction::Return { src: registers[6] }),
        );
    }

    #[test]
    fn finish_rejects_out_of_range_return_register() {
        assert!(CompileCtx::new().finish(0).is_err());
    }
}
