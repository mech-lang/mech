"""Build source-extracted Mech sort replays and two C algorithm arms."""
import argparse
from hashlib import sha256
import importlib.util
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess


def digest(path):
    return sha256(Path(path).read_bytes()).hexdigest()


def module(here, name):
    spec = importlib.util.spec_from_file_location(name, here / (name + ".py"))
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


def text_size(path):
    data = Path(path).read_bytes()
    assert struct.unpack_from('<I', data)[0] == 0xfeedfacf
    at = 32
    for _ in range(struct.unpack_from('<I', data, 16)[0]):
        command, size = struct.unpack_from('<II', data, at)
        if command == 0x19:
            for index in range(struct.unpack_from('<I', data, at+64)[0]):
                section = at+72+index*80
                if data[section:section+16].split(b'\0')[0] == b'__text':
                    return struct.unpack_from('<Q', data, section+40)[0]
        at += size
    raise AssertionError('No text section')


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('config', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    here, out = Path(__file__).resolve().parent, args.output.resolve()
    config = json.loads(args.config.read_text())
    assert all(digest(p) == pin for p, pin in config['inputs_sha256'].items())
    out.mkdir(parents=True, exist_ok=False)
    (out / 'config.json').write_text(args.config.read_text())
    generator, c_generator = module(here, 'generate'), module(here, 'generate_c')
    (out / 'sort.c').write_text(c_generator.generate(Path(config['C_source']).read_text()))
    shutil.copyfile(here / 'worker.rs', out / 'worker.rs')
    shutil.copyfile(config['allocator_source'], out / 'pinned_allocator.rs')
    builds, roles = [], {}

    def run(command, label, flags=None):
        environment = {k: v for k, v in os.environ.items() if not k.startswith('MECH_')}
        environment.update(flags or {})
        with (out / (label+'.log')).open('w') as log:
            subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, env=environment, check=True)
        builds.append({'label':label, 'command':command, 'flags':flags or {},
                       'log_sha256':digest(out / (label+'.log'))})

    for version, value in [('timing', '0'), ('counts', '1')]:
        run(['/usr/bin/clang', '-arch', 'arm64', '-std=gnu11', '-O2', '-Wall', '-Werror',
             '-DSORT_COUNTERS='+value, '-c', str(out/'sort.c'), '-o', str(out/('sort-'+version+'.o'))],
            'C-'+version)
    for role in ('shell', 'bucket'):
        source = out / (role+'.mec')
        source.write_text(generator.generate(Path(config['codec_source']).read_text(), role))
        census = out / (role+'-homes.json')
        command = [config['compiler'], str(source), str(out/(role+'-census')), 'compact']
        run(command, role+'-census', config['flags'] | {'MECH_COMPACT_DIRECT_SCALAR_HOME_CENSUS':str(census)})
        homes = [v['slot'] for v in json.loads(census.read_text()) if v['admitted']]
        flags = dict(config['flags'])
        if homes:
            flags['MECH_COMPACT_DIRECT_SCALAR_HOMES'] = ','.join(map(str, homes))
        native = out / role
        run([config['compiler'], str(source), str(native), 'compact'], role+'-native', flags)
        checked = out / (role+'-checked')
        run([config['compiler'], str(source), str(checked), 'compact'], role+'-checked',
            flags | {'MECH_COMPACT_PAIR_REGISTER_SPILLS':'checked'})
        artifacts = {name:digest(native/name) for name in ('program.o','metadata.json','program.mecb')}
        assert artifacts == {name:digest(checked/name) for name in artifacts}
        roles[role] = {'source':str(source),'source_sha256':digest(source),'flags':flags,
            'admitted_direct_homes':len(homes),'artifacts':artifacts,'text_bytes':text_size(native/'program.o'),
            'normal_checked_exact':True,'metadata':str(native/'metadata.json')}
        for version in (('timing','counts') if role == 'shell' else ('timing',)):
            command = list(config['selected_link'])
            command[command.index(config['worker_source'])] = str(out/'worker.rs')
            command[command.index('-o')+1] = str(out/(role+'-'+version+'-worker'))
            old_object = 'link-arg='+config['selected_object']
            command[command.index(old_object)] = 'link-arg='+str(native/'program.o')
            command += ['-C','link-arg='+str(out/('sort-'+version+'.o'))]
            run(command, role+'-'+version+'-worker')
            roles[role][version+'_worker'] = command[command.index('-o')+1]
            roles[role][version+'_worker_sha256'] = digest(roles[role][version+'_worker'])
    cases = []
    for name in config['cases']:
        trace = json.loads((Path(config['trace_directory'])/(name+'.json')).read_text())
        vectors = []
        for index, vector in enumerate(trace['frequency_vectors']):
            freqs = vector['freqs']
            assert len(freqs) in (19,32,288) and sum(freqs) < 1<<22
            expected = sorted((frequency<<10)|sym for sym,frequency in enumerate(freqs) if frequency)
            vectors.append({'freqs':freqs,'expected':expected,'captured_vector':index,
                'stage':vector['stage'],'purpose':vector['purpose'],'block':vector['block']})
        cases.append({'name':name,'vectors':vectors})
    assert sum(len(case['vectors']) for case in cases) == 212
    (out/'vectors.json').write_text(json.dumps(cases,indent=2)+'\n')
    result = {'config_sha256':digest(args.config),'builds':builds,'roles':roles,
        'vectors_sha256':digest(out/'vectors.json'),'vectors':212,'cases':len(cases),
        'C_source_sha256':digest(out/'sort.c'),'C_objects':{v:digest(out/('sort-'+v+'.o')) for v in ('timing','counts')},
        'script_inputs':{p.name:digest(p) for p in here.iterdir() if p.suffix in ('.py','.rs')},
        'scope':'Standalone source-extracted phases on actual pinned C frequency inputs; fresh scratch allocation through cleanup, packed-key result copy included equally. Not shipping helper code or whole-call qualification.',
        'complete':True}
    (out/'build.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({'complete':True,'vectors':212,'roles':{k:{a:b for a,b in v.items() if a in ['text_bytes','admitted_direct_homes','normal_checked_exact']} for k,v in roles.items()}},indent=2),flush=True)


if __name__ == '__main__':
    main()
