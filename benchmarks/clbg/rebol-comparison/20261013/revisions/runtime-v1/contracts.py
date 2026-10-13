"""Unexecuted bounded private-runtime contracts; never benchmark admission."""
from pathlib import Path, PurePosixPath
import posixpath

TAG = '3.22.1'
COMMIT = '7fe158a9a9b0be8826c35a27596cb3c207b5ed6c'
RUNTIME_URL = 'https://github.com/Oldes/Rebol3/releases/download/3.22.1/rebol3-bulk-macos-arm64.gz'
RUNTIME_SHA = '6f078cfba45dcfc43502fee6795361180ce0f1ed6964521278a488613408311a'
SOURCE_URL = 'https://github.com/Oldes/Rebol3/archive/' + COMMIT + '.tar.gz'
SOURCE_PREFIX = 'Rebol3-' + COMMIT
SOURCE_REQUIRED = [
    '.version', 'README.md', 'make/rebol3.nest', 'make/pre-make.r3',
    'make/tools/form-header.reb', 'make/tools/utils.reb', 'make/tools/map-conv.reb',
    'make/tools/make-headers.reb', 'make/tools/make-boot.reb',
    'make/tools/make-host-init.reb', 'make/tools/make-host-ext.reb',
    'make/tools/make-os-ext.reb', 'make/tools/make-reb-lib.reb',
    'src/core/a-lib.c', 'src/core/f-extension.c', 'src/core/b-init.c',
    'src/core/n-system.c', 'src/core/m-gc.c', 'src/core/p-console.c',
    'src/os/posix/dev-stdio.c', 'src/os/posix/dev-file.c', 'src/core/p-file.c',
    'src/include/reb-ext.h', 'src/include/reb-host.h',
    'src/boot/types-ext.reb', 'src/boot/typespec.reb', 'src/boot/sysobj.reb',
    'src/boot/strings.reb', 'src/boot/actions.reb', 'src/mezz/sys-start.reb',
    'src/tests/extension/test.c',
]
IDS = [
    'test_runtime.RuntimeContracts.test_seal_cannot_authorize_downloads_or_native_work',
    'test_runtime.RuntimeContracts.test_publisher_runtime_and_full_commit_source_identity',
    'test_runtime.RuntimeContracts.test_source_members_reject_traversal_links_duplicates_and_wrong_root',
    'test_runtime.RuntimeContracts.test_os_images_reject_escaped_and_private_install_names',
    'test_runtime.RuntimeContracts.test_startup_closure_requires_private_empty_inputs',
    'test_runtime.RuntimeContracts.test_observed_interpretation_gc_and_regular_bytes_never_prove_pipe_io',
]


def require(condition, message):
    if not condition:
        raise ValueError(message)


def identity(plan):
    require(plan['runtime_url'] == RUNTIME_URL and plan['publisher_runtime_sha256'] == RUNTIME_SHA and
            plan['source_url'] == SOURCE_URL and plan['source_commit'] == COMMIT and plan['tag'] == TAG and
            plan['source_archive_prior_sha256'] is None,
            'Exact publisher gzip and full-commit source route required; no invented archive digest')


def authorize(mode, proposal=None):
    require(mode in {'seal','run'}, 'Unknown runtime phase')
    if mode == 'seal':
        require(proposal is None, 'First seal has no runtime authorization')
        return dict(downloads=False, native_commands=False, header_generation=False, full_processes=0)
    identity(proposal)
    require(proposal['status'] == 'sealed-awaiting-root-review' and
            proposal['runtime_configuration_admitted'] is False and
            proposal['source_work_qualified'] is False and type(proposal['full_processes_authorized']) is int and
            proposal['full_processes_authorized'] == 0,
            'Only reviewed metadata seal can authorize bounded runtime discovery')
    receipt = proposal['fixture_receipt']
    require(receipt['status'] == 'passed' and receipt['execution_completed'] is True and
            receipt['inherited_common_mutex'] is True and receipt['expected_test_ids'] == IDS,
            'Actual local runtime fixtures required')
    for key in ('discovered_tests','tests_run'):
        require(type(receipt[key]) is int and receipt[key] == len(IDS), 'Exact fixture count required')
    for key in ('discovered_test_ids','actual_test_ids'):
        require(len(receipt[key]) == len(set(receipt[key])) == len(IDS) and set(receipt[key]) == set(IDS),
                'Exact actual discovered and started IDs required')
    require(all(receipt[key] == [] for key in ('failures','errors','skips','expected_failures','unexpected_successes')),
            'Skipped/failed/unknown fixtures cannot authorize runtime discovery')
    return dict(downloads=True, native_commands=True, header_generation=False, full_processes=0)


def source_member(name, kind, seen):
    require(type(name) is str and name and not any(c in name for c in ('\\','\x00','\n','\r')),
            'Invalid source archive path')
    name = name.rstrip('/') if kind == 'directory' else name
    path = PurePosixPath(name)
    require(not path.is_absolute() and all(part not in ('.','..') for part in name.split('/')) and
            path.parts[0] == SOURCE_PREFIX and path.as_posix() == name,
            'Source member escapes its exact full-commit root')
    require(name not in seen and kind in {'directory','regular'}, 'Duplicate or linked/device source member')
    seen.add(name)
    return Path(*path.parts[1:])


def os_image(install):
    require(type(install) is str and install.startswith('/') and '\x00' not in install and
            posixpath.normpath(install) == install and
            not any(part in ('.','..') for part in install.split('/')),
            'Unnormalized/escaped image cannot receive OS exemption')
    require(install.startswith(('/usr/lib/','/System/Library/')), 'Private runtime dependency needs separate image closure')
    return dict(install_name=install, scope='OS-shared-cache-image-not-hashed')


def startup_inventory(runtime_directory, private_data):
    runtime_directory = Path(runtime_directory).resolve(strict=True)
    private_data = Path(private_data).resolve(strict=True)
    require(runtime_directory != private_data and
            not (runtime_directory/'rebol.reb').exists() and not (runtime_directory/'rebol.reb').is_symlink() and
            not (private_data/'user.reb').exists() and not (private_data/'user.reb').is_symlink(),
            'Unreviewed startup script present')
    files = [str(path.relative_to(private_data)) for path in private_data.rglob('*') if path.is_file() or path.is_symlink()]
    require(files == [], 'Private data/module directory contains unreviewed inputs')
    return dict(runtime_adjacent_rebol_reb_absent=True, private_user_reb_absent=True,
                private_data_files=[], rebol_home=str(private_data), runtime_directory=str(runtime_directory))


def private_target(cohort, destination):
    cohort = Path(cohort).resolve(strict=True)
    destination = Path(destination)
    require(destination != cohort and destination.is_relative_to(cohort) and
            destination.resolve(strict=False).is_relative_to(cohort),
            'Prospective private runtime directory escapes its cohort')
    ancestor = destination
    while ancestor != cohort:
        require(not ancestor.is_symlink(), 'Symlinked private runtime ancestor')
        ancestor = ancestor.parent
    return destination


def observations(fields, regular_bytes):
    require(fields['version'] == TAG and fields['platform'] == 'macOS' and
            fields['function_is_interpreted_value'] is True and fields['function_is_native_value'] is False and
            fields['dynamic_rebinding'] is True and fields['fresh_block_identity'] is True,
            'Actual selected runtime interpretation/type/identity probe required')
    before, after = fields['automatic_gc_before'], fields['automatic_gc_after']
    require(type(before) is int and type(after) is int and 0 <= before < after,
            'Automatic default GC counter must advance without a manual policy change')
    require(regular_bytes == bytes(range(256)), 'Complete regular-file byte output differs')
    return dict(interpreted_function_value_observed=True, automatic_default_gc_observed=True,
        regular_file_bytes_verified=True, pipe_stdin_stdout_bytes_verified=False,
        extension_abi_verified=False, generated_headers_verified=False,
        native_application_aot_proven=False, native_machine_code_jit_proven=False,
        runtime_configuration_admitted=False, source_work_qualified=False, full_processes_authorized=0)
