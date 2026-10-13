"""Six meaningful source-only contracts, qualified later under the common lock."""
from copy import deepcopy
from pathlib import Path
import tempfile
import unittest
import contracts as c


def proposal():
    return dict(status='sealed-awaiting-root-review',runtime_configuration_admitted=False,
        source_work_qualified=False,full_processes_authorized=0,tag=c.TAG,source_commit=c.COMMIT,
        runtime_url=c.RUNTIME_URL,publisher_runtime_sha256=c.RUNTIME_SHA,source_url=c.SOURCE_URL,
        source_archive_prior_sha256=None,fixture_receipt=dict(status='passed',execution_completed=True,
            inherited_common_mutex=True,expected_test_ids=c.IDS,discovered_tests=6,tests_run=6,
            discovered_test_ids=c.IDS[:],actual_test_ids=c.IDS[:],failures=[],errors=[],skips=[],
            expected_failures=[],unexpected_successes=[]))


class RuntimeContracts(unittest.TestCase):
    def test_seal_cannot_authorize_downloads_or_native_work(self):
        self.assertEqual(c.authorize('seal'),dict(downloads=False,native_commands=False,header_generation=False,full_processes=0))
        self.assertEqual(c.authorize('run',proposal()),dict(downloads=True,native_commands=True,header_generation=False,full_processes=0))
        changes=[lambda p:p.update(status='draft'),lambda p:p.update(runtime_configuration_admitted=True),
            lambda p:p.update(full_processes_authorized=1),lambda p:p['fixture_receipt']['actual_test_ids'].pop(),
            lambda p:p.update(full_processes_authorized=False),
            lambda p:p['fixture_receipt'].update(tests_run=True),lambda p:p['fixture_receipt']['skips'].append('skip')]
        for change in changes:
            p=proposal(); change(p)
            with self.assertRaises(ValueError):c.authorize('run',p)
        with self.assertRaises(ValueError):c.authorize('seal',proposal())

    def test_publisher_runtime_and_full_commit_source_identity(self):
        c.identity(proposal())
        for key,value in [('runtime_url',c.RUNTIME_URL.replace('arm64','x64')),
                ('publisher_runtime_sha256','0'*64),('source_commit',c.COMMIT[:7]),
                ('source_url',c.SOURCE_URL.replace(c.COMMIT,'master')),('source_archive_prior_sha256','0'*64)]:
            p=proposal();p[key]=value
            with self.subTest(key=key),self.assertRaises(ValueError):c.identity(p)

    def test_source_members_reject_traversal_links_duplicates_and_wrong_root(self):
        seen=set();prefix=c.SOURCE_PREFIX
        self.assertEqual(c.source_member(prefix+'/src/core/a-lib.c','regular',seen),Path('src/core/a-lib.c'))
        for name,kind in [(prefix+'/../escape','regular'),('/'+prefix+'/x','regular'),
                (prefix+'/src/./a','regular'),(prefix+'/src/core/a-lib.c','regular'),
                ('Rebol3-master/src/a','regular'),(prefix+'/link','symlink'),(prefix+'/hard','hardlink'),
                (prefix+'/device','device'),(prefix+'/bad\\name','regular')]:
            with self.subTest(name=name,kind=kind),self.assertRaises(ValueError):c.source_member(name,kind,seen)

    def test_os_images_reject_escaped_and_private_install_names(self):
        self.assertEqual(c.os_image('/usr/lib/libSystem.B.dylib')['scope'],'OS-shared-cache-image-not-hashed')
        for path in ['/usr/lib/../../tmp/private.dylib','/System/Library/../private.dylib',
                '@rpath/private.dylib','/opt/homebrew/lib/private.dylib','/usr/lib//libSystem.B.dylib']:
            with self.subTest(path=path),self.assertRaises(ValueError):c.os_image(path)

    def test_startup_closure_requires_private_empty_inputs(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory).resolve();cohort=root/'cohort';outside=root/'outside'
            cohort.mkdir();outside.mkdir();(cohort/'.build').symlink_to(outside,target_is_directory=True)
            with self.assertRaises(ValueError):c.private_target(cohort,cohort/'.build/runtime-v1')
            self.assertFalse((outside/'runtime-v1').exists())
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);runtime=root/'runtime';data=root/'data';runtime.mkdir();data.mkdir()
            self.assertTrue(c.startup_inventory(runtime,data)['private_user_reb_absent'])
            for target in (runtime/'rebol.reb',data/'user.reb',data/'modules'/'foreign.reb'):
                target.parent.mkdir(exist_ok=True);target.write_text('synthetic, never evaluated')
                with self.assertRaises(ValueError):c.startup_inventory(runtime,data)
                target.unlink()

    def test_observed_interpretation_gc_and_regular_bytes_never_prove_pipe_io(self):
        fields=dict(version='3.22.1',platform='macOS',function_is_interpreted_value=True,
            function_is_native_value=False,dynamic_rebinding=True,fresh_block_identity=True,
            automatic_gc_before=1,automatic_gc_after=2)
        result=c.observations(fields,bytes(range(256)))
        self.assertIs(result['pipe_stdin_stdout_bytes_verified'],False)
        self.assertIs(result['extension_abi_verified'],False)
        for key,value in [('version','3.22.5'),('platform','Linux'),('function_is_native_value',True),
                ('function_is_interpreted_value',1),('automatic_gc_after',1),('automatic_gc_before',True)]:
            changed=deepcopy(fields);changed[key]=value
            with self.subTest(key=key),self.assertRaises(ValueError):c.observations(changed,bytes(range(256)))
        with self.assertRaises(ValueError):c.observations(fields,bytes(range(255)))
