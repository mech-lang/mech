"""Meaningful preparation corruption fixtures; execute only under common lock."""
from copy import deepcopy
import hashlib
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest

import guarded_prepare as preparation
import references


def tar(files):
    output = io.BytesIO()
    with tarfile.open(fileobj=output, mode='w') as archive:
        for name, value in files:
            member = tarfile.TarInfo(name)
            member.size = len(value)
            archive.addfile(member, io.BytesIO(value))
    return output.getvalue()


def hex_archive(alteration=None):
    version, metadata, contents = b'3', b'{synthetic, true}.', b'synthetic package contents'
    digest = hashlib.sha256(version + metadata + contents).hexdigest().upper().encode()
    files = [('VERSION', version), ('CHECKSUM', digest), ('metadata.config', metadata), ('contents.tar.gz', contents)]
    if alteration == 'checksum': files[1] = ('CHECKSUM', b'0' * 64)
    if alteration == 'missing': files.pop()
    if alteration == 'unknown': files.append(('unknown', b'x'))
    if alteration == 'duplicate': files.append(('VERSION', version))
    return tar(files), contents


class CandidatePreparation(unittest.TestCase):
    def test_preheader_input_is_declared_and_reference_ignores_preamble(self):
        cases = dict(preparation.inputs('revcomp', references))
        self.assertEqual(cases['preheader-ignored'], b'ACGT\n>one\nA\n')
        self.assertEqual(references.revcomp(cases['preheader-ignored']), b'>one\nT\n')
        inventory = preparation.preflight_inputs(references)
        self.assertIn('preheader-ignored', inventory['revcomp'])
        self.assertEqual(2 * sum(map(len, inventory.values())), 156)

    def test_complete_authored_inventory_and_explicit_beam_command(self):
        value = json.loads((preparation.HERE / 'profiles.json').read_text())
        rows = preparation.profiles(value)
        self.assertEqual(len(rows), 20)
        self.assertTrue(all('support/clbg_runner.erl' in row['compiled_source_bundle'] for row in rows))
        command = preparation.compile_command(Path('/compiler'), Path('/package'), Path('/out'), Path('/libraries'))
        self.assertIn('--no-dev', command)
        self.assertEqual(command[command.index('--target') + 1], 'erlang')
        runtime = preparation.runtime_command(Path('/application'), Path('/libraries'))
        self.assertEqual(runtime[0], str(preparation.ERL))
        self.assertIn('/application/ebin', runtime)
        self.assertIn('/libraries/gleam_stdlib/ebin', runtime)
        self.assertEqual(runtime[-2:], ['-extra', '{n}'])

    def test_missing_form_foreign_source_and_ffi_module_collision_are_rejected(self):
        original = json.loads((preparation.HERE / 'profiles.json').read_text())
        for alteration in ('missing-form', 'duplicate-id', 'foreign-source', 'main-not-first', 'module-collision', 'boolean-cap'):
            value = deepcopy(original)
            row = value['profiles'][0]
            if alteration == 'missing-form': value['profiles'].pop()
            elif alteration == 'duplicate-id': value['profiles'][1]['id'] = row['id']
            elif alteration == 'foreign-source': row['compiled_source_bundle'].append('../foreign.gleam')
            elif alteration == 'main-not-first': row['compiled_source_bundle'].reverse()
            elif alteration == 'module-collision': row['compiled_source_bundle'].append('support/clbg_io.erl')
            else: row['maximum_compute_workers'] = True
            with self.subTest(alteration=alteration), self.assertRaises(AssertionError):
                preparation.profiles(value)

    def test_hex_inner_checksum_and_exact_outer_inventory_are_required(self):
        source, contents = hex_archive()
        self.assertEqual(preparation.hex_contents(source), contents)
        for alteration in ('checksum', 'missing', 'unknown', 'duplicate'):
            source, _ = hex_archive(alteration)
            with self.subTest(alteration=alteration), self.assertRaises(AssertionError):
                preparation.hex_contents(source)

    def test_archive_traversal_and_links_cannot_become_compiler_inputs(self):
        for alteration in ('traversal', 'absolute', 'symlink', 'duplicate'):
            output = io.BytesIO()
            with tarfile.open(fileobj=output, mode='w') as archive:
                member = tarfile.TarInfo('../foreign' if alteration == 'traversal' else '/absolute' if alteration == 'absolute' else 'entry')
                if alteration == 'symlink':
                    member.type = tarfile.SYMTYPE
                    member.linkname = '../foreign'
                    archive.addfile(member)
                else:
                    member.size = 1
                    archive.addfile(member, io.BytesIO(b'x'))
                    if alteration == 'duplicate': archive.addfile(member, io.BytesIO(b'y'))
            with tempfile.TemporaryDirectory(prefix='mech-gleam-archive-fixture-') as directory:
                with self.subTest(alteration=alteration), self.assertRaises(AssertionError):
                    preparation.extract(output.getvalue(), Path(directory) / 'new')


if False and __name__ == '__main__':
    unittest.main(verbosity=2)


class AdditiveRepair(unittest.TestCase):
    def test_exact_old_compact_tree_records_are_retained_without_mutation(self):
        import proof_reuse
        report=proof_reuse.mock_report()
        import copy
        original=copy.deepcopy(report)
        records=proof_reuse.validate_records(report)
        self.assertEqual(len(records),4)
        self.assertEqual(report,original)

    def test_changed_missing_or_nonboolean_old_record_is_rejected(self):
        import proof_reuse
        for change in (lambda r:r['attempts'].pop(),lambda r:r['attempts'][0].__setitem__('qualified',False),
            lambda r:r['attempts'][0].__setitem__('source_sha256','wrong'),
            lambda r:r['attempts'][0]['qualification_checks'].__setitem__('pins_unchanged',1)):
            report=proof_reuse.mock_report();change(report)
            with self.assertRaises(AssertionError):proof_reuse.validate_records(report)

    def test_exact_six_grouping_pairs_change_only_the_five_reviewed_sources(self):
        import proof_reuse
        proof_reuse.validate_repairs(preparation)
        self.assertEqual(sum(len(v) for v in proof_reuse.REPAIRS.values()),6)
