"""Preserved legacy platform evidence only. Caller holds the common mutex."""
import ast
from copy import deepcopy
import hashlib
import io
import json
import math
from pathlib import Path
import statistics
import tarfile

COUNTS = {'c':18,'cpp':18,'rust':19,'mech':15,'mech-jit':14,'mech-resident':14,'julia':17,'javascript':18}
IDENTITY_FIELDS = ('key','id','benchmark','language','report','source_report','samples','seconds','timing_boundary')
RETIME = 'retime/results/apple-m3-ac-normal-20261008.json'
JIT = 'execution-modes/20261010-final/timings.json'
RESIDENT = 'interpreter-integration/20261011/timings.json'
ARRAY = 'mandelbrot/experiments/20261011-array-state/full-study/report.json'
APPLICATION = 'function-application-audit/20261009T171437Z/application-v3/timings.json'
STATUS = ('qualified-existing-platform-history','reconciled-with-unresolved-existing-platform-history')


class MissingHistory(ValueError): pass


def require(value, message):
    if not value: raise ValueError(message)


def missing(value, message):
    if not value: raise MissingHistory(message)


def digest(value):
    return type(value) is str and len(value)==64 and all(c in '0123456789abcdef' for c in value)


def command(value):
    return type(value) is list and bool(value) and all(type(part) is str and part for part in value)


def pointer(value, path):
    require(type(path) is str and path.startswith('/'), 'Invalid historical JSON pointer')
    for token in path.split('/')[1:]:
        token=token.replace('~1','/').replace('~0','~')
        value=value[int(token)] if type(value) is list else value[token]
    return value


def token(value): return str(value).replace('~','~0').replace('/','~1')


def samples(displayed, saved):
    actual=saved.get('samples_seconds')
    require(type(actual) is list and len(actual)>=3 and all(type(x) in (int,float) and math.isfinite(x) and x>0 for x in actual), 'Invalid actual historical samples')
    require(displayed['samples']==actual, 'Original sample array changed')
    require(type(displayed['seconds']) in (int,float) and math.isfinite(displayed['seconds']) and displayed['seconds']>0,'Invalid original displayed timing')
    if saved.get('median_seconds') is not None:
        require(type(saved['median_seconds']) in (int,float) and displayed['seconds']==saved['median_seconds'], 'Original saved median changed')
    else:
        require(math.isclose(displayed['seconds'],statistics.median(actual),rel_tol=0,abs_tol=1e-10),'Unbound original timing summary')


def inventory(data, expected):
    selected={key:row for key,row in data['programs'].items() if row['language'] in COUNTS}
    require(type(expected) is list and len(expected)==len(set(expected))==133 and set(selected)==set(expected),'Exact legacy133 inventory changed')
    require(all(sum(row['language']==name for row in selected.values())==n for name,n in COUNTS.items()), 'Legacy language counts changed')
    require(all(type(row) is dict and key==row['key']==row['benchmark']+':'+row['id'] for key,row in selected.items()),'Actual mapping/program identity mismatch')
    return selected


class History:
    def __init__(self, root, data, reads, output):
        self.root,self.data,self.reads,self.output=Path(root),data,reads,Path(output)
    def report(self, relative, expected=None):
        require(type(relative) is str and not Path(relative).is_absolute() and '..' not in Path(relative).parts,'Historical report escaped corpus')
        pin=expected or self.data['provenance'].get(relative)
        missing(digest(pin),'missing-preserved-report-pin:'+relative)
        return self.reads.report(relative,pin)
    def version(self, relative, field, value):
        require(pointer(self.report(relative),field)==value,'Runtime receipt pointer changed')
        encoded=value if type(value) is str else json.dumps(value,sort_keys=True)
        missing(type(encoded) is str and bool(encoded.strip()),'missing-cohort-runtime-observation')
        return dict(verified=True,report=relative,report_sha256=self.reads.provenance[relative],field=field,value=encoded,
            value_encoding='literal-saved-string' if type(value) is str else 'json-serialized-preserved-value')
    def source_files(self, displayed, origin):
        files=[self.data['files'][key] for key in displayed['files']]
        require(files and len(displayed['files'])==len(set(displayed['files'])),'Invalid display source bundle')
        for item in files: self.reads.read(self.root/item['path'],item['sha256'])
        values=set()
        if type(origin.get('source_sha256')) is str: values.add(origin['source_sha256'])
        if type(origin.get('sha256')) is str: values.add(origin['sha256'])
        source=origin.get('source')
        if type(source) is dict:
            hashes=source.get('sha256')
            if type(hashes) is dict: values.update(hashes.values())
            elif type(hashes) is str: values.add(hashes)
        missing(values,'missing-historical-authored-source-binding')
        require(all(item['sha256'] in values for item in files),'Authored source/report digest mismatch')
        return [dict(path=item['path'],sha256=item['sha256']) for item in files]
    def producer(self, relative, language, saved):
        report=self.report(relative); snap=report.get('compiler_snapshot')
        missing(type(snap) is dict and digest(snap.get('sha256')),'missing-corresponding-runtime-producer-snapshot')
        archive=str(Path(relative).parent/snap['path'])
        require((self.root/archive).resolve().is_relative_to(self.root.resolve()),'Runtime-producer archive escaped corpus')
        payload=self.reads.read(self.root/archive,snap['sha256'])
        filename='benchmarks/clbg/run_remaining.py'
        expected=snap['files'].get(filename); missing(digest(expected),'missing-runtime-producer-source-pin')
        with tarfile.open(fileobj=io.BytesIO(payload),mode='r:gz') as tar:
            members=[member for member in tar.getmembers() if member.name==filename]
            require(len(members)==1 and members[0].isfile(),'Ambiguous runtime-producer archive source')
            source=tar.extractfile(members[0]).read()
        require(hashlib.sha256(source).hexdigest()==expected,'Runtime-producer source bytes changed')
        binding=observer_binding(source.decode(),language)
        missing(binding is not None,'missing-corresponding-runtime-version-command-binding')
        target=self.output/'producer-snapshots'/snap['sha256']/'run_remaining.py'
        if not target.exists():
            target.parent.mkdir(parents=True,exist_ok=True); target.write_bytes(source); target.chmod(0o444)
        self.reads.read(target,expected)
        return dict(report=relative,archive=archive,archive_sha256=snap['sha256'],source=str(target),source_sha256=expected,**binding)


def observer_binding(source, language):
    """Same preserved producer symbol supplies observation and sample runtime."""
    tree=ast.parse(source)
    functions={node.name:node for node in tree.body if isinstance(node,ast.FunctionDef)}
    if not {'toolchains','prepare'}<=functions.keys(): return None
    tables=[node.value for node in ast.walk(functions['toolchains']) if isinstance(node,ast.Assign)
            and any(isinstance(t,ast.Name) and t.id=='commands' for t in node.targets) and isinstance(node.value,ast.Dict)]
    if len(tables)!=1:return None
    entries={key.value:value for key,value in zip(tables[0].keys,tables[0].values) if isinstance(key,ast.Constant)}
    name='node' if language=='javascript' else 'julia'
    query=entries.get(name)
    if not isinstance(query,ast.List) or len(query.elts)!=2 or not isinstance(query.elts[1],ast.Constant) or query.elts[1].value!='--version':return None
    wanted=ast.dump(query.elts[0],include_attributes=False)
    selected=[]
    for branch in ast.walk(functions['prepare']):
        if not isinstance(branch,ast.If) or not isinstance(branch.test,ast.Compare):continue
        test=branch.test
        if not isinstance(test.left,ast.Name) or test.left.id!='language' or len(test.ops)!=len(test.comparators) or len(test.ops)!=1:continue
        comparator=test.comparators[0]
        exact=isinstance(test.ops[0],ast.Eq) and isinstance(comparator,ast.Constant) and comparator.value==language
        dynamic=language=='javascript' and isinstance(test.ops[0],ast.In) and isinstance(comparator,ast.Name) and comparator.id=='RUNTIME'
        if not (exact or dynamic):continue
        # Only assignments in the selected branch body count. An unrelated
        # fallback or another language branch cannot stand in for its runtime.
        for node in branch.body:
            if not isinstance(node,ast.Assign) or not any(isinstance(t,ast.Name) and t.id=='argv' for t in node.targets) or not isinstance(node.value,ast.List) or not node.value.elts:continue
            value=deepcopy(node.value.elts[0])
            if dynamic and isinstance(value,ast.Subscript) and isinstance(value.value,ast.Name) and value.value.id=='RUNTIME' and isinstance(value.slice,ast.Name) and value.slice.id=='language':
                value.slice=ast.Constant('javascript')
            selected.append(ast.dump(value,include_attributes=False))
    if selected!=[wanted]:return None
    return dict(observer='toolchains.commands.'+name,version_argument='--version',same_preserved_runtime_symbol=wanted,
        selected_sample_branch='language == '+language if language=='julia' else 'language in RUNTIME; javascript selected')


def record_candidates(report, displayed):
    values=report.get('programs')
    if type(values) is list: items=list(enumerate(values))
    elif type(values) is dict: items=list(values.items())
    else:return []
    matches=[]
    for index,row in items:
        if type(row) is not dict:continue
        identity=row.get('id')==displayed['id'] or row.get('key',index)==displayed['key']
        if displayed['language']=='mech-resident':
            identity=row.get('source_key')==displayed['key'].removesuffix(':resident') and row.get('phase')=='after'
        if identity and row.get('language',displayed['language'])==displayed['language'] and row.get('benchmark',report.get('benchmark',displayed['benchmark']))==displayed['benchmark']:
            matches.append((index,row))
    return matches


def build_candidates(value, binary, digest_value, path=''):
    """Exact preserved artifact path+SHA, never a name/suffix lookup."""
    found=[]
    if type(value) is dict:
        if (value.get('binary')==binary and value.get('binary_sha256')==digest_value and
                (value.get('commands') or value.get('compile_command') or value.get('build_command'))):
            found.append((path,value))
        for key,child in value.items():found.extend(build_candidates(child,binary,digest_value,path+'/'+token(key)))
    elif type(value) is list:
        for index,child in enumerate(value):found.extend(build_candidates(child,binary,digest_value,path+'/'+str(index)))
    return found


def measurement(history, displayed):
    relative=displayed['report']; report=history.report(relative)
    if relative==ARRAY:
        manifest_name=displayed['source_report']; manifest=history.report(manifest_name,report['manifest_sha256'])
        require(report['manifest']==str(history.root/manifest_name),'Array-state manifest origin changed')
        found=[(index,row) for index,row in enumerate(manifest['profiles']) if row['key']==displayed['key']]
        require(len(found)==1,'Exact array-state manifest selector missing')
        index,origin=found[0]; saved=report['profiles'][origin['id']]
        samples(displayed,saved)
        attempts=[item for item in report['attempts'] if item['id']==origin['id']]
        require(len(attempts)==len(saved['samples_seconds']) and [item['seconds'] for item in attempts]==saved['samples_seconds'],'Array-state attempt samples changed')
        require(all(item['source_sha256']==origin['source_sha256'] and item['qualified'] is True for item in attempts),'Array-state source/native attempts changed')
        actual_command=[part.replace('{n}',str(manifest['size'])) for part in origin['command']]
        raw_capture=[]
        for item in attempts:
            require(item['command']==actual_command and item['full_workload'] is True and item['valid_environment'] is True,
                'Array-state actual command/full-work binding changed')
            require(type(item['qualification_checks']) is dict and len(item['qualification_checks'])==14 and
                all(flag is True for flag in item['qualification_checks'].values()),'Array-state historical fourteen gates changed')
            group=item['supervised_native_group']
            require(group['command']==item['command'] and type(group['pid']) is int and type(group['pgid']) is int,
                'Array-state native group command changed')
            raw_path=Path(item['result_path'])
            require(raw_path.is_absolute() and raw_path.is_relative_to(history.root),'Array-state raw receipt escaped corpus')
            raw=json.loads(history.reads.read(raw_path))
            require(type(raw) is dict and raw and all(name in item and type(value) is type(item[name]) and value==item[name] for name,value in raw.items()),
                'Array-state original raw/enriched receipt differs')
            raw_capture.append(dict(path=str(raw_path),sha256=history.reads.pins[str(raw_path)],
                capture='newly pinned preserved raw; original report bytes are independently historical-pinned'))
        for path in [origin['source'],origin['binary'],*origin['link']['command'][1:]]:
            if path in manifest['pins'] and Path(path).is_file():history.reads.read(path,manifest['pins'][path])
        return dict(origin=origin,saved=saved,report=report,relative=relative,source_relative=manifest_name,selector='/profiles/'+token(origin['id']),
            command=origin['command'],origin_field='/profiles/'+str(index),raw_capture=raw_capture,
            actual_sample_commands=[item['command'] for item in attempts])
    found=record_candidates(report,displayed); require(len(found)==1,'Exact historical row selector ambiguous or absent: '+displayed['key'])
    index,saved=found[0]; field='/programs/'+token(index)
    if relative==APPLICATION:
        actual=saved['variants']['after']; samples(displayed,actual)
        builds=history.report(displayed['source_report'],report['build_report_sha256'])
        origins=[(i,row) for i,row in enumerate(builds['programs']) if row['key']==displayed['key']]
        require(len(origins)==1,'Application source/build selector absent')
        i,wrapper=origins[0]; origin=wrapper['builds']['after']
        require(origin['binary_sha256']==actual['binary_sha256'],'Application after/build binary differs')
        # Do not synthesize a complete historical argv from a direct binary.
        # If no preserved command receipt is bound, this row stays diagnostic.
        actual_command=actual.get('command') or origin.get('command')
        command_role='historical-timed-argv'
        if not command(actual_command):
            actual_command=native_compile(origin,[origin['binary']])
            command_role='native-build-link'
        return dict(origin=origin,saved=actual,report=report,relative=relative,source_relative=displayed['source_report'],selector=field+'/variants/after',command=actual_command,
            command_role=command_role,missing_historical_timed_argv=command_role=='native-build-link',origin_field='/programs/'+str(i)+'/builds/after')
    samples(displayed,saved); origin=saved; source_relative=relative; origin_field=field
    if not (saved.get('source_sha256') or saved.get('sha256') or type(saved.get('source')) is dict and saved['source'].get('sha256')):
        source_relative=displayed.get('source_report'); missing(type(source_relative) is str,'missing-historical-source-origin')
        prior=history.report(source_relative)
        if type(prior.get('control_sources')) is dict and displayed['id'] in prior['control_sources']:
            origin=prior['control_sources'][displayed['id']]; origin_field='/control_sources/'+token(displayed['id'])
            if displayed['language'] in ('c','cpp','rust'):
                retained=history.report(RETIME)
                original=record_candidates(retained,displayed)
                missing(len(original)==1,'missing-preserved-native-control-build-origin')
                old_index,old=original[0]
                missing(old.get('command')==saved.get('command') and digest(saved.get('binary_sha256'))
                    and retained['asset_hashes'].get(saved['command'][0])==saved['binary_sha256'],
                    'missing-exact-cross-report-native-artifact-reuse-binding')
                historical_sources=old.get('source',{}).get('sha256',{})
                missing(type(historical_sources) is dict and origin['source_sha256'] in historical_sources.values(),
                    'missing-exact-cross-report-native-source-reuse-binding')
                origin=old; source_relative=RETIME; origin_field='/programs/'+token(old_index)
        else:
            candidates=record_candidates(prior,displayed)
            require(len(candidates)==1,'Historical source origin selector missing')
            old_index,origin=candidates[0]; origin_field='/programs/'+token(old_index)
    if displayed['language']=='mech' and not (origin.get('commands') or origin.get('compile_command') or origin.get('build_command')):
        declared=displayed.get('source_report')
        if declared and declared!=relative:
            original_builds=history.report(declared)
            cmd=saved.get('command'); expected=saved.get('binary_sha256') or displayed.get('binary_sha256')
            candidates=build_candidates(original_builds,cmd[0] if command(cmd) else None,expected)
            missing(len(candidates)==1,'missing-or-ambiguous-exact-native-build-origin')
            origin_field,origin=candidates[0];source_relative=declared
    return dict(origin=origin,saved=saved,report=report,relative=relative,source_relative=source_relative,selector=field,command=saved.get('command'),origin_field=origin_field)


def historical_pin(measurement, cmd):
    saved,report,origin=measurement['saved'],measurement['report'],measurement['origin']
    expected=saved.get('binary_sha256') or saved.get('executable_sha256') or origin.get('binary_sha256') or origin.get('executable_sha256')
    aliases=[cmd[0],*saved.get('command_assets',[])]
    assets=report.get('asset_hashes',{})
    if not digest(expected):
        candidates=[(name,assets[name]) for name in aliases if name in assets and name==cmd[0]]
        if len(candidates)==1: path,expected=candidates[0]
        else:return None
    else:path=cmd[0]
    return path,expected


def historical_bytes(history, measurement, cmd):
    bound=historical_pin(measurement,cmd)
    if bound is None:
        # /tmp versus /private/tmp aliases are allowed only when that exact
        # resolved asset was explicitly captured in this measurement's list.
        saved,report=measurement['saved'],measurement['report']
        aliases=[name for name in saved.get('command_assets',[]) if str(Path(name).resolve())==str(Path(cmd[0]).resolve()) and name in report.get('asset_hashes',{})]
        if len(aliases)!=1:return False,dict(reason='missing-historical-runtime-binary-binding',dimension='bytes')
        bound=aliases[0],report['asset_hashes'][aliases[0]]
    path,expected=bound
    if not Path(path).is_file():
        return False,dict(reason='historically-pinned-artifact-unavailable',dimension='bytes',path=path,historical_sha256=expected)
    history.reads.read(path,expected)
    return True,dict(path=path,sha256=expected,role='present corroboration of the exact independently stored historical SHA')


def native_compile(origin,cmd):
    commands=origin.get('commands') or [origin.get('compile_command') or origin.get('build_command') or origin.get('link',{}).get('command')]
    require(type(commands) is list,'Malformed preserved compiler command list')
    for compile_cmd in commands:
        if not command(compile_cmd):continue
        if '-o' in compile_cmd and compile_cmd.index('-o')+1<len(compile_cmd) and compile_cmd[compile_cmd.index('-o')+1]==cmd[0]:
            return compile_cmd
        if compile_cmd[0]=='cargo' and compile_cmd[1:3]==['build','--release'] and '--manifest-path' in compile_cmd:
            # A saved produced executable hash is still mandatory for this route.
            if digest(origin.get('executable_sha256') or origin.get('binary_sha256')):return compile_cmd
    raise MissingHistory('missing-historical-native-build-output-binding')


def annotation(history,displayed,measured,files):
    relative=measured['relative']; origin,saved=measured['origin'],measured['saved']; cmd=measured['command']
    missing(command(cmd),'missing-historical-execution-command')
    language=displayed['language']; evidence=dict(verified=True,report=relative,report_sha256=history.reads.provenance[relative],
        source_sha256=[item['sha256'] for item in files],command=cmd,measurement_selector=measured['selector'],
        actual_sample_array=saved['samples_seconds'],source_receipt=dict(report=measured['source_relative'],field=measured['origin_field'],report_sha256=history.reads.provenance[measured['source_relative']]),
        original_timing_boundary=saved.get('timing_boundary',displayed.get('timing_boundary')))
    evidence['command_role']=measured.get('command_role','historical-timed-argv')
    evidence['missing_historical_timed_argv']=measured.get('missing_historical_timed_argv',False)
    if measured.get('raw_capture'):evidence['preserved_raw_capture']=measured['raw_capture']
    if measured.get('actual_sample_commands'):evidence['actual_sample_commands']=measured['actual_sample_commands']
    artifact_command=[origin['binary']] if evidence['command_role']=='native-build-link' else cmd
    confidence=dict(historical_platform_verified=True,historical_execution_verified=False,historical_executable_bytes_verified=False)
    lacks=[]; label=''; backend='native-runtime'; execution='unknown'
    if evidence['missing_historical_timed_argv']:
        lacks.append(dict(reason='unrecorded-historical-timed-arguments',dimension='timed-argv'))
    if any('{n}' in part for part in cmd):
        evidence['historical_command_template']=True
        if not measured.get('actual_sample_commands'):
            lacks.append(dict(reason='unrecorded-per-sample-expanded-runtime-arguments',dimension='timed-argv'))
    if language in ('mech-jit','mech-resident'):
        mode='jit' if language=='mech-jit' else 'resident'
        missing(saved.get('mode')==mode and len(cmd)>=3 and cmd[2]==mode and cmd[1]==origin['source'],'missing-historical-mode-runtime-source-binding')
        missing(digest(saved.get('binary_sha256')) and saved.get('binary')==cmd[0],'missing-historical-mode-host-binding')
        if mode=='jit':
            objects=history.report('execution-modes/20261010-final/objects.json',measured['report']['optimization_objects_sha256'])
            require(objects['runner_sha256']==saved['binary_sha256'],'Final JIT host differs from original object audit')
        else:
            audit=history.report('interpreter-integration/20261011/codegen-inspection.json')
            require(audit['rows']['after']['binary_sha256']==saved['binary_sha256'],'Resident after host differs from actual native audit')
            history.reads.read(audit['rows']['after']['disassembly_file'],audit['rows']['after']['disassembly_sha256'])
        backend,execution=('native-jit','jit') if mode=='jit' else ('native-interpreter','interpreted')
        label='Mech native JIT' if mode=='jit' else 'Mech loaded typed interpreter'
        confidence['historical_execution_verified']=True
        runtime=history.version(relative,measured['selector']+'/mode',mode)
    elif language in ('julia','javascript'):
        runtime=saved.get('runtime')
        if language=='julia' and relative==RETIME and saved.get('execution_mode')=='julia':
            # This literal is saved runtime-family evidence for the actual
            # retime command/assets, not a JIT mode or a borrowed old version.
            runtime=history.version(relative,measured['selector']+'/execution_mode','julia')
            evidence['runtime_receipt_role']='preserved-runtime-family-only; no historical version or JIT attribution'
        elif type(runtime) is dict:
            missing(runtime.get('path')==cmd[0] and type(runtime.get('version')) is str,'missing-cohort-runtime-observation')
            field=measured['selector']+'/runtime/version'; runtime=history.version(relative,field,runtime['version'])
        else:
            name='javascript' if language=='javascript' and type(measured['report'].get('toolchains',{}).get('javascript')) is dict else ('node' if language=='javascript' else 'julia')
            observed=measured['report'].get('toolchains',{}).get(name)
            if type(observed) is dict:
                missing(observed.get('path')==cmd[0] and type(observed.get('version')) is str,'missing-cohort-runtime-observation')
                runtime=history.version(relative,'/toolchains/'+name+'/version',observed['version'])
                evidence['historical_runtime_pin']=deepcopy(observed)
            elif type(observed) is str:
                evidence['producer_binding']=history.producer(relative,language,saved)
                runtime=history.version(relative,'/toolchains/'+name,observed)
            else:raise MissingHistory('missing-corresponding-runtime-observation')
        backend='javascript-runtime' if language=='javascript' else 'native-runtime'
        label=('Node JavaScript runtime' if language=='javascript' else 'Julia native runtime')
        if runtime['value']!='julia':label+=' · '+runtime['value']
        lacks.append(dict(reason='unrecorded-legacy-runtime-execution-options' if language=='javascript' else 'missing-literal-saved-runtime-mode',dimension='execution'))
    else:
        compile_cmd=native_compile(origin,artifact_command)
        backend,execution,label='native-aot','aot','Native AOT · preserved build/output'
        confidence['historical_execution_verified']=True
        # Actual build command is a preserved structural value, not a new version.
        field_name='commands' if origin.get('commands') else ('compile_command' if origin.get('compile_command') else 'build_command' if origin.get('build_command') else 'link/command')
        value=pointer(origin,'/'+field_name)
        runtime=history.version(measured['source_relative'],measured['origin_field']+'/'+field_name,value)
        evidence['actual_native_build_command']=compile_cmd
    verified_bytes,byte_fact=historical_bytes(history,measured,artifact_command)
    if not verified_bytes and type(saved.get('runtime')) is dict and digest(saved['runtime'].get('sha256')):
        try:
            history.reads.read(cmd[0],saved['runtime']['sha256']); verified_bytes=True; byte_fact=dict(path=cmd[0],sha256=saved['runtime']['sha256'])
        except FileNotFoundError:pass
    if not verified_bytes and type(evidence.get('historical_runtime_pin')) is dict and digest(evidence['historical_runtime_pin'].get('sha256')):
        if Path(cmd[0]).is_file():
            history.reads.read(cmd[0],evidence['historical_runtime_pin']['sha256']); verified_bytes=True; byte_fact=dict(path=cmd[0],sha256=evidence['historical_runtime_pin']['sha256'])
    confidence['historical_executable_bytes_verified']=verified_bytes
    if not verified_bytes:lacks.append(byte_fact)
    else:evidence['historical_executable']=byte_fact
    evidence['runtime_receipt']=runtime
    return dict(key=backend,execution=execution,label=label,verified=True,confidence=confidence,evidence=evidence,
        missing_evidence=list(dict.fromkeys(item['reason'] for item in lacks)),missing_evidence_details=lacks)


def qualify(root,data,expected,reads,output):
    selected=inventory(data,expected); history=History(root,data,reads,output)
    annotations,identities={},{}
    for key,displayed in sorted(selected.items()):
        files=[dict(path=data['files'][file]['path'],sha256=data['files'][file]['sha256']) for file in displayed['files']]
        identity={name:deepcopy(displayed.get(name)) for name in IDENTITY_FIELDS}; identity['files']=files; identities[key]=identity
        for item in files: reads.read(Path(root)/item['path'],item['sha256'])
        history.report(displayed['report'])
        if displayed.get('source_report'):history.report(displayed['source_report'])
        measured=None
        try:
            measured=measurement(history,displayed)
            bound=history.source_files(displayed,measured['origin'])
            annotations[key]=annotation(history,displayed,measured,bound)
        except MissingHistory as error:
            annotations[key]=dict(key='unknown',execution='unknown',label='Unclassified',verified=False,
                confidence=dict(historical_platform_verified=False,historical_execution_verified=False,historical_executable_bytes_verified=False),
                evidence=dict(verified=False,report=displayed['report'],report_sha256=data['provenance'][displayed['report']],source_sha256=[item['sha256'] for item in files],command=measured['command'] or [] if measured else [],
                    runtime_receipt=dict(verified=False,report=displayed['report'],report_sha256=data['provenance'][displayed['report']],field=None,value=None,value_encoding=None)),
                missing_evidence=[str(error)],missing_evidence_details=[dict(dimension='platform',reason=str(error))])
    reads.unchanged()
    return dict(status=STATUS[0] if all(all(value is True for value in item['confidence'].values()) and not item['missing_evidence'] for item in annotations.values()) else STATUS[1],profile_count=133,
        annotations=annotations,measurement_identities=identities,provenance=dict(reads.provenance),pins=dict(reads.pins),
        qualified_platform_rows=sum(item['verified'] is True for item in annotations.values()),
        qualified_execution_rows=sum(item['confidence']['historical_execution_verified'] is True for item in annotations.values()),
        qualified_historical_byte_rows=sum(item['confidence']['historical_executable_bytes_verified'] is True for item in annotations.values()),
        actual_native_processes=0,runtime_version_reruns=0,benchmark_replays=0,chart_admission=False,dataset_mutated=False,sources_mutated=False,timings_mutated=False)
