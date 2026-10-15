"""Verify a review evidence bundle; optionally hydrate missing files.

Default is read-only. Existing different files are never overwritten. Fetch the
archive and manifest from the same reviewed release before invoking this tool.
"""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import shutil
import tarfile
import tempfile


def digest(stream):
    result = hashlib.sha256()
    for chunk in iter(lambda: stream.read(1024 * 1024), b''):
        result.update(chunk)
    return result.hexdigest()


def destination(root, name):
    path = PurePosixPath(name)
    if (path.is_absolute() or '..' in path.parts or str(path) != name
            or not (name.startswith(('validation/', 'concepts/', 'tests/fixtures/'))
                    or name == 'engine/plan.svg')):
        raise ValueError(f'Invalid evidence path: {name}')
    target = root.joinpath(*path.parts)
    # Reject links even if they currently resolve inside root: hydration must
    # never write through a different tree than the one the manifest names.
    for parent in [target, *target.parents]:
        if parent == root:
            break
        if parent.is_symlink():
            raise ValueError(f'Symlink destination: {name}')
    if not target.resolve().is_relative_to(root):
        raise ValueError(f'Destination escapes root: {name}')
    return target


def hydrate(archive, manifest_path, root, apply=False, fixtures_only=False):
    root = root.resolve()
    if not root.is_dir():
        raise ValueError('Destination must be an existing directory')
    manifest = json.loads(manifest_path.read_text())
    with archive.open('rb') as stream:
        if digest(stream) != manifest['archiveSHA256']:
            raise ValueError('Archive SHA-256 mismatch')
    entries = {}
    for entry in manifest['files']:
        name = entry['path']
        destination(root, name)
        if name in entries:
            raise ValueError(f'Duplicate manifest entry: {name}')
        entries[name] = entry
    selected = {name for name in entries if not fixtures_only or name.startswith('tests/fixtures/') or entries[name].get('testFixture') is True}
    missing = []
    # Verify the entire archive and all destination conflicts before writing.
    # Staging avoids a second unverified read of the archive during hydration.
    with tempfile.TemporaryDirectory(prefix='city-evidence-') as staging:
        stage = Path(staging)
        seen = set()
        with tarfile.open(archive, 'r:gz') as bundle:
            for member in bundle:
                name = member.name
                if name not in entries or name in seen or not member.isfile():
                    raise ValueError(f'Unexpected, duplicate or non-file archive member: {name}')
                seen.add(name)
                entry = entries[name]
                if member.size != entry['bytes']:
                    raise ValueError(f'Size mismatch: {name}')
                payload = bundle.extractfile(member)
                assert payload is not None
                staged = stage / name
                if apply and name in selected:
                    staged.parent.mkdir(parents=True, exist_ok=True)
                    with staged.open('wb') as out:
                        shutil.copyfileobj(payload, out)
                    payload.close()
                    with staged.open('rb') as stream:
                        actual = digest(stream)
                else:
                    with payload:
                        actual = digest(payload)
                if actual != entry['sha256']:
                    raise ValueError(f'File SHA-256 mismatch: {name}')
                if name not in selected:
                    continue
                target = destination(root, name)
                if target.exists():
                    if not target.is_file():
                        raise ValueError(f'Destination is not a file: {name}')
                    with target.open('rb') as stream:
                        if digest(stream) != actual:
                            raise ValueError(f'Existing file differs; retain or relocate it explicitly: {name}')
                else:
                    missing.append(name)
        if seen != entries.keys():
            raise ValueError('Archive is missing manifest entries')
        if apply:
            for name in missing:
                target = destination(root, name)
                target.parent.mkdir(parents=True, exist_ok=True)
                # Exclusive creation refuses files introduced after preflight.
                with target.open('xb') as out, (stage / name).open('rb') as source:
                    shutil.copyfileobj(source, out)
    return {'verifiedFiles': len(entries), 'selectedFiles': len(selected),
            'alreadyPresent': len(selected) - len(missing),
            'missingFiles': len(missing), 'writtenFiles': len(missing) if apply else 0}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--archive', type=Path, required=True)
    parser.add_argument('--manifest', type=Path, required=True)
    parser.add_argument('--destination', type=Path, required=True)
    parser.add_argument('--fixtures-only', action='store_true')
    parser.add_argument('--apply', action='store_true', help='Write missing files after verification')
    args = parser.parse_args()
    try:
        print(json.dumps(hydrate(args.archive, args.manifest, args.destination,
                                 args.apply, args.fixtures_only), indent=2))
    except (ValueError, KeyError, OSError, tarfile.TarError) as error:
        parser.exit(1, f'Evidence hydration failed: {error}\n')
