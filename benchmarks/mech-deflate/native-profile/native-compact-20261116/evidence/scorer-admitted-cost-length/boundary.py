"""Run the exact guard transform on a small native lookup and test both edges."""

import json
import os
from pathlib import Path
import subprocess

from stage import CHECKOUT, HERE, PROGRAM, ROADMAP, SETUP, sha, write

OUT = HERE / 'boundary'
DIRECT = PROGRAM / 'codegen/cfg_cleanup'


def piece(source, first, after):
    begin = source.index(first)
    return source[begin:source.index(after, begin)]


DRIVER = '''
pub fn emit(output: &str) {
    let isa = cranelift_native::builder().unwrap()
        .finish(cranelift_codegen::settings::Flags::new(cranelift_codegen::settings::builder()))
        .unwrap();
    let builder = ObjectBuilder::new(isa, "cost_length_checks", default_libcall_names()).unwrap();
    let mut module = ObjectModule::new(builder);
    for (name, enabled, used_word, expected) in [
        ("lookup_original", false, 1, 0),
        ("lookup_split", true, 1, 1),
        ("lookup_other_length", true, 2, 0),
    ] {
        let mut context = module.make_context();
        context.func.signature = module.make_signature();
        context.func.signature.params = vec![AbiParam::new(types::I64); 3];
        context.func.signature.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function(name, Linkage::Export, &context.func.signature).unwrap();
        let mut state = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut context.func, &mut state);
        let entry = b.create_block();
        let header = b.create_block();
        let yes = b.create_block();
        let no = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.append_block_param(header, types::I64);
        b.switch_to_block(entry);
        let [raw, packet, _] = b.block_params(entry).try_into().unwrap();
        let count = b.ins().load(types::I64, MemFlags::new(), packet, 8);
        let data = b.ins().load(types::I64, MemFlags::new(), packet, 0);
        let index = b.ins().iadd_imm(raw, 259);
        b.ins().jump(header, &[count.into()]);
        b.switch_to_block(header);
        let length = b.block_params(header)[0];
        let fit = b.ins().icmp(IntCC::UnsignedLessThan, index, length);
        let guard = b.ins().band_imm(fit, 1);
        b.ins().brif(guard, yes, &[], no, &[]);
        b.switch_to_block(yes);
        let scaled = b.ins().ishl_imm(index, 2);
        let at = b.ins().iadd(data, scaled);
        let value = b.ins().uload32(MemFlags::new(), at, 0);
        b.ins().return_(&[value]);
        b.switch_to_block(no);
        let default = b.ins().iconst(types::I64, 0);
        b.ins().return_(&[default]);
        b.seal_all_blocks();
        b.finalize();

        let loads = BTreeMap::from([(count, (definition(&context.func, count).unwrap().0, 1))]);
        let view = View { array: 2, pointer_word: 0, length_word: used_word, width: 4, literal: None };
        let capture = Capture { packet_loads: &loads, views: &[view], header, countdown_word: 0 };
        let admission = Admission { costs: 2, minimum_costs: 545, cache: 0, cursor_word: 0,
            cells_per_bank: 256, second_bias: 14, minimum_records: 2048, immutable_code_bytes: Some(65536) };
        context.compute_cfg();
        if enabled {
            let n = split_admitted_cost_checks(&mut context.func, &context.cfg, &capture, admission);
            assert_eq!(n, expected, "{name}");
        }
        context.compute_cfg();
        context.verify(module.isa()).unwrap();
        module.define_function(id, &mut context).unwrap();
    }
    std::fs::write(output, module.finish().emit().unwrap()).unwrap();
}
'''


HOST = r'''
#include <assert.h>
#include <stdint.h>
#include <stdio.h>

extern uint64_t lookup_original(uint64_t, const uint64_t *, uint64_t);
extern uint64_t lookup_split(uint64_t, const uint64_t *, uint64_t);
extern uint64_t lookup_other_length(uint64_t, const uint64_t *, uint64_t);

int main(void) {
    static uint32_t values[65536];
    for (uint32_t j = 0; j < 65536; j++)
        values[j] = j + 1;
    const uint64_t lengths[] = {545, 546, 547, 610, 1024, 65536};
    const uint64_t indexes[] = {
        0, 1, 255, 285, 286, 287, 288, 349, 351, 764, 765, 766, 65000,
        UINT64_MAX - 259, UINT64_MAX - 258, UINT64_MAX - 1, UINT64_MAX
    };
    unsigned checked = 0;
    for (unsigned n = 0; n < sizeof(lengths) / sizeof(lengths[0]); n++) {
        const uint64_t packet[] = {(uint64_t)(uintptr_t)values, lengths[n], 65536};
        for (unsigned j = 0; j < sizeof(indexes) / sizeof(indexes[0]); j++) {
            const uint64_t raw = indexes[j], index = raw + 259;
            const uint64_t expected = index < lengths[n] ? values[index] : 0;
            assert(lookup_original(raw, packet, 0) == expected);
            assert(lookup_split(raw, packet, 0) == expected);
            assert(lookup_other_length(raw, packet, 0) == expected);
            checked++;
        }
    }
    printf("%u lookup inputs passed on all three generated entries\n", checked);
}
'''


def main():
    out = OUT
    out.mkdir()
    base = CHECKOUT / DIRECT
    composed = (base / 'composed_reads.rs').read_text()
    words = (base / 'word_ranges/mod.rs').read_text()
    windows = (base / 'window_bounds/mod.rs').read_text()
    conditions = (base / 'boolean_conditions/mod.rs').read_text()
    probe = (HERE / 'source' / SETUP).read_text()
    selected = [
        piece(composed, '#[derive(Clone)]\npub(in crate::native_artifact) struct Literal', '#[derive(Clone, PartialEq'),
        piece(composed, 'pub(super) fn definition', 'pub(super) fn scalar'),
        piece(composed, 'pub(super) fn read_comparison', 'pub(super) fn defaulted_read'),
        piece(words, 'fn word_mask', '#[cfg(test)]'),
        piece(windows, 'fn predicate_definition', 'fn length_test'),
        piece(conditions, 'pub(in super::super) fn masked_integer_comparison', 'fn literal(')
            .replace('pub(in super::super) ', ''),
        piece(probe, '/// Use the caller\'s admitted cost extent', 'fn minus_one'),
    ]
    rust = out / 'emitter.rs'
    output = out / 'lookup.o'
    tests = out / 'check.c'
    write(rust, '''
#![allow(dead_code)]
mod native_artifact {
use std::{collections::{BTreeMap, BTreeSet}, sync::Arc};
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::{AbiParam, Block, Function, Inst, InstBuilder,
    Opcode, Value, ValueDef, MemFlags, condcodes::IntCC, types};
use cranelift_codegen::ir::{InstructionData as D, InstructionData as Data, Opcode as O, Opcode as Op};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};
''' + '\n'.join(selected) + DRIVER + '''
}
fn main() {
    let output = std::env::args().nth(1).unwrap();
    native_artifact::emit(&output);
}
''')
    write(tests, HOST)
    pinned = ROADMAP / 'immutable-root-literals/compiler/disabled/compiler-build.jsonl'
    libs = {}
    names = {'serde', 'cranelift_codegen', 'cranelift_frontend',
             'cranelift_native', 'cranelift_module', 'cranelift_object'}
    for line in pinned.read_text().splitlines():
        if not line.startswith('{'):
            continue
        item = json.loads(line)
        if item.get('reason') == 'compiler-artifact' and (target := item['target']['name']) in names:
            (libs[target],) = [Path(p) for p in item['filenames'] if p.endswith('.rlib')]
    assert set(libs) == names and all(lib.is_file() for lib in libs.values())
    dependency = libs['cranelift_codegen'].parent
    command = ['rustc', '+nightly-2026-03-03', '--edition', '2024', '-C', 'opt-level=1',
               '-L', f'dependency={dependency}']
    for name, lib in libs.items():
        command.extend(('--extern', f'{name}={lib}'))
    command.extend((rust, '-o', out / 'emitter'))
    commands = [command, [out / 'emitter', output],
                ['/usr/bin/cc', '-std=c11', '-O2', '-Wall', '-Werror', tests, output, '-o', out / 'check'],
                [out / 'check']]
    log = out / 'check.log'
    with log.open('w') as stream:
        for args in commands:
            subprocess.run(args, stdout=stream, stderr=subprocess.STDOUT, check=True)
    assert '102 lookup inputs passed on all three generated entries' in log.read_text()
    write(out / 'result.json', {'cases': 102, 'entries': 3, 'objects_sha256': sha(output),
          'source_sha256': sha(rust), 'test_sha256': sha(tests), 'log_sha256': sha(log),
          'commands': [list(map(str, command)) for command in commands],
          'library_sha256': {name: sha(lib) for name, lib in libs.items()}})
    print(log.read_text().strip(), flush=True)


if __name__ == '__main__':
    main()
