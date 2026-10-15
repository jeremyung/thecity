"""Focused integrity and overwrite regressions for evidence hydration."""
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('hydrate', Path(__file__).with_name('hydrate-review-evidence.py'))
hydrator = importlib.util.module_from_spec(spec)
spec.loader.exec_module(hydrator)


class EvidenceTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.folder = Path(self.temp.name)
        self.root = self.folder / 'checkout'
        self.root.mkdir()

    def bundle(self, files):
        archive = self.folder / 'evidence.tar.gz'
        entries = []
        with tarfile.open(archive, 'w:gz') as tar:
            for name, data in files.items():
                info = tarfile.TarInfo(name)
                info.size = len(data)
                tar.addfile(info, io.BytesIO(data))
                entries.append({'path': name, 'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()})
        manifest = self.folder / 'manifest.json'
        manifest.write_text(json.dumps({'files': entries, 'archiveSHA256': hashlib.sha256(archive.read_bytes()).hexdigest()}))
        return archive, manifest

    def test_verify_then_hydrate_and_repeat_without_rewriting(self):
        args = self.bundle({'tests/fixtures/example.json': b'{}', 'validation/frame.png': b'picture'})
        self.assertEqual(hydrator.hydrate(*args, self.root)['missingFiles'], 2)
        self.assertEqual(list(self.root.iterdir()), [])
        result = hydrator.hydrate(*args, self.root, apply=True, fixtures_only=True)
        self.assertEqual(result['writtenFiles'], 1)
        self.assertFalse((self.root / 'validation').exists())
        target = self.root / 'tests/fixtures/example.json'
        stamp = target.stat().st_mtime_ns
        self.assertEqual(hydrator.hydrate(*args, self.root, apply=True, fixtures_only=True)['writtenFiles'], 0)
        self.assertEqual(target.stat().st_mtime_ns, stamp)

    def test_fixture_role_includes_validation_dependencies_only(self):
        archive, manifest = self.bundle({'validation/witness.json': b'{}', 'validation/frame.png': b'picture'})
        data = json.loads(manifest.read_text())
        data['files'][0]['testFixture'] = True
        manifest.write_text(json.dumps(data))
        result = hydrator.hydrate(archive, manifest, self.root, apply=True, fixtures_only=True)
        self.assertEqual(result['writtenFiles'], 1)
        self.assertTrue((self.root / 'validation/witness.json').exists())
        self.assertFalse((self.root / 'validation/frame.png').exists())

    def test_conflict_prevents_all_writes(self):
        args = self.bundle({'validation/new.txt': b'new', 'validation/existing.txt': b'original'})
        target = self.root / 'validation/existing.txt'
        target.parent.mkdir()
        target.write_bytes(b'local work')
        with self.assertRaisesRegex(ValueError, 'Existing file differs'):
            hydrator.hydrate(*args, self.root, apply=True)
        self.assertEqual(target.read_bytes(), b'local work')
        self.assertFalse((self.root / 'validation/new.txt').exists())

    def test_corruption_prevents_writes(self):
        archive, manifest = self.bundle({'validation/new.txt': b'new'})
        archive.write_bytes(archive.read_bytes() + b'corruption')
        with self.assertRaisesRegex(ValueError, 'Archive SHA-256 mismatch'):
            hydrator.hydrate(archive, manifest, self.root, apply=True)
        self.assertEqual(list(self.root.iterdir()), [])

    def test_file_hash_checked_after_archive_hash(self):
        archive, manifest = self.bundle({'validation/new.txt': b'new'})
        data = json.loads(manifest.read_text())
        data['files'][0]['sha256'] = '0' * 64
        manifest.write_text(json.dumps(data))
        with self.assertRaisesRegex(ValueError, 'File SHA-256 mismatch'):
            hydrator.hydrate(archive, manifest, self.root, apply=True)
        self.assertEqual(list(self.root.iterdir()), [])

    def test_traversal_and_git_paths_refused(self):
        for name in ['../escape', 'validation/../../escape', '.git/config']:
            with self.subTest(name=name):
                args = self.bundle({name: b'bad'})
                with self.assertRaisesRegex(ValueError, 'Invalid evidence path'):
                    hydrator.hydrate(*args, self.root, apply=True)
        self.assertEqual(list(self.root.iterdir()), [])

    def test_symlink_destination_refused(self):
        args = self.bundle({'validation/new.txt': b'new'})
        (self.root / 'validation').symlink_to(self.folder, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, 'Symlink destination'):
            hydrator.hydrate(*args, self.root, apply=True)
        self.assertFalse((self.folder / 'new.txt').exists())


if __name__ == '__main__':
    unittest.main()
