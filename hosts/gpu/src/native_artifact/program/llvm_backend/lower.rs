use super::recording::Definition;
use cranelift_codegen::ir::{self, InstructionData as D, Opcode as O};
use cranelift_module::{FuncId, Linkage, ModuleDeclarations};
use std::collections::{BTreeMap, BTreeSet};

type Result<T> = std::result::Result<T, String>;

fn ty(t: ir::Type) -> Result<String> {
    let lane = t.lane_type();
    let scalar = if lane.is_int() {
        format!("i{}", lane.bits())
    } else if lane == ir::types::F32 {
        "float".into()
    } else if lane == ir::types::F64 {
        "double".into()
    } else {
        return Err(format!("unsupported LLVM type {t}"));
    };
    if t.is_dynamic_vector() {
        return Err(format!("unsupported scalable LLVM type {t}"));
    }
    Ok(if t.is_vector() {
        format!("<{} x {scalar}>", t.lane_count())
    } else {
        scalar
    })
}

fn symbol(name: &str) -> Result<String> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_$.-".contains(&c))
    {
        return Err(format!("unsupported LLVM symbol {name:?}"));
    }
    Ok(format!("@\"{name}\""))
}

fn signature(sig: &ir::Signature) -> Result<(Vec<String>, String)> {
    calling_convention(sig)?;
    let param = |p: &ir::AbiParam| -> Result<String> {
        if p.purpose != ir::ArgumentPurpose::Normal || p.extension != ir::ArgumentExtension::None {
            return Err("unsupported LLVM special/extended ABI parameter".into());
        }
        ty(p.value_type)
    };
    let params = sig.params.iter().map(param).collect::<Result<Vec<_>>>()?;
    let returns = sig.returns.iter().map(param).collect::<Result<Vec<_>>>()?;
    // Apple AArch64 returns a pair of U64 words in x0/x1. This is the
    // outlined status/continuation ABI. Other multiple-return layouts decline.
    let ret = match returns.as_slice() {
        [] => "void".into(),
        [one] => one.clone(),
        [a, b] if a == "i64" && b == "i64" => "{ i64, i64 }".into(),
        _ => return Err("unsupported LLVM multiple-return ABI".into()),
    };
    Ok((params, ret))
}

fn calling_convention(sig: &ir::Signature) -> Result<&'static str> {
    match sig.call_conv {
        cranelift_codegen::isa::CallConv::AppleAarch64 => Ok(""),
        cranelift_codegen::isa::CallConv::PreserveAll if sig.returns.is_empty() => {
            Ok("preserve_allcc ")
        }
        _ => Err(format!(
            "unsupported LLVM calling convention {}",
            sig.call_conv
        )),
    }
}

pub(super) fn module<'a>(
    declarations: &ModuleDeclarations,
    definitions: impl Iterator<Item = &'a Definition>,
) -> Result<String> {
    let definitions = definitions.collect::<Vec<_>>();
    let ids = definitions
        .iter()
        .map(|d| d.id.as_u32())
        .collect::<BTreeSet<_>>();
    if declarations.get_data_objects().next().is_some() {
        return Err("LLVM data declarations require an explicit data lowering".into());
    }
    let mut imports = Vec::new();
    for (id, decl) in declarations.get_functions() {
        let name = symbol(decl.name.as_deref().ok_or("anonymous LLVM function")?)?;
        let (params, ret) = signature(&decl.signature)?;
        if !ids.contains(&id.as_u32()) {
            if decl.linkage != Linkage::Import {
                return Err(format!("missing LLVM definition {name}"));
            }
            imports.push(format!(
                "declare {}{ret} {name}({})",
                calling_convention(&decl.signature)?,
                params.join(", ")
            ));
        } else if decl.linkage == Linkage::Import {
            return Err(format!("defined LLVM import {name}"));
        }
    }
    let mut intrinsics = BTreeSet::new();
    let mut bodies = Vec::new();
    let retained = definitions
        .iter()
        .map(|d| {
            Ok(format!(
                "ptr {}",
                symbol(
                    declarations
                        .get_function_decl(d.id)
                        .name
                        .as_deref()
                        .ok_or("anonymous LLVM definition")?
                )?
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    for def in definitions {
        bodies.push(Function::new(&def.function, declarations, &mut intrinsics).emit(def.id)?);
    }
    Ok(format!(
        "; Mech verified SSA; not a native descriptor or an executable image.\n\
         target triple = \"arm64-apple-macosx26.0.0\"\n\n{}\n{}\n{}\n{}\n\
         attributes #0 = {{ noinline uwtable \"frame-pointer\"=\"all\" }}\n",
        imports.join("\n"),
        intrinsics.into_iter().collect::<Vec<_>>().join("\n"),
        format!(
            "@llvm.used = appending global [{} x ptr] [{}], section \"llvm.metadata\"",
            retained.len(),
            retained.join(", ")
        ),
        bodies.join("\n\n")
    ))
}

struct Edge {
    name: String,
    destination: ir::Block,
    values: Vec<ir::Value>,
}
struct Function<'a> {
    f: &'a ir::Function,
    declarations: &'a ModuleDeclarations,
    intrinsics: &'a mut BTreeSet<String>,
    edges: Vec<Edge>,
    branches: BTreeMap<ir::Inst, Vec<usize>>,
    output: String,
    temporary: u64,
}
impl<'a> Function<'a> {
    fn new(
        f: &'a ir::Function,
        declarations: &'a ModuleDeclarations,
        intrinsics: &'a mut BTreeSet<String>,
    ) -> Self {
        Self {
            f,
            declarations,
            intrinsics,
            edges: Vec::new(),
            branches: BTreeMap::new(),
            output: String::new(),
            temporary: 0,
        }
    }
    fn line(&mut self, text: impl AsRef<str>) {
        self.output.push_str("  ");
        self.output.push_str(text.as_ref());
        self.output.push('\n');
    }
    fn temporary(&mut self, text: impl AsRef<str>) -> String {
        let result = format!("%t{}", self.temporary);
        self.temporary += 1;
        self.line(format!("{result} = {}", text.as_ref()));
        result
    }
    fn value(&self, v: ir::Value) -> String {
        format!("%{}", self.f.dfg.resolve_aliases(v))
    }
    fn value_type(&self, v: ir::Value) -> ir::Type {
        self.f.dfg.value_type(self.f.dfg.resolve_aliases(v))
    }
    fn typed(&self, v: ir::Value) -> Result<String> {
        Ok(format!("{} {}", ty(self.value_type(v))?, self.value(v)))
    }
    fn truth(&mut self, v: ir::Value) -> Result<String> {
        if !self.value_type(v).is_int() || self.value_type(v).is_vector() {
            return Err("non-scalar LLVM branch condition".into());
        }
        Ok(self.temporary(format!("icmp ne {}, 0", self.typed(v)?)))
    }
    fn intrinsic(&mut self, name: &str, result: &str, args: &[(String, String)]) -> String {
        self.intrinsics.insert(format!(
            "declare {result} @{name}({})",
            args.iter()
                .map(|a| a.0.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
        self.temporary(format!(
            "call {result} @{name}({})",
            args.iter()
                .map(|(t, v)| format!("{t} {v}"))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
    fn function_name(&self, reference: ir::FuncRef) -> Result<String> {
        let external = &self.f.dfg.ext_funcs[reference];
        let ir::ExternalName::User(reference) = external.name else {
            return Err("LLVM non-module function reference".into());
        };
        let name = &self.f.params.user_named_funcs()[reference];
        if name.namespace != 0 {
            return Err("LLVM function reference uses data namespace".into());
        }
        let decl = self
            .declarations
            .get_function_decl(FuncId::from_u32(name.index));
        if self.f.dfg.signatures[external.signature] != decl.signature {
            return Err("LLVM module call signature differs".into());
        }
        symbol(decl.name.as_deref().ok_or("anonymous LLVM callee")?)
    }
    fn pointer(&mut self, base: ir::Value, offset: i64) -> Result<String> {
        if self.value_type(base) != ir::types::I64 {
            return Err("LLVM non-host pointer width".into());
        }
        let p = self.temporary(format!("inttoptr {} to ptr", self.typed(base)?));
        Ok(self.offset(p, offset))
    }
    fn offset(&mut self, p: String, offset: i64) -> String {
        if offset == 0 {
            p
        } else {
            self.temporary(format!("getelementptr i8, ptr {p}, i64 {offset}"))
        }
    }
    fn guard_trap(&mut self, bad: String) {
        let n = self.temporary;
        self.temporary += 1;
        self.line(format!("br i1 {bad}, label %trap{n}, label %ok{n}"));
        self.output.push_str(&format!("trap{n}:\n"));
        self.intrinsics.insert("declare void @llvm.trap()".into());
        self.line("call void @llvm.trap()");
        self.line("unreachable");
        self.output.push_str(&format!("ok{n}:\n"));
    }
    fn constant(t: ir::Type, n: i64) -> Result<String> {
        if !t.is_int() || t.is_vector() || t.bits() > 64 {
            return Err(format!("unsupported LLVM integer constant {t}"));
        }
        let bits = t.bits();
        Ok(if bits == 64 {
            (n as u64).to_string()
        } else {
            ((n as u64) & ((1u64 << bits) - 1)).to_string()
        })
    }
    fn mask_constant(t: ir::Type, n: u32) -> Result<String> {
        if !t.lane_type().is_int() {
            return Err("non-integer LLVM shift".into());
        }
        Ok(if t.is_vector() {
            format!(
                "<{}>",
                (0..t.lane_count())
                    .map(|_| format!("i{} {n}", t.lane_bits()))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        } else {
            n.to_string()
        })
    }
    fn emit(mut self, id: FuncId) -> Result<String> {
        let decl = self.declarations.get_function_decl(id);
        if self.f.signature != decl.signature {
            return Err("LLVM definition signature differs".into());
        }
        let (params, ret) = signature(&self.f.signature)?;
        let entry = self
            .f
            .layout
            .entry_block()
            .ok_or("LLVM function has no entry")?;
        if !self.f.dynamic_stack_slots.is_empty() || !self.f.global_values.is_empty() {
            return Err("unsupported LLVM dynamic stack/global values".into());
        }
        for block in self.f.layout.blocks() {
            for inst in self.f.layout.block_insts(block) {
                let mut indices = Vec::new();
                for call in self.f.dfg.insts[inst]
                    .branch_destination(&self.f.dfg.jump_tables, &self.f.dfg.exception_tables)
                {
                    let mut values = Vec::new();
                    for arg in call.args(&self.f.dfg.value_lists) {
                        let ir::BlockArg::Value(v) = arg else {
                            return Err("unsupported LLVM exception edge".into());
                        };
                        values.push(v);
                    }
                    let destination = call.block(&self.f.dfg.value_lists);
                    if destination == entry {
                        return Err("LLVM entry has a predecessor".into());
                    }
                    if values.len() != self.f.dfg.block_params(destination).len() {
                        return Err("LLVM edge argument count differs".into());
                    }
                    let index = self.edges.len();
                    indices.push(index);
                    self.edges.push(Edge {
                        name: format!("edge{index}"),
                        destination,
                        values,
                    });
                }
                if !indices.is_empty() {
                    self.branches.insert(inst, indices);
                }
            }
        }
        let entry_params = self.f.dfg.block_params(entry);
        if params
            != entry_params
                .iter()
                .map(|v| ty(self.value_type(*v)))
                .collect::<Result<Vec<_>>>()?
        {
            return Err("LLVM entry parameters differ".into());
        }
        let arguments = entry_params
            .iter()
            .map(|v| self.typed(*v))
            .collect::<Result<Vec<_>>>()?
            .join(", ");
        // Retain all functions for exact closed machine metadata. Localizing
        // private symbols belongs to the final Object assembly stage.
        self.output.push_str(&format!(
            "define {}{ret} {}({arguments}) #0 align 128 {{\n",
            calling_convention(&self.f.signature)?,
            symbol(decl.name.as_deref().ok_or("anonymous LLVM definition")?)?
        ));
        for block in self.f.layout.blocks() {
            self.output.push_str(&format!("{block}:\n"));
            if block == entry {
                for (slot, data) in self.f.sized_stack_slots.iter() {
                    if data.kind != ir::StackSlotKind::ExplicitSlot {
                        return Err("unsupported LLVM stack slot kind".into());
                    }
                    self.line(format!(
                        "%{slot} = alloca [{} x i8], align {}",
                        data.size,
                        1u64 << data.align_shift
                    ));
                }
            } else {
                for (i, param) in self.f.dfg.block_params(block).iter().enumerate() {
                    let mut arms = Vec::new();
                    for e in self.edges.iter().filter(|e| e.destination == block) {
                        if self.value_type(*param) != self.value_type(e.values[i]) {
                            return Err("LLVM phi type differs".into());
                        }
                        arms.push(format!("[ {}, %{} ]", self.value(e.values[i]), e.name));
                    }
                    if arms.is_empty() {
                        return Err("unreachable LLVM block has parameters".into());
                    }
                    self.line(format!(
                        "{} = phi {} {}",
                        self.value(*param),
                        ty(self.value_type(*param))?,
                        arms.join(", ")
                    ));
                }
            }
            for inst in self.f.layout.block_insts(block) {
                self.instruction(inst).map_err(|e| {
                    format!(
                        "{}:{block}:{inst}: {e}",
                        decl.name.as_deref().unwrap_or("?")
                    )
                })?;
            }
        }
        for e in &self.edges {
            self.output
                .push_str(&format!("{}:\n  br label %{}\n", e.name, e.destination));
        }
        self.output.push_str("}\n");
        Ok(self.output)
    }
    fn instruction(&mut self, inst: ir::Inst) -> Result<()> {
        let data = &self.f.dfg.insts[inst];
        let op = data.opcode();
        let args = self.f.dfg.inst_args(inst);
        let results = self.f.dfg.inst_results(inst);
        let result = results.first().copied();
        let dest = result.map(|v| self.value(v)).unwrap_or_default();
        let rt = result.map(|v| self.value_type(v));
        let t = rt.map(ty).transpose()?.unwrap_or_default();
        match data {
            D::UnaryImm {
                opcode: O::Iconst,
                imm,
            } => {
                self.line(format!(
                    "{dest} = add {t} 0, {}",
                    Self::constant(rt.unwrap(), imm.bits())?
                ));
            }
            D::UnaryIeee64 {
                opcode: O::F64const,
                imm,
            } => self.line(format!("{dest} = bitcast i64 {} to double", imm.bits())),
            D::UnaryIeee32 {
                opcode: O::F32const,
                imm,
            } => self.line(format!("{dest} = bitcast i32 {} to float", imm.bits())),
            D::Load {
                opcode,
                arg,
                offset,
                flags,
            } => {
                Self::memory_flags(*flags)?;
                let p = self.pointer(*arg, i64::from(*offset))?;
                let narrow = match opcode {
                    O::Uload8 | O::Sload8 => Some(8),
                    O::Uload16 | O::Sload16 => Some(16),
                    O::Uload32 | O::Sload32 => Some(32),
                    O::Load => None,
                    _ => return Err(format!("unsupported LLVM load {op}")),
                };
                if let Some(bits) = narrow {
                    let v = self.temporary(format!("load i{bits}, ptr {p}, align 1"));
                    let ext = if matches!(opcode, O::Sload8 | O::Sload16 | O::Sload32) {
                        "sext"
                    } else {
                        "zext"
                    };
                    self.line(format!("{dest} = {ext} i{bits} {v} to {t}"));
                } else {
                    self.line(format!("{dest} = load {t}, ptr {p}, align 1"));
                }
            }
            D::Store {
                opcode,
                args: [value, address],
                offset,
                flags,
            } => {
                Self::memory_flags(*flags)?;
                let p = self.pointer(*address, i64::from(*offset))?;
                let bits = match opcode {
                    O::Istore8 => Some(8),
                    O::Istore16 => Some(16),
                    O::Istore32 => Some(32),
                    O::Store => None,
                    _ => return Err(format!("unsupported LLVM store {op}")),
                };
                let value = if let Some(bits) = bits {
                    let v = self.temporary(format!("trunc {} to i{bits}", self.typed(*value)?));
                    format!("i{bits} {v}")
                } else {
                    self.typed(*value)?
                };
                self.line(format!("store {value}, ptr {p}, align 1"));
            }
            D::StackLoad {
                opcode,
                stack_slot,
                offset,
            } => {
                let p = self.offset(format!("%{stack_slot}"), i64::from(*offset));
                if *opcode == O::StackAddr {
                    self.line(format!("{dest} = ptrtoint ptr {p} to {t}"));
                } else if *opcode == O::StackLoad {
                    self.line(format!("{dest} = load {t}, ptr {p}, align 1"));
                } else {
                    return Err(format!("unsupported LLVM stack load {op}"));
                }
            }
            D::StackStore {
                opcode: O::StackStore,
                arg,
                stack_slot,
                offset,
            } => {
                let p = self.offset(format!("%{stack_slot}"), i64::from(*offset));
                self.line(format!("store {}, ptr {p}, align 1", self.typed(*arg)?));
            }
            D::IntCompare {
                cond, args: [a, b], ..
            } => self.compare(
                &dest,
                rt.unwrap(),
                &cond.to_string(),
                *a,
                self.value(*b),
                false,
            )?,
            D::IntCompareImm { cond, arg, imm, .. } => self.compare(
                &dest,
                rt.unwrap(),
                &cond.to_string(),
                *arg,
                Self::constant(self.value_type(*arg), imm.bits())?,
                false,
            )?,
            D::FloatCompare {
                cond, args: [a, b], ..
            } => self.compare(
                &dest,
                rt.unwrap(),
                &cond.to_string(),
                *a,
                self.value(*b),
                true,
            )?,
            D::FuncAddr {
                opcode: O::FuncAddr,
                func_ref,
            } => self.line(format!(
                "{dest} = ptrtoint ptr {} to {t}",
                self.function_name(*func_ref)?
            )),
            D::Call {
                opcode: O::Call,
                func_ref,
                ..
            } => {
                let sig = &self.f.dfg.signatures[self.f.dfg.ext_funcs[*func_ref].signature];
                self.call(sig, self.function_name(*func_ref)?, args, results)?;
            }
            D::CallIndirect {
                opcode: O::CallIndirect,
                sig_ref,
                ..
            } => {
                let address = self.pointer(args[0], 0)?;
                self.call(
                    &self.f.dfg.signatures[*sig_ref],
                    address,
                    &args[1..],
                    results,
                )?;
            }
            D::Jump {
                opcode: O::Jump, ..
            } => self.line(format!("br label %edge{}", self.branches[&inst][0])),
            D::Brif {
                opcode: O::Brif,
                arg,
                ..
            } => {
                let c = self.truth(*arg)?;
                let e = &self.branches[&inst];
                self.line(format!(
                    "br i1 {c}, label %edge{}, label %edge{}",
                    e[0], e[1]
                ));
            }
            D::BranchTable {
                opcode: O::BrTable,
                arg,
                ..
            } => {
                let e = &self.branches[&inst];
                let arms = e[1..]
                    .iter()
                    .enumerate()
                    .map(|(i, e)| {
                        format!("{} {i}, label %edge{e}", ty(self.value_type(*arg)).unwrap())
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                self.line(format!(
                    "switch {}, label %edge{} [ {arms} ]",
                    self.typed(*arg)?,
                    e[0]
                ));
            }
            D::MultiAry {
                opcode: O::Return, ..
            } => match args {
                [] => self.line("ret void"),
                [v] => self.line(format!("ret {}", self.typed(*v)?)),
                [a, b] => {
                    let ret = signature(&self.f.signature)?.1;
                    let first =
                        self.temporary(format!("insertvalue {ret} poison, {}, 0", self.typed(*a)?));
                    let pair = self
                        .temporary(format!("insertvalue {ret} {first}, {}, 1", self.typed(*b)?));
                    self.line(format!("ret {ret} {pair}"));
                }
                _ => return Err("unsupported LLVM multiple-return ABI".into()),
            },
            _ => self.operation(op, args, results, data)?,
        }
        Ok(())
    }
    fn compare(
        &mut self,
        dest: &str,
        result: ir::Type,
        cond: &str,
        a: ir::Value,
        b: String,
        float: bool,
    ) -> Result<()> {
        let cc = if float {
            match cond {
                "ord" => "ord",
                "uno" => "uno",
                "eq" => "oeq",
                "ne" => "une",
                "one" => "one",
                "ueq" => "ueq",
                "lt" => "olt",
                "le" => "ole",
                "gt" => "ogt",
                "ge" => "oge",
                "ult" => "ult",
                "ule" => "ule",
                "ugt" => "ugt",
                "uge" => "uge",
                _ => return Err(format!("unsupported LLVM float condition {cond}")),
            }
        } else {
            match cond {
                "eq" => "eq",
                "ne" => "ne",
                "slt" => "slt",
                "sle" => "sle",
                "sgt" => "sgt",
                "sge" => "sge",
                "ult" => "ult",
                "ule" => "ule",
                "ugt" => "ugt",
                "uge" => "uge",
                _ => return Err(format!("unsupported LLVM integer condition {cond}")),
            }
        };
        let flag = self.temporary(format!(
            "{} {cc} {}, {b}",
            if float { "fcmp" } else { "icmp" },
            self.typed(a)?
        ));
        let flag_ty = if result.is_vector() {
            format!("<{} x i1>", result.lane_count())
        } else {
            "i1".into()
        };
        self.line(format!(
            "{dest} = {} {flag_ty} {flag} to {}",
            if result.is_vector() { "sext" } else { "zext" },
            ty(result)?
        ));
        Ok(())
    }
    fn call(
        &mut self,
        sig: &ir::Signature,
        callee: String,
        args: &[ir::Value],
        results: &[ir::Value],
    ) -> Result<()> {
        let (params, ret) = signature(sig)?;
        if params
            != args
                .iter()
                .map(|v| ty(self.value_type(*v)))
                .collect::<Result<Vec<_>>>()?
        {
            return Err("LLVM call parameter types differ".into());
        }
        let actual = args
            .iter()
            .map(|v| self.typed(*v))
            .collect::<Result<Vec<_>>>()?
            .join(", ");
        let call = format!("call {}{ret} {callee}({actual})", calling_convention(sig)?);
        match results {
            [] => self.line(call),
            [one] => self.line(format!("{} = {call}", self.value(*one))),
            [a, b] => {
                let pair = self.temporary(call);
                self.line(format!("{} = extractvalue {ret} {pair}, 0", self.value(*a)));
                self.line(format!("{} = extractvalue {ret} {pair}, 1", self.value(*b)));
            }
            _ => return Err("unsupported LLVM call result shape".into()),
        }
        Ok(())
    }
    fn operation(
        &mut self,
        op: O,
        args: &[ir::Value],
        results: &[ir::Value],
        data: &D,
    ) -> Result<()> {
        let Some(&result) = results.first() else {
            return Err(format!("unsupported LLVM effect {op}"));
        };
        let rt = self.value_type(result);
        let t = ty(rt)?;
        let dest = self.value(result);
        let binary = match op {
            O::Iadd | O::IaddImm => Some("add"),
            O::Isub => Some("sub"),
            O::Imul | O::ImulImm => Some("mul"),
            O::Band | O::BandImm => Some("and"),
            O::Bor | O::BorImm => Some("or"),
            O::Bxor | O::BxorImm => Some("xor"),
            O::Fadd => Some("fadd"),
            O::Fsub => Some("fsub"),
            O::Fmul => Some("fmul"),
            O::Fdiv => Some("fdiv"),
            _ => None,
        };
        if let Some(binary) = binary {
            let b = if let D::BinaryImm64 { imm, .. } = data {
                Self::constant(rt, imm.bits())?
            } else {
                self.value(args[1])
            };
            self.line(format!("{dest} = {binary} {}, {b}", self.typed(args[0])?));
            return Ok(());
        }
        match op {
            O::Umin | O::Umax | O::Smin | O::Smax | O::UsubSat | O::UaddSat => {
                let name = match op {
                    O::UsubSat => "usub.sat",
                    O::UaddSat => "uadd.sat",
                    O::Umin => "umin",
                    O::Umax => "umax",
                    O::Smin => "smin",
                    _ => "smax",
                };
                let suffix = if rt.is_vector() {
                    format!("v{}i{}", rt.lane_count(), rt.lane_bits())
                } else {
                    t.clone()
                };
                let value = self.intrinsic(
                    &format!("llvm.{name}.{suffix}"),
                    &t,
                    &[
                        (t.clone(), self.value(args[0])),
                        (t.clone(), self.value(args[1])),
                    ],
                );
                self.line(format!("{dest} = or {t} {value}, zeroinitializer"));
            }
            O::Ineg => self.line(format!(
                "{dest} = sub {t} zeroinitializer, {}",
                self.value(args[0])
            )),
            O::Bnot => {
                let ones = if rt.is_vector() {
                    format!(
                        "<{}>",
                        (0..rt.lane_count())
                            .map(|_| format!("i{} -1", rt.lane_bits()))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                } else {
                    "-1".into()
                };
                self.line(format!("{dest} = xor {}, {ones}", self.typed(args[0])?));
            }
            O::IrsubImm => {
                let D::BinaryImm64 { imm, .. } = data else {
                    return Err("LLVM reverse subtraction has no immediate".into());
                };
                self.line(format!(
                    "{dest} = sub {t} {}, {}",
                    Self::constant(rt, imm.bits())?,
                    self.value(args[0])
                ));
            }
            O::Isplit => {
                let src = self.value_type(args[0]);
                if results.len() != 2
                    || src.is_vector()
                    || rt.is_vector()
                    || src.bits() != rt.bits() * 2
                {
                    return Err("unsupported LLVM integer split shape".into());
                }
                self.line(format!("{dest} = trunc {} to {t}", self.typed(args[0])?));
                let high = self.temporary(format!("lshr {}, {}", self.typed(args[0])?, rt.bits()));
                self.line(format!(
                    "{} = trunc {} {high} to {t}",
                    self.value(results[1]),
                    ty(src)?
                ));
            }
            O::Bitselect => {
                let inverse = self.temporary(format!(
                    "xor {}, {}",
                    self.typed(args[0])?,
                    if rt.is_vector() {
                        format!(
                            "<{}>",
                            (0..rt.lane_count())
                                .map(|_| format!("i{} -1", rt.lane_bits()))
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    } else {
                        "-1".into()
                    }
                ));
                let left = self.temporary(format!(
                    "and {}, {}",
                    self.typed(args[0])?,
                    self.value(args[1])
                ));
                let right = self.temporary(format!("and {t} {inverse}, {}", self.value(args[2])));
                self.line(format!("{dest} = or {t} {left}, {right}"));
            }
            O::Rotl | O::RotlImm | O::Rotr | O::RotrImm => {
                if rt.is_vector() {
                    return Err("unsupported LLVM vector rotate".into());
                }
                let amount = if let D::BinaryImm64 { imm, .. } = data {
                    Self::constant(rt, imm.bits())?
                } else {
                    let st = self.value_type(args[1]);
                    if st == rt {
                        self.value(args[1])
                    } else {
                        let convert = if st.bits() < rt.bits() {
                            "zext"
                        } else {
                            "trunc"
                        };
                        self.temporary(format!("{convert} {} to {t}", self.typed(args[1])?))
                    }
                };
                let kind = if matches!(op, O::Rotl | O::RotlImm) {
                    "fshl"
                } else {
                    "fshr"
                };
                let a = self.value(args[0]);
                let value = self.intrinsic(
                    &format!("llvm.{kind}.{t}"),
                    &t,
                    &[(t.clone(), a.clone()), (t.clone(), a), (t.clone(), amount)],
                );
                self.line(format!("{dest} = or {t} {value}, 0"));
            }
            O::Uextend
            | O::Sextend
            | O::Ireduce
            | O::Fpromote
            | O::Fdemote
            | O::FcvtFromUint
            | O::FcvtFromSint => {
                let convert = match op {
                    O::Uextend => "zext",
                    O::Sextend => "sext",
                    O::Ireduce => "trunc",
                    O::Fpromote => "fpext",
                    O::Fdemote => "fptrunc",
                    O::FcvtFromUint => "uitofp",
                    _ => "sitofp",
                };
                self.line(format!(
                    "{dest} = {convert} {} to {t}",
                    self.typed(args[0])?
                ));
            }
            O::Bitcast => {
                if let D::LoadNoOffset { flags, .. } = data {
                    Self::memory_flags(*flags)?;
                }
                self.line(format!("{dest} = bitcast {} to {t}", self.typed(args[0])?));
            }
            O::Ishl | O::IshlImm | O::Ushr | O::UshrImm | O::Sshr | O::SshrImm => {
                let lane_bits = rt.lane_bits();
                let shift = if let D::BinaryImm64 { imm, .. } = data {
                    Self::mask_constant(rt, (imm.bits() as u32) & (lane_bits - 1))?
                } else {
                    let mut b = self.value(args[1]);
                    let st = self.value_type(args[1]);
                    if rt.is_vector() && !st.is_vector() {
                        let lane = rt.lane_type();
                        if st != lane {
                            let convert = if st.bits() < lane.bits() {
                                "zext"
                            } else {
                                "trunc"
                            };
                            b = self.temporary(format!(
                                "{convert} {} to {}",
                                self.typed(args[1])?,
                                ty(lane)?
                            ));
                        }
                        let one = self.temporary(format!(
                            "insertelement {t} poison, {} {b}, i32 0",
                            ty(lane)?
                        ));
                        b = self.temporary(format!(
                            "shufflevector {t} {one}, {t} poison, <{} x i32> zeroinitializer",
                            rt.lane_count()
                        ));
                    } else if st != rt {
                        if st.is_vector() || rt.is_vector() {
                            return Err("unsupported LLVM mixed shift shapes".into());
                        }
                        let convert = if st.bits() < rt.bits() {
                            "zext"
                        } else {
                            "trunc"
                        };
                        b = self.temporary(format!("{convert} {} to {t}", self.typed(args[1])?));
                    }
                    self.temporary(format!(
                        "and {t} {b}, {}",
                        Self::mask_constant(rt, lane_bits - 1)?
                    ))
                };
                let shift_op = match op {
                    O::Ishl | O::IshlImm => "shl",
                    O::Ushr | O::UshrImm => "lshr",
                    _ => "ashr",
                };
                self.line(format!(
                    "{dest} = {shift_op} {}, {shift}",
                    self.typed(args[0])?
                ));
            }
            O::Udiv
            | O::Urem
            | O::Sdiv
            | O::Srem
            | O::UdivImm
            | O::UremImm
            | O::SdivImm
            | O::SremImm => {
                if rt.is_vector() || rt.bits() > 64 {
                    return Err("unsupported LLVM division type".into());
                }
                let b = if let D::BinaryImm64 { imm, .. } = data {
                    Self::constant(rt, imm.bits())?
                } else {
                    self.value(args[1])
                };
                let zero = self.temporary(format!("icmp eq {t} {b}, 0"));
                self.guard_trap(zero);
                let signed = matches!(op, O::Sdiv | O::SdivImm | O::Srem | O::SremImm);
                let remainder = matches!(op, O::Urem | O::UremImm | O::Srem | O::SremImm);
                let mut divisor = b.clone();
                if signed {
                    let min = self.temporary(format!(
                        "icmp eq {}, {}",
                        self.typed(args[0])?,
                        1u64 << (rt.bits() - 1)
                    ));
                    let minus_one =
                        self.temporary(format!("icmp eq {t} {b}, {}", Self::constant(rt, -1)?));
                    let overflow = self.temporary(format!("and i1 {min}, {minus_one}"));
                    if remainder {
                        divisor = self.temporary(format!("select i1 {overflow}, {t} 1, {t} {b}"));
                    } else {
                        self.guard_trap(overflow);
                    }
                }
                self.line(format!(
                    "{dest} = {}{} {}, {divisor}",
                    if signed { "s" } else { "u" },
                    if remainder { "rem" } else { "div" },
                    self.typed(args[0])?
                ));
            }
            O::UaddOverflow
            | O::UsubOverflow
            | O::UmulOverflow
            | O::SaddOverflow
            | O::SsubOverflow
            | O::SmulOverflow => {
                if rt.is_vector() || results.len() != 2 {
                    return Err("unsupported LLVM overflow shape".into());
                }
                let name = format!(
                    "llvm.{}.with.overflow.{t}",
                    op.to_string().trim_end_matches("_overflow")
                );
                let pair_ty = format!("{{ {t}, i1 }}");
                let pair = self.intrinsic(
                    &name,
                    &pair_ty,
                    &[
                        (t.clone(), self.value(args[0])),
                        (t.clone(), self.value(args[1])),
                    ],
                );
                self.line(format!("{dest} = extractvalue {pair_ty} {pair}, 0"));
                let flag = self.temporary(format!("extractvalue {pair_ty} {pair}, 1"));
                self.line(format!(
                    "{} = zext i1 {flag} to {}",
                    self.value(results[1]),
                    ty(self.value_type(results[1]))?
                ));
            }
            O::Umulhi | O::Smulhi => {
                if rt.is_vector() {
                    return Err("unsupported LLVM vector high product".into());
                }
                let wide = format!("i{}", rt.bits() * 2);
                let extend = if op == O::Umulhi { "zext" } else { "sext" };
                let a = self.temporary(format!("{extend} {} to {wide}", self.typed(args[0])?));
                let b = self.temporary(format!("{extend} {} to {wide}", self.typed(args[1])?));
                let p = self.temporary(format!("mul {wide} {a}, {b}"));
                let high = self.temporary(format!("lshr {wide} {p}, {}", rt.bits()));
                self.line(format!("{dest} = trunc {wide} {high} to {t}"));
            }
            O::Popcnt | O::Clz | O::Ctz => {
                let kind = match op {
                    O::Popcnt => "ctpop",
                    O::Clz => "ctlz",
                    _ => "cttz",
                };
                let mut actual = vec![(t.clone(), self.value(args[0]))];
                if op != O::Popcnt {
                    actual.push(("i1".into(), "false".into()));
                }
                let suffix = if rt.is_vector() {
                    format!("v{}i{}", rt.lane_count(), rt.lane_bits())
                } else {
                    t.clone()
                };
                let v = self.intrinsic(&format!("llvm.{kind}.{suffix}"), &t, &actual);
                self.line(format!("{dest} = or {t} {v}, zeroinitializer"));
            }
            O::Select => {
                let c = self.truth(args[0])?;
                self.line(format!(
                    "{dest} = select i1 {c}, {}, {}",
                    self.typed(args[1])?,
                    self.typed(args[2])?
                ));
            }
            O::Splat => {
                let elem = self.temporary(format!(
                    "insertelement {t} poison, {}, i32 0",
                    self.typed(args[0])?
                ));
                let n = rt.lane_count();
                self.line(format!(
                    "{dest} = shufflevector {t} {elem}, {t} poison, <{n} x i32> zeroinitializer"
                ));
            }
            O::VanyTrue | O::VallTrue => {
                if !self.value_type(args[0]).is_vector() {
                    return Err("LLVM vector reduction requires vector".into());
                }
                let input = self.value_type(args[0]);
                let lanes = input.lane_count();
                let flags =
                    self.temporary(format!("icmp ne {}, zeroinitializer", self.typed(args[0])?));
                let kind = if op == O::VanyTrue { "or" } else { "and" };
                let flag = self.intrinsic(
                    &format!("llvm.vector.reduce.{kind}.v{lanes}i1"),
                    "i1",
                    &[(format!("<{lanes} x i1>"), flags)],
                );
                self.line(format!("{dest} = zext i1 {flag} to {t}"));
            }
            O::Fneg => self.line(format!("{dest} = fneg {}", self.typed(args[0])?)),
            O::Fabs
            | O::Sqrt
            | O::Ceil
            | O::Floor
            | O::Trunc
            | O::Nearest
            | O::Fmin
            | O::Fmax
            | O::Fcopysign
            | O::Fma => {
                if rt.is_vector() {
                    return Err("unsupported LLVM floating vector intrinsic".into());
                }
                let kind = match op {
                    O::Fabs => "fabs",
                    O::Sqrt => "sqrt",
                    O::Ceil => "ceil",
                    O::Floor => "floor",
                    O::Trunc => "trunc",
                    O::Nearest => "roundeven",
                    O::Fmin => "minimum",
                    O::Fmax => "maximum",
                    O::Fcopysign => "copysign",
                    _ => "fma",
                };
                let actual = args
                    .iter()
                    .map(|v| Ok((ty(self.value_type(*v))?, self.value(*v))))
                    .collect::<Result<Vec<_>>>()?;
                let v = self.intrinsic(&format!("llvm.{kind}.f{}", rt.bits()), &t, &actual);
                self.line(format!("{dest} = bitcast {t} {v} to {t}"));
            }
            O::FcvtToUintSat | O::FcvtToSintSat => {
                if rt.is_vector() {
                    return Err("unsupported LLVM vector float conversion".into());
                }
                let src = self.value_type(args[0]);
                let kind = if op == O::FcvtToUintSat {
                    "fptoui"
                } else {
                    "fptosi"
                };
                let v = self.intrinsic(
                    &format!("llvm.{kind}.sat.{t}.f{}", src.bits()),
                    &t,
                    &[(ty(src)?, self.value(args[0]))],
                );
                self.line(format!("{dest} = add {t} {v}, 0"));
            }
            O::FcvtToUint | O::FcvtToSint => {
                if rt.is_vector() || rt.bits() > 64 {
                    return Err("unsupported LLVM trapping float conversion shape".into());
                }
                let src = self.value_type(args[0]);
                let st = ty(src)?;
                let signed = op == O::FcvtToSint;
                let trunc = self.intrinsic(
                    &format!("llvm.trunc.f{}", src.bits()),
                    &st,
                    &[(st.clone(), self.value(args[0]))],
                );
                let upper = 2f64.powi((rt.bits() - u32::from(signed)) as i32);
                let lower = if signed { -upper } else { 0.0 };
                let float_constant = |x: f64| -> String { format!("0x{:016X}", x.to_bits()) };
                let lo =
                    self.temporary(format!("fcmp oge {st} {trunc}, {}", float_constant(lower)));
                let hi =
                    self.temporary(format!("fcmp olt {st} {trunc}, {}", float_constant(upper)));
                let valid = self.temporary(format!("and i1 {lo}, {hi}"));
                let bad = self.temporary(format!("xor i1 {valid}, true"));
                self.guard_trap(bad);
                self.line(format!(
                    "{dest} = {} {st} {trunc} to {t}",
                    if signed { "fptosi" } else { "fptoui" }
                ));
            }
            _ => return Err(format!("unsupported LLVM operation {op}")),
        }
        Ok(())
    }

    fn memory_flags(flags: ir::MemFlags) -> Result<()> {
        if flags.endianness(ir::Endianness::Little) != ir::Endianness::Little {
            return Err("unsupported LLVM big-endian memory operation".into());
        }
        // Alignment, readonly and can_move facts are deliberately not upgraded
        // to LLVM alias/purity/speculation promises. The source checks remain.
        Ok(())
    }
}
