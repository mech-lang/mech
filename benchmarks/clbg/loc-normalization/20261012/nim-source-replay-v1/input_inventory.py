"""Bounded original Nim frontend command and Processing evidence adapter."""
from pathlib import Path
import re

AUDIT_FLAGS = ('--compileOnly:on','--processing:filenames','--filenames:abs','--colors:off','--incremental:off')
PROCESSING = re.compile(r'^Hint:\s*(?P<depth>>*)\s*(?P<caller>[^:]+): (?P<operation>import|include): (?P<path>/.*) \[Processing\]$')


def audit_command(original, cache, output):
    assert type(original) is list and len(original) > 5 and all(type(value) is str for value in original)
    assert original[1] == 'c' and Path(original[0]).is_absolute() and Path(original[-1]).is_absolute()
    assert Path(original[-1]).suffix == '.nim'
    assert all(Path(path).is_absolute() for path in (cache,output))
    cache_indices = [index for index,value in enumerate(original) if value.startswith('--nimcache:')]
    output_indices = [index for index,value in enumerate(original) if value.startswith('--out:')]
    assert len(cache_indices) == len(output_indices) == 1
    assert not any(value.lower().startswith(('--compileonly:','--processing:','--filenames:','--colors:','--incremental:','--symbolfiles:')) for value in original)
    command = list(original)
    command[cache_indices[0]] = '--nimcache:'+str(cache)
    command[output_indices[0]] = '--out:'+str(output)
    command[-1:-1] = AUDIT_FLAGS
    restored = [value for value in command if value not in AUDIT_FLAGS]
    restored[cache_indices[0]],restored[output_indices[0]] = original[cache_indices[0]],original[output_indices[0]]
    assert restored == original
    return command


def processing_inputs(text,main,helpers,vendor_roots):
    """Bind every logged import/include; unknown roots or omitted glue reject."""
    assert type(text) is str and '\x1b' not in text
    main = Path(main).resolve(); helpers = [Path(path).resolve() for path in helpers]
    assert main.is_absolute() and len(helpers) == len(set(helpers)) and main not in helpers
    vendors = [Path(path).resolve() for path in vendor_roots]
    assert vendors and all(path.is_absolute() for path in vendors)
    authored = {main:'application',**{path:'application-helper' for path in helpers}}
    events,files = [],{}
    for line_number,line in enumerate(text.splitlines(),1):
        if '[Processing]' not in line: continue
        matched = PROCESSING.fullmatch(line)
        assert matched is not None, ('Unknown/cached/config frontend event',line)
        path = Path(matched['path']); assert path.is_absolute() and path.suffix == '.nim'
        resolved = path.resolve()
        if resolved in authored: role = authored[resolved]
        else:
            assert any(resolved.is_relative_to(root) for root in vendors), ('Unknown original compiler input',str(path))
            role = 'vendor-standard-or-general-library'
        event = dict(path=str(path),resolved_path=str(resolved),role=role,operation=matched['operation'],
            caller=matched['caller'].strip(),depth=len(matched['depth']),raw_line=line,line=line_number)
        events.append(event)
        key = str(resolved)
        if key not in files: files[key] = dict(path=key,role=role,counted=resolved in authored,lookup_aliases=[],event_indices=[])
        if str(path) not in files[key]['lookup_aliases']: files[key]['lookup_aliases'].append(str(path))
        files[key]['event_indices'].append(len(events)-1)
    assert events and {Path(path) for path,value in files.items() if value['counted']} == set(authored)
    assert any(event['resolved_path'] == str(main) and event['operation'] == 'import' for event in events)
    return dict(events=events,files=list(files.values()),authored_files=[files[str(path)] for path in authored],
        complete_import_and_include_events=True,unknown_roots=[],generated_source_substitute=False)


def completed_owned_receipt(value,command):
    assert type(value) is dict and value['command'] == command
    assert value['inherited_common_mutex'] is value['preparation_only'] is value['never_chart_eligible'] is True
    assert type(value['exit_status']) is type(value['final_exit_status']) is int
    assert value['exit_status'] == value['final_exit_status'] == 0
    assert not any(value.get(name) for name in ('error','cleanup_error','cleanup_errors','remaining_group_members','output_drain_error'))
    return True
