"""Bind verified v1 compiler/bootstrap and failed first build; no scored successes."""
import copy
import hashlib
import json
from pathlib import Path
import re
import zipfile

HERE=Path(__file__).resolve().parents[2]
PACKET=HERE/'revisions/private-v2'
PRIOR=HERE/'qualification/private-v1'
BUILD=HERE/'.build/private-v1'
PRIOR_PROPOSAL=HERE/'proposals/private-v1.json'
PRIOR_SHA='43438603d675472393050d05928e488a20f16040493935aca676007fc0ada050'


def validate_report(report):
    assert report['status']=='stopped-unqualified' and report['active'] is None
    assert report['proposal_sha256']==PRIOR_SHA and report['attempts']==[] and not report.get('profiles')
    assert report['full_performance_runs']==0 and report['chart_admission'] is False
    assert report['never_chart_eligible'] is True and report['no_automatic_retry'] is True
    assert report['error_type']=='AssertionError' and report['error']=="('Owned command failed', 'build', 1)"


def validate_sources():
    old=json.loads((HERE/'profiles.json').read_text());actual=json.loads((PACKET/'profiles.json').read_text())
    expected=copy.deepcopy(old)
    for row in expected['profiles']:
        if row['benchmark']=='regexredux':
            original=row['source'];corrected='revisions/private-v2/'+original
            row['source']=corrected;row['compiled_source_bundle']=[corrected if p==original else p for p in row['compiled_source_bundle']]
    assert actual==expected
    for variant in ('compact','optimized'):
        old=(HERE/'sources/regexredux'/(variant+'.c3')).read_text()
        assert old.count('libc::strlen(pattern)')==old.count('libc::strlen(replacement)')==1
        fixed=old.replace('libc::strlen(pattern)','libc::strlen((ZString)pattern)').replace('libc::strlen(replacement)','libc::strlen((ZString)replacement)')
        assert (PACKET/'sources/regexredux'/(variant+'.c3')).read_text()==fixed


def archive_correspondence(preparation,archive):
    pins={};names=set();root=BUILD/'c3'
    with zipfile.ZipFile(archive) as bundle:
        for entry in bundle.infolist():
            target=root/entry.filename
            assert target.resolve().is_relative_to(root.resolve()) and entry.filename not in names
            names.add(entry.filename)
            mode=entry.external_attr>>16
            if (mode & 0o170000)==0o120000:
                assert target.is_symlink() and target.readlink()==Path(bundle.read(entry).decode())
            elif entry.is_dir():assert target.is_dir()
            else:
                digest=hashlib.sha256(bundle.read(entry)).hexdigest();assert preparation.sha(target)==digest
                pins[str(target)]=digest;pins[str(target.resolve())]=digest
    assert pins
    return pins


def reconstruct(preparation,guard):
    validate_sources()
    assert preparation.sha(PRIOR_PROPOSAL)==PRIOR_SHA
    proposal=json.loads(PRIOR_PROPOSAL.read_text());guard.verify(proposal['pins'])
    report_path=PRIOR/'report.json';validate_report(json.loads(report_path.read_text()))
    archive=BUILD/'c3-macos.zip'
    assert preparation.sha(archive)==proposal['compiler_archive_sha256']==preparation.C3_SHA
    candidates=[p for p in (BUILD/'c3').rglob('c3c') if p.is_file()];assert len(candidates)==1
    compiler=candidates[0];stdlib=compiler.parent/'lib';sdk=Path(proposal['sdk'])
    assert (stdlib/'std/libc/libc.c3').is_file()
    version=(PRIOR/'bootstrap/c3-version.stdout').read_text()
    assert re.search(r'^C3 Compiler Version:\s+0\.8\.5\s*$',version,re.M)
    commit=re.search(r'^Git Hash:\s+(\w+)',version,re.M);assert commit and commit[1]==preparation.C3_COMMIT
    llvm=re.search(r'^LLVM version:\s+(\d+\.\d+(?:\.\d+)?)',version,re.M)
    assert llvm and re.search(r'^Backends:\s+.*LLVM',version,re.M)
    version_receipt=json.loads((PRIOR/'bootstrap/c3-version.json').read_text())
    assert version_receipt['command']==[str(compiler),'--version']
    assert version_receipt['exit_status']==version_receipt['final_exit_status']==0 and version_receipt['inherited_common_mutex'] is True
    assert not any(version_receipt.get(k) for k in ('error','cleanup_error','cleanup_errors','output_drain_error'))
    actual=json.loads((PRIOR/'bootstrap/actual-toolchain.json').read_text())
    assert actual['compiler']==str(compiler) and actual['compiler_sha256']==preparation.sha(compiler)
    assert actual['compiler_version']==version and actual['bundled_llvm_version']==llvm[1]
    assert actual['explicit_backend']=='llvm' and actual['explicit_target']=='macos-aarch64'
    assert actual['linker']==str(preparation.CLANG) and actual['sdk']==str(sdk)
    pins=preparation.merge(proposal['pins'],{str(PRIOR_PROPOSAL):PRIOR_SHA,str(report_path):preparation.sha(report_path),str(archive):preparation.sha(archive)},
        archive_correspondence(preparation,archive),preparation.pin_tree(BUILD/'c3'),preparation.pin_tree(PRIOR/'bootstrap'),
        preparation.pin_tree(PRIOR/'source-snapshot'),preparation.pin_tree(PRIOR/'builds'),preparation.pin_tree(BUILD/'cache'))
    images=PRIOR/'bootstrap/images'
    assert actual['compiler_host_image_review']==str(images/'dependency-review.json')
    review=json.loads((images/'dependency-review.json').read_text())
    assert review['binary']==str(compiler) and (images/'architecture.stdout').read_text().split()==review['architectures']
    assert 'arm64' in review['architectures']
    for name,command in {'architecture':['/usr/bin/lipo','-archs',str(compiler)],'linked-images':['/usr/bin/otool','-L',str(compiler)]}.items():
        receipt=json.loads((images/(name+'.json')).read_text())
        assert receipt['command']==command and receipt['exit_status']==receipt['final_exit_status']==0
        assert receipt['inherited_common_mutex'] is True and not any(receipt.get(k) for k in ('error','cleanup_error','cleanup_errors','output_drain_error'))
    linked=[line.strip().split(' (',1)[0] for line in (images/'linked-images.stdout').read_text().splitlines()[1:]]
    assert linked==[d['install_name'] for d in review['dependencies']]
    for dep in review['dependencies']:
        if dep['install_name'].startswith(('/usr/lib/','/System/Library/')):
            assert dep==dict(install_name=dep['install_name'],scope='OS-image-not-hashed')
        else:
            lookup=Path(dep['lookup_path']);target=lookup.resolve(strict=True)
            assert str(target)==dep['path'] and any(target.is_relative_to(r.resolve()) for r in (compiler.parent,preparation.GMP,preparation.PCRE,preparation.CLANG.parent.parent))
            assert preparation.sha(lookup)==preparation.sha(target)==dep['sha256']
            pins=preparation.merge(pins,{str(lookup):dep['sha256'],str(target):dep['sha256']})
    failed=PRIOR/'builds/c3-regexredux-compact';receipt=json.loads((failed/'build.json').read_text())
    declaration=proposal['profiles'][0];assert declaration['id']=='c3-regexredux-compact'
    sources=[PRIOR/'source-snapshot'/name for name in declaration['compiled_source_bundle']]
    command=preparation.build_command(compiler,stdlib,sdk,sources,BUILD/'bin/c3-regexredux-compact',BUILD/'cache/c3-regexredux-compact',create_cache=False)
    assert receipt['command']==command and receipt['exit_status']==receipt['final_exit_status']==1
    assert receipt['error_type']=='AssertionError' and receipt['error']=="('Owned command failed', 'build', 1)" and receipt['inherited_common_mutex'] is True
    assert not any(receipt.get(k) for k in ('cleanup_error','cleanup_errors','output_drain_error'))
    stderr=(failed/'build.stderr').read_text()
    assert "Implicitly casting 'char*' to 'ZString' is not permitted" in stderr and 'libc::strlen(pattern)' in stderr
    assert sources[0].read_bytes()==(HERE/'sources/regexredux/compact.c3').read_bytes()
    guard.verify(pins)
    return dict(status='verified-exact-C3-v1-bootstrap-and-first-build-failure',pins=pins,
        prior_report=str(report_path),prior_report_sha256=pins[str(report_path)],prior_proposal_sha256=PRIOR_SHA,
        toolchain=dict(compiler=str(compiler),stdlib=str(stdlib),sdk=str(sdk),bundled_llvm_version=llvm[1],actual_toolchain=actual),
        failed_build=dict(receipt=str(failed/'build.json'),stderr=str(failed/'build.stderr'),source=str(sources[0])),
        reused_bounded_cases=0,new_scored_case_identity_claims=0,full_performance_runs=0,never_chart_eligible=True)


def validate_packet(packet,preparation,guard):
    guard.verify(packet['pins']);assert packet==reconstruct(preparation,guard)
    return packet
