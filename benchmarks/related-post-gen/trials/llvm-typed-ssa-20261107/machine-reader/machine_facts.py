#!/usr/bin/env python3
"""Read actual AArch64 Mach-O code and LLVM stack reports before Object binding.

This is a compile-time tool. It neither executes source operations nor grants
source/layout authority. The compiler must supply its closed definition names,
validate these machine facts against its limits, then construct a fresh identity.
"""
from pathlib import Path
import argparse, hashlib, json, re, struct


def llvm_stack_sizes(text):
    stacks = {}
    for doc in re.split(r'^---', text, flags=re.M):
        fields = dict(re.findall(r'^([A-Za-z]+):\s*([^\n]+)$', doc, flags=re.M))
        if fields.get('Pass', '').strip() != 'prolog-epilog' or fields.get('Name', '').strip() != 'StackSize':
            continue
        name = fields.get('Function', '').strip().strip("'\"")
        if not re.fullmatch(r'[A-Za-z_$][A-Za-z0-9_.$]*', name):
            raise ValueError('unsupported stack-report definition name')
        values = re.findall(r'^\s+- NumStackBytes:\s*[\'\"]?(\d+)[\'\"]?\s*$', doc, flags=re.M)
        if len(values) != 1 or name in stacks:
            raise ValueError('missing or repeated stack-size record: ' + name)
        size = int(values[0])
        if size > 0xffffffffffffffff:
            raise ValueError('stack-size overflow: ' + name)
        stacks[name] = size
    return stacks


def macho(data):
    if len(data) < 32:
        raise ValueError('short Mach-O header')
    magic, cpu, subtype, kind, ncmds, command_bytes, _, _ = struct.unpack_from('<8I', data)
    if magic != 0xfeedfacf or cpu != 0x0100000c or subtype != 0 or kind != 1:
        raise ValueError('requires an AArch64 Mach-O relocatable Object')
    end = 32 + command_bytes
    if end > len(data):
        raise ValueError('invalid load-command extent')
    sections = []; symtab = None; at = 32
    for _ in range(ncmds):
        if at + 8 > end:
            raise ValueError('truncated load command')
        command, size = struct.unpack_from('<II', data, at)
        if size < 8 or at + size > end:
            raise ValueError('invalid load command')
        if command == 0x19:
            if size < 72:
                raise ValueError('short segment command')
            count = struct.unpack_from('<I', data, at + 64)[0]
            if size != 72 + count * 80:
                raise ValueError('invalid section table')
            for i in range(count):
                p = at + 72 + i * 80
                name, segment, address, length, offset, alignment, _, _, flags, _, _, _ = struct.unpack_from('<16s16sQQ8I', data, p)
                name = name.rstrip(b'\0').decode('ascii'); segment = segment.rstrip(b'\0').decode('ascii')
                if length and (flags & 0xff) in (1, 12, 18):
                    raise ValueError('unexpected zero-fill machine section')
                if address + length > 0xffffffffffffffff:
                    raise ValueError('machine section address overflow')
                if offset + length > len(data):
                    raise ValueError('machine section outside Object')
                sections.append(dict(name=name, segment=segment, address=address, size=length, offset=offset, alignment_power=alignment, flags=flags))
        elif command == 2:
            if size != 24 or symtab is not None:
                raise ValueError('invalid symbol table')
            symtab = struct.unpack_from('<4I', data, at + 8)
        at += size
    if at != end or symtab is None:
        raise ValueError('incomplete load commands')
    symoff, count, stroff, strsize = symtab
    if symoff + count * 16 > len(data) or stroff + strsize > len(data):
        raise ValueError('symbol table outside Object')
    strings = data[stroff:stroff+strsize]; symbols = []
    for i in range(count):
        string, typ, section, _, value = struct.unpack_from('<IBBHQ', data, symoff+i*16)
        if string >= len(strings):
            raise ValueError('invalid symbol string')
        stop = strings.find(b'\0', string)
        if stop < 0:
            raise ValueError('unterminated symbol string')
        name = strings[string:stop].decode('utf8')
        if not typ & 0xe0 and typ & 0x0e == 0x0e:
            if not 1 <= section <= len(sections):
                raise ValueError('invalid symbol section')
            symbols.append(dict(name=name, section=section, value=value))
    return sections, symbols


def extract(object_bytes, remarks, names):
    if not names or len(names) != len(set(names)) or any(not re.fullmatch(r'[A-Za-z_$][A-Za-z0-9_.$]*', n) for n in names):
        raise ValueError('definition manifest must be a nonempty closed set')
    stacks = llvm_stack_sizes(remarks)
    if set(stacks) != set(names):
        raise ValueError('stack reports do not match definitions: missing=' + str(sorted(set(names)-set(stacks))) + ' extra=' + str(sorted(set(stacks)-set(names))))
    sections, symbols = macho(object_bytes)
    texts = [(i+1,s)for i,s in enumerate(sections)if s['name']=='__text']
    if len(texts) != 1:
        raise ValueError('expected one machine text section')
    text_index, text = texts[0]; definitions = {}; expected = {'_'+n:n for n in names}
    for symbol in symbols:
        if symbol['section'] != text_index:
            continue
        spelling = symbol['name']
        if spelling in expected:
            name = expected[spelling]
            if name in definitions:
                raise ValueError('duplicate machine definition: '+name)
            definitions[name] = symbol['value']
        elif spelling.startswith('_'):
            raise ValueError('unreported machine function: '+spelling)
    if set(definitions) != set(names) or len(set(definitions.values())) != len(names):
        raise ValueError('missing or aliased machine definitions')
    ordered = sorted(definitions.items(), key=lambda x:x[1]); functions = {}
    for i,(name,address) in enumerate(ordered):
        next_address = ordered[i+1][1]if i+1<len(ordered)else text['address']+text['size']
        if not text['address'] <= address < next_address <= text['address']+text['size']:
            raise ValueError('invalid function code extent: '+name)
        functions[name] = dict(stack_bytes=stacks[name],code_bytes=next_address-address,text_offset=address-text['address'])
    # As with Cranelift's total_size, executable constants count as code.
    # Unwind/link metadata is reported separately, not hidden in source data.
    constants = {'__const','__literal4','__literal8','__literal16','__literal_ptr'}
    excluded = {'__compact_unwind','__eh_frame','__debug_line','__debug_info','__debug_abbrev','__debug_str'}
    machine_constant_bytes=0; other=[]
    for s in sections:
        if s['name']=='__text':continue
        if s['name']in constants:machine_constant_bytes+=s['size']
        elif s['name']in excluded or s['segment']=='__DWARF':other.append(s)
        elif s['size']:raise ValueError('unaccounted machine section: '+s['segment']+','+s['name'])
    code_bytes=text['size']+machine_constant_bytes
    if code_bytes>0xffffffffffffffff:
        raise ValueError('machine code-size overflow')
    return dict(schema='mech.llvm.native-machine-facts.v1',object_sha256=hashlib.sha256(object_bytes).hexdigest(),remarks_sha256=hashlib.sha256(remarks.encode()).hexdigest(),definition_count=len(names),functions=functions,text_bytes=text['size'],machine_constant_bytes=machine_constant_bytes,code_bytes=code_bytes,maximum_function_stack_bytes=max(stacks.values()),padding_before_first_function_bytes=ordered[0][1]-text['address'],excluded_link_and_unwind_sections=other,code_extent_policy='Conservative per-function extent includes padding to the next sealed definition; total code includes all machine text and executable constants.',source_or_layout_authority_granted=False)


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--object',type=Path,required=True);p.add_argument('--remarks',type=Path,required=True);p.add_argument('--definitions',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
    names=json.loads(a.definitions.read_text());result=extract(a.object.read_bytes(),a.remarks.read_text(),names);a.output.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(dict(definitions=result['definition_count'],code_bytes=result['code_bytes'],maximum_function_stack_bytes=result['maximum_function_stack_bytes'],source_or_layout_authority_granted=False)))

if __name__=='__main__':main()
