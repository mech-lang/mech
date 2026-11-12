#!/usr/bin/env python3
"""Record original executable sections and Mech code/IR/stack sizes."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--llvm-size', default='/opt/homebrew/opt/llvm/bin/llvm-size')
    parser.add_argument('--reference', default='rust-concurrent')
    args = parser.parse_args()
    config = json.loads(args.config.read_text())
    rows = []
    for variant in config['implementations']:
        executable = Path(variant['executable'])
        result = subprocess.run([args.llvm_size, '--format=sysv', '-d', str(executable)], capture_output=True, text=True, check=True)
        sections = {}
        for line in result.stdout.splitlines():
            match = re.fullmatch(r'\s*(\S+)\s+(\d+)\s+(\d+)\s*', line)
            if match:
                sections[match[1]] = sections.get(match[1], 0)+int(match[2])
        row = dict(variant=variant['name'], category=variant['category'], executable=str(executable),
                   executable_sha256=sha(executable), executable_file_bytes=executable.stat().st_size,
                   executable_sections_bytes=sections, executable_text_bytes=sections.get('__text'),
                   size_tool_stdout=result.stdout)
        if variant['kind'] == 'mech':
            receipt = json.loads(Path(variant['receipt']).read_text())
            bundle = Path(receipt['bundle']['source.mec']['path']).parent
            metadata = json.loads((bundle/'metadata.json').read_text())
            functions = metadata['compiled_numeric_functions']
            cleanup = metadata['compiled_cfg_cleanup']
            row['native_program'] = dict(
                code_bytes=metadata['code_bytes'], object_bytes=metadata['object_bytes'],
                object_file_bytes=(bundle/'object.o').stat().st_size,
                artifact_bytecode_bytes=(bundle/'artifact.mecb').stat().st_size,
                descriptor_json_bytes=(bundle/'descriptor.json').stat().st_size,
                native_main_stack_frame_bytes=metadata['stack_bytes'],
                max_numeric_function_stack_frame_bytes=max((r['stack_bytes'] for r in functions), default=0),
                numeric_function_count=len(functions),
                main_ir_instructions_before_cleanup=cleanup['before_instructions'],
                main_ir_instructions_after_cleanup=cleanup['after_instructions'],
                numeric_function_ir_instructions_before_cleanup=sum(r['cleanup']['before_instructions'] for r in functions),
                numeric_function_ir_instructions_after_cleanup=sum(r['cleanup']['after_instructions'] for r in functions),
                typed_ir_version=metadata['typed_ir_version'], work_policy_version=metadata['work_policy_version'],
                abi_version=metadata['abi_version'], optimization=metadata['object_optimization'],
                requested_adapter_thread_stack_bytes=32*1024*1024,
                requested_numeric_worker_stack_bytes=2*1024*1024,
            )
            row['symbol_policy'] = 'strip=debuginfo; native symbols retained for link auditing'
        rows.append(row)
    reference = next(row for row in rows if row['variant'] == args.reference)
    for row in rows:
        row['executable_file_bytes_over_fastest_rust'] = row['executable_file_bytes']/reference['executable_file_bytes']
        if row['executable_text_bytes'] is not None:
            row['executable_text_bytes_over_fastest_rust'] = row['executable_text_bytes']/reference['executable_text_bytes']
    report = dict(schema='mech.related-post-gen.code-size.v1', reference=args.reference, implementations=rows,
                  limitations=[
                      'Original latency executables, not the instrumented adapters. File size includes symbols, embedded images and runtime; symbol/stripping policies differ.',
                      'Whole-executable text is comparable as a deployment cost. Mech native-program-only code excludes its adapter/runtime and is a separately labeled subset.',
                      'IR instruction counts are static compiler instructions, not machine instructions or dynamic retired instructions.',
                      'Stack figures are individual generated frame sizes or requested OS stack reservations; they are not measured peak call-stack use.',
                  ])
    args.output.write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(dict(status='recorded', variants=len(rows), output=str(args.output))))


if __name__ == '__main__':
    main()
