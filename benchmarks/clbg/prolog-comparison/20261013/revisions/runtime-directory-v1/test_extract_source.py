"""Four source-authored archive contracts; run only in the common queue."""
import io
from pathlib import Path
import tarfile
import tempfile
import unittest
from extract_source import extract_source


def member(name, kind=tarfile.DIRTYPE, *, link='', mode=0o775, mtime=1, uid=0, gid=0, data=b''):
    value = tarfile.TarInfo(name)
    value.type, value.linkname, value.mode, value.mtime = kind, link, mode, mtime
    value.uid, value.gid = uid, gid
    value.size = len(data) if kind == tarfile.REGTYPE else 0
    return value, data


def archive(root, members):
    path = root/'source.tar.gz'
    with tarfile.open(path, 'w:gz') as output:
        for info, data in members: output.addfile(info, io.BytesIO(data) if info.isfile() else None)
    return path


class DirectoryContracts(unittest.TestCase):
    def test_repeated_exact_directory_identity_allows_only_unapplied_timestamp_difference(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            source = archive(root, [member('root/bench', mtime=7),
                member('root/bench', mtime=19), member('root/bench/source.pl', tarfile.REGTYPE, data=b'original source\n')])
            result = extract_source(source, root/'output')
            self.assertEqual((root/'output/root/bench/source.pl').read_bytes(), b'original source\n')
            self.assertEqual(result['repeated_directory_count'], 1)
            self.assertEqual(result['repeated_directories'][0]['first_mtime'], 7)
            self.assertEqual(result['repeated_directories'][0]['repeat_mtime'], 19)
            self.assertFalse(result['timestamps_applied'])

    def test_regular_link_and_cross_type_duplicates_never_overwrite(self):
        cases = [
            [member('root/item', tarfile.REGTYPE, data=b'one'), member('root/item', tarfile.REGTYPE, data=b'two')],
            [member('root/item'), member('root/item', tarfile.REGTYPE, data=b'two')],
            [member('root/item', tarfile.REGTYPE, data=b'one'), member('root/item')],
            [member('root/item', tarfile.SYMTYPE, link='other'), member('root/item')],
            [member('root/item'), member('root/item', tarfile.SYMTYPE, link='other')],
            [member('root/item', tarfile.SYMTYPE, link='other'), member('root/item', tarfile.SYMTYPE, link='other')]]
        for entries in cases:
            with self.subTest(types=[value.type for value, _ in entries]), tempfile.TemporaryDirectory() as folder:
                root = Path(folder); source = archive(root, entries)
                with self.assertRaises(ValueError): extract_source(source, root/'output')
                self.assertEqual(list((root/'output').rglob('*')), [])

    def test_alias_escape_metadata_and_unselected_member_types_reject(self):
        cases = [
            [member('root/item'), member('root/./item')],
            [member('root/item'), member('root/item', mode=0o755)],
            [member('root/item'), member('root/item', uid=7)],
            [member('root/item'), member('root/item', gid=7)],
            [member('../escape', tarfile.REGTYPE, data=b'escape')],
            [member('/absolute', tarfile.REGTYPE, data=b'escape')],
            [member('root/link', tarfile.SYMTYPE, link='../../escape')],
            [member('root/link', tarfile.SYMTYPE, link='/absolute')],
            [member('root/hard', tarfile.LNKTYPE, link='root/file')],
            [member('root/fifo', tarfile.FIFOTYPE)]]
        for entries in cases:
            with self.subTest(names=[value.name for value, _ in entries]), tempfile.TemporaryDirectory() as folder:
                root = Path(folder); source = archive(root, entries)
                with self.assertRaises(ValueError): extract_source(source, root/'output')
                self.assertEqual(list((root/'output').rglob('*')), [])

    def test_existing_destination_and_dangling_link_keep_original_policy(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder); output = root/'output'; output.mkdir()
            (output/'sentinel').write_bytes(b'preserve')
            source = archive(root, [member('root/file', tarfile.REGTYPE, data=b'data')])
            with self.assertRaises(FileExistsError): extract_source(source, output)
            self.assertEqual((output/'sentinel').read_bytes(), b'preserve')
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder); source = archive(root, [member('root/link', tarfile.SYMTYPE, link='absent')])
            with self.assertRaises(FileNotFoundError): extract_source(source, root/'output')


if __name__ == '__main__': unittest.main()
