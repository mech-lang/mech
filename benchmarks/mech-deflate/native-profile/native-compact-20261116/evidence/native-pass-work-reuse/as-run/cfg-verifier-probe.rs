use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::ir::{AbiParam, Block, Inst, InstBuilder, types};
use cranelift_codegen::{Context, settings};

fn example() -> (Context, Block, Block, Inst) {
    let mut context = Context::new();
    let entry = context.func.dfg.make_block();
    let first = context.func.dfg.make_block();
    let second = context.func.dfg.make_block();
    let mut cursor = FuncCursor::new(&mut context.func);
    cursor.insert_block(entry);
    let jump = cursor.ins().jump(first, &[]);
    cursor.insert_block(first);
    cursor.ins().return_(&[]);
    cursor.insert_block(second);
    cursor.ins().return_(&[]);
    context.flowgraph();
    (context, entry, second, jump)
}

fn main() {
    let flags = settings::Flags::new(settings::builder());
    let (mut context, _, second, jump) = example();
    context.verify(&flags).unwrap();
    println!("valid CFG and instructions");
    context.func.dfg.replace(jump).jump(second, &[]);
    let result = context.verify(&flags).unwrap_err().to_string();
    assert!(result.contains("cfg lacked the following successor"));
    print!("{result}");

    let (mut context, entry, _, jump) = example();
    let first = context.cfg.succ_iter(entry).next().unwrap();
    let mut cursor = FuncCursor::new(&mut context.func);
    cursor.goto_inst(jump);
    cursor.remove_inst();
    cursor.goto_bottom(entry);
    let newer = cursor.ins().jump(first, &[]);
    assert_ne!(jump, newer);
    let result = context.verify(&flags).unwrap_err().to_string();
    assert!(result.contains("cfg lacked the following predecessor"));
    print!("{result}");

    let (mut context, _, _, _) = example();
    context
        .func
        .signature
        .returns
        .push(AbiParam::new(types::I64));
    let result = context.verify(&flags).unwrap_err().to_string();
    assert!(!result.is_empty() && !result.contains("cfg lacked"));
    print!("{result}");
}
