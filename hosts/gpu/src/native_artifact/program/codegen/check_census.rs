//! Diagnostic JIT-only native check counters. No callback executes per check.
//! Enabled only in a custom-cfg diagnostic build and only while compiling a JIT
//! owner. Object emission is never enabled. Counter allocations live until exit,
//! so subsequent compilations cannot invalidate existing generated pointers.
use cranelift_codegen::ir::{AtomicRmwOp, InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use std::{cell::Cell, collections::BTreeMap, sync::{Mutex, OnceLock, atomic::{AtomicU64, Ordering}}};
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
struct Key { compilation: u64, kind: &'static str, owner: usize, site: usize, version: bool }
thread_local! { static CURRENT: Cell<Option<u64>> = const { Cell::new(None) }; }
static NEXT: AtomicU64 = AtomicU64::new(0);
static ROWS: OnceLock<Mutex<BTreeMap<Key, Box<[AtomicU64; 2]>>>> = OnceLock::new();
pub(in crate::native_artifact) struct Guard;
impl Drop for Guard { fn drop(&mut self) { CURRENT.with(|c| c.set(None)); } }
pub(in crate::native_artifact) fn begin() -> Guard {
    CURRENT.with(|c| { assert!(c.get().is_none()); c.set(Some(NEXT.fetch_add(1, Ordering::Relaxed))); });
    Guard
}
pub(in crate::native_artifact) fn record(b: &mut FunctionBuilder<'_>, kind: &'static str, owner: usize, site: usize, version: bool, condition: Option<Value>) {
    if cfg!(mech_native_opportunity_census)
        && (std::env::var("MECH_NATIVE_OPPORTUNITY_MODE").as_deref() != Ok("retirements") || kind != "retired-scalar-pair") { return; }
    let Some(compilation) = CURRENT.with(Cell::get) else { return; };
    let key = Key { compilation, kind, owner, site, version };
    let address = {
        let mut rows = ROWS.get_or_init(|| Mutex::new(BTreeMap::new())).lock().unwrap();
        assert!(rows.len() < 100_000, "diagnostic census row bound exceeded");
        let words = rows.entry(key).or_insert_with(|| Box::new([AtomicU64::new(0), AtomicU64::new(0)]));
        words.as_ptr() as usize
    };
    let one = b.ins().iconst(types::I64, 1);
    let success = if let Some(condition) = condition {
        let truth = b.ins().icmp_imm(IntCC::NotEqual, condition, 0);
        b.ins().uextend(types::I64, truth)
    } else { one };
    for (at, value) in [(address, one), (address + 8, success)] {
        let at = b.ins().iconst(types::I64, at as i64);
        b.ins().atomic_rmw(types::I64, MemFlags::trusted(), AtomicRmwOp::Add, at, value);
    }
}
pub(in crate::native_artifact) fn snapshot() -> serde_json::Value {
    let rows = ROWS.get_or_init(|| Mutex::new(BTreeMap::new())).lock().unwrap();
    serde_json::json!({"schema":"mech.native-executed-check-census.v1", "diagnostic_only":true, "JIT_only":true, "timing_claim":false,
        "coverage":"Opportunity mode selects either retired scalar home pairs or post-optimizer branch cones. Branch success means first successor taken; version means inverse Boolean consumer. No timings, CPU shares or instruction-cost claim.",
        "opportunity_mode":std::env::var("MECH_NATIVE_OPPORTUNITY_MODE").ok(),
        "rows": rows.iter().map(|(key, words)| { let attempts=words[0].load(Ordering::SeqCst); let successes=words[1].load(Ordering::SeqCst); serde_json::json!({"key":key,"attempts":attempts,"successes":successes,"failures":attempts.checked_sub(successes).expect("census underflow")}) }).collect::<Vec<_>>()})
}


#[cfg(mech_native_opportunity_census)]
pub(in crate::native_artifact) fn observe_optimized_branches(
    context: &mut cranelift_codegen::Context,
    isa: &dyn cranelift_codegen::isa::TargetIsa,
    owner: usize,
) -> Result<(), crate::native_artifact::NativeCompileError> {
    use cranelift_codegen::cursor::{Cursor, FuncCursor};
    use cranelift_codegen::ir::{InstructionData as D, Opcode as O};
    let Some(compilation) = CURRENT.with(Cell::get) else { return Ok(()); };
    if std::env::var("MECH_NATIVE_OPPORTUNITY_MODE").as_deref() != Ok("branches") { return Ok(()); }
    // No early check/retirement counters exist in this mode. Identify the
    // predicates after the ordinary Cranelift optimizations have run. Counters
    // are added only afterwards; their timings and altered machine code are
    // ineligible for performance claims.
    context.optimize(isa, &mut Default::default()).map_err(crate::native_artifact::compiler::backend)?;
    if let Ok(directory) = std::env::var("MECH_OPPORTUNITY_IR_DIR") {
        std::fs::create_dir_all(&directory).map_err(crate::native_artifact::compiler::backend)?;
        std::fs::write(std::path::Path::new(&directory).join(format!("owner-{owner}.clif")),context.func.display().to_string())
            .map_err(crate::native_artifact::compiler::backend)?;
    }
    let function = &mut context.func;
    let mut uses=vec![0usize;function.dfg.num_values()];
    for block in function.layout.blocks() { for inst in function.layout.block_insts(block) {
        for &value in function.dfg.inst_args(inst) { uses[function.dfg.resolve_aliases(value).as_u32()as usize]+=1; }
        // Conservatively include branch destination operands too. Counting a
        // transport twice can only decline a single-use opportunity.
        for call in function.dfg.insts[inst].branch_destination(&function.dfg.jump_tables,&function.dfg.exception_tables) {
            for arg in call.args(&function.dfg.value_lists) { if let Some(value)=arg.as_value() { uses[function.dfg.resolve_aliases(value).as_u32()as usize]+=1; } }
        }
    }}
    let mut observed=Vec::new();
    let mut carry_sites=Vec::new();
    for block in function.layout.blocks() {
        let Some(inst)=function.layout.last_inst(block) else {continue};
        let D::Brif {arg,..}=function.dfg.insts[inst] else {continue};
        let (value,inverted)=normalize_predicate(function,arg,16);
        if literal(function,value).is_some() {continue;}
        let kind=if let Some((op,left,right))=logic(function,value) {
            match (integer_comparison(function,left),integer_comparison(function,right)) {
                (Some((a,aw)),Some((b,bw))) if aw==64 && bw==64 && op==O::Band && a==IntCC::Equal && b==IntCC::Equal => "branch-eq-and64-native",
                (Some((a,aw)),Some((b,bw))) if aw==64 && bw==64 && op==O::Bor && a==IntCC::NotEqual && b==IntCC::NotEqual => "branch-ne-or64-native",
                (Some(_),Some(_)) => "branch-two-integer-comparisons-other",
                _ if comparison_leaves(function,value,16).is_some_and(|n|n>=2) => "branch-integer-comparison-tree",
                _ => "branch-other-logic",
            }
        } else if add_carry_comparison(function,value) {"branch-single-add-carry-comparison"}
        else if integer_comparison(function,value).is_some() {"branch-single-integer-comparison"}
        else {"branch-raw-or-other"};
        let single=single_boolean_cone(function,arg,&uses,16);
        let kind=if single {kind}else{match kind {
            "branch-eq-and64-native"=>"branch-eq-and64-shared",
            "branch-ne-or64-native"=>"branch-ne-or64-shared",
            "branch-two-integer-comparisons-other"=>"branch-two-integer-comparisons-shared",
            "branch-integer-comparison-tree"=>"branch-integer-comparison-tree-shared",
            other=>other,
        }};
        if kind == "branch-single-add-carry-comparison" {
            let mut defs=std::collections::BTreeMap::new();
            value_definitions(function,arg,16,&mut defs);
            carry_sites.push(serde_json::json!({"site":inst.as_u32(),"owner":owner,"condition":arg.to_string(),"normalized":value.to_string(),"inverted":inverted,"block":block.to_string(),"block_instructions":function.layout.block_insts(block).map(|i|format!("{}: {}",i,function.dfg.display_inst(i))).collect::<Vec<_>>(),"definitions":defs}));
        }
        observed.push((inst,arg,inverted,kind));
    }
    if let Ok(directory)=std::env::var("MECH_OPPORTUNITY_IR_DIR") {
        std::fs::write(std::path::Path::new(&directory).join(format!("owner-{owner}-carry-sites.json")),serde_json::to_vec_pretty(&carry_sites).map_err(crate::native_artifact::compiler::backend)?)
            .map_err(crate::native_artifact::compiler::backend)?;
    }
    for (inst,condition,inverted,kind) in observed {
        let key=Key {compilation,kind,owner,site:inst.as_u32()as usize,version:inverted};
        let address={
            let mut rows=ROWS.get_or_init(||Mutex::new(BTreeMap::new())).lock().unwrap();
            assert!(rows.len()<100_000,"diagnostic census row bound exceeded");
            rows.entry(key).or_insert_with(||Box::new([AtomicU64::new(0),AtomicU64::new(0)])).as_ptr()as usize
        };
        let mut cursor=FuncCursor::new(function).at_inst(inst);
        let one=cursor.ins().iconst(types::I64,1);
        let truth=cursor.ins().icmp_imm(IntCC::NotEqual,condition,0);
        let success=cursor.ins().uextend(types::I64,truth);
        for (at,value)in[(address,one),(address+8,success)] {
            let at=cursor.ins().iconst(types::I64,at as i64);
            cursor.ins().atomic_rmw(types::I64,MemFlags::trusted(),AtomicRmwOp::Add,at,value);
        }
    }
    Ok(())
}

#[cfg(mech_native_opportunity_census)]
fn literal(f:&cranelift_codegen::ir::Function,v:Value)->Option<u64>{
    use cranelift_codegen::ir::{ValueDef,InstructionData as D,Opcode as O};
    let v=f.dfg.resolve_aliases(v);
    let ValueDef::Result(i,0)=f.dfg.value_def(v)else{return None};
    if let D::UnaryImm{opcode:O::Iconst,imm}=f.dfg.insts[i]{Some(imm.bits()as u64)}else{None}
}
#[cfg(mech_native_opportunity_census)]
fn boolean(f:&cranelift_codegen::ir::Function,v:Value,depth:usize)->bool{
    use cranelift_codegen::ir::{ValueDef,InstructionData as D,Opcode as O};
    if depth==0{return false;}let v=f.dfg.resolve_aliases(v);
    let ValueDef::Result(i,0)=f.dfg.value_def(v)else{return false};
    match f.dfg.insts[i]{
        D::IntCompare{..}|D::IntCompareImm{..}|D::FloatCompare{..}=>true,
        D::UnaryImm{opcode:O::Iconst,..}=>matches!(literal(f,v),Some(0|1)),
        D::Unary{opcode:O::Uextend|O::Sextend|O::Ireduce,arg}=>boolean(f,arg,depth-1),
        D::Binary{opcode:O::Band|O::Bor|O::Bxor,args}=>args.iter().all(|&a|boolean(f,a,depth-1)),
        D::BinaryImm64{opcode:O::BandImm,imm,..}if imm.bits()==1=>true,
        _=>false,
    }
}
#[cfg(mech_native_opportunity_census)]
fn normalize_predicate(f:&cranelift_codegen::ir::Function,v:Value,depth:usize)->(Value,bool){
    use cranelift_codegen::ir::{ValueDef,InstructionData as D,Opcode as O};
    let v=f.dfg.resolve_aliases(v);if depth==0{return(v,false);}
    let ValueDef::Result(i,0)=f.dfg.value_def(v)else{return(v,false)};
    match f.dfg.insts[i]{
        D::Unary{opcode:O::Uextend|O::Sextend|O::Ireduce,arg}if boolean(f,arg,depth-1)=>normalize_predicate(f,arg,depth-1),
        D::BinaryImm64{opcode:O::BandImm,arg,imm}if imm.bits()==1&&boolean(f,arg,depth-1)=>normalize_predicate(f,arg,depth-1),
        D::IntCompareImm{cond,arg,imm,..}if matches!(cond,IntCC::Equal|IntCC::NotEqual)&&matches!(imm.bits(),0|1)&&boolean(f,arg,depth-1)=>{
            let(a,inv)=normalize_predicate(f,arg,depth-1);(a,inv^((cond==IntCC::Equal)==(imm.bits()==0)))
        },
        _=>(v,false),
    }
}
#[cfg(mech_native_opportunity_census)]
fn logic(f:&cranelift_codegen::ir::Function,v:Value)->Option<(cranelift_codegen::ir::Opcode,Value,Value)>{
    use cranelift_codegen::ir::{ValueDef,InstructionData as D,Opcode as O};
    let ValueDef::Result(i,0)=f.dfg.value_def(f.dfg.resolve_aliases(v))else{return None};
    match f.dfg.insts[i]{D::Binary{opcode,args:[a,b]}if matches!(opcode,O::Band|O::Bor)=>Some((opcode,a,b)),_=>None}
}
#[cfg(mech_native_opportunity_census)]
fn integer_comparison(f:&cranelift_codegen::ir::Function,v:Value)->Option<(IntCC,u32)>{
    use cranelift_codegen::ir::{ValueDef,InstructionData as D,condcodes::CondCode};
    let(v,inverted)=normalize_predicate(f,v,16);let ValueDef::Result(i,0)=f.dfg.value_def(v)else{return None};
    let(cc,arg)=match f.dfg.insts[i]{D::IntCompare{cond,args:[a,_],..}=>(cond,a),D::IntCompareImm{cond,arg,..}=>(cond,arg),_=>return None};
    let ty=f.dfg.value_type(arg);if !ty.is_int()||ty.bits()>64{return None};Some((if inverted{cc.complement()}else{cc},ty.bits()))
}
#[cfg(mech_native_opportunity_census)]
fn comparison_leaves(f:&cranelift_codegen::ir::Function,v:Value,depth:usize)->Option<usize>{
    if depth==0{return None;}if integer_comparison(f,v).is_some(){return Some(1);}
    let(v,_)=normalize_predicate(f,v,depth);let(_,a,b)=logic(f,v)?;
    comparison_leaves(f,a,depth-1)?.checked_add(comparison_leaves(f,b,depth-1)?)
}


#[cfg(mech_native_opportunity_census)]
fn single_boolean_cone(f:&cranelift_codegen::ir::Function,v:Value,uses:&[usize],depth:usize)->bool{
    use cranelift_codegen::ir::{ValueDef,InstructionData as D,Opcode as O};
    if depth==0{return false;}let v=f.dfg.resolve_aliases(v);
    if uses[v.as_u32()as usize]!=1{return false;}
    let ValueDef::Result(i,0)=f.dfg.value_def(v)else{return false};
    match f.dfg.insts[i]{
        D::IntCompareImm{cond,arg,imm,..}if matches!(cond,IntCC::Equal|IntCC::NotEqual)&&matches!(imm.bits(),0|1)&&boolean(f,arg,depth-1)=>single_boolean_cone(f,arg,uses,depth-1),
        D::IntCompare{..}|D::IntCompareImm{..}=>true,
        D::Unary{opcode:O::Uextend|O::Sextend|O::Ireduce,arg}if boolean(f,arg,depth-1)=>single_boolean_cone(f,arg,uses,depth-1),
        D::BinaryImm64{opcode:O::BandImm,arg,imm}if imm.bits()==1&&boolean(f,arg,depth-1)=>single_boolean_cone(f,arg,uses,depth-1),
        D::Binary{opcode:O::Band|O::Bor,args}=>args.iter().all(|&a|single_boolean_cone(f,a,uses,depth-1)),
        _=>false,
    }
}
#[cfg(mech_native_opportunity_census)]
fn add_carry_comparison(f:&cranelift_codegen::ir::Function,v:Value)->bool{
    use cranelift_codegen::ir::{ValueDef,InstructionData as D,Opcode as O};
    let(v,_)=normalize_predicate(f,v,16);let ValueDef::Result(i,0)=f.dfg.value_def(v)else{return false};
    let D::IntCompare{cond,args,..}=f.dfg.insts[i]else{return false};
    let(sum,base)=match cond{
        IntCC::UnsignedLessThan|IntCC::UnsignedGreaterThanOrEqual=>(args[0],args[1]),
        IntCC::UnsignedGreaterThan|IntCC::UnsignedLessThanOrEqual=>(args[1],args[0]),
        _=>return false,
    };
    let ValueDef::Result(i,0)=f.dfg.value_def(f.dfg.resolve_aliases(sum))else{return false};
    match f.dfg.insts[i]{
        D::Binary{opcode:O::Iadd,args}=>args.iter().any(|&v|f.dfg.resolve_aliases(v)==f.dfg.resolve_aliases(base)),
        D::BinaryImm64{opcode:O::IaddImm,arg,..}=>f.dfg.resolve_aliases(arg)==f.dfg.resolve_aliases(base),
        _=>false,
    }
}

#[cfg(mech_native_opportunity_census)]
fn value_definitions(f:&cranelift_codegen::ir::Function,v:Value,depth:usize,out:&mut std::collections::BTreeMap<String,String>){
    use cranelift_codegen::ir::ValueDef;
    if depth==0{return;}let v=f.dfg.resolve_aliases(v);let name=v.to_string();if out.contains_key(&name){return;}
    match f.dfg.value_def(v){
        ValueDef::Result(i,_)=>{
            out.insert(name,format!("{}: {}",i,f.dfg.display_inst(i)));
            for &arg in f.dfg.inst_args(i){value_definitions(f,arg,depth-1,out);}
        },
        ValueDef::Param(block,index)=>{out.insert(name,format!("{block} parameter {index}"));},
        _=>{out.insert(name,"detached or union value".to_owned());},
    }
}
