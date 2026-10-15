"""Download pinned regression fixtures, or hydrate a supplied offline archive."""
import argparse
import importlib.util
import json
from pathlib import Path
import tempfile
import urllib.request

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / 'scripts/test-fixtures-manifest.json'
URL = 'https://github.com/jeremyung/thecity/releases/download/runtime-review-fixtures-2026-10-14/test-fixtures.tar.gz'
spec = importlib.util.spec_from_file_location('hydrate', ROOT / 'scripts/hydrate-review-evidence.py')
hydrator = importlib.util.module_from_spec(spec)
spec.loader.exec_module(hydrator)

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--archive', type=Path, help='Use a local archive instead of downloading')
    args = parser.parse_args()
    try:
        with tempfile.TemporaryDirectory(prefix='city-fixtures-') as temp:
            archive = args.archive
            if archive is None:
                archive = Path(temp) / 'test-fixtures.tar.gz'
                # Hash validation uses the committed manifest, not remote metadata.
                with urllib.request.urlopen(URL, timeout=60) as response, archive.open('wb') as target:
                    total = 0
                    while chunk := response.read(1024 * 1024):
                        total += len(chunk)
                        if total > 8 * 1024 * 1024:
                            raise ValueError('Fixture download exceeds the 8 MiB bound')
                        target.write(chunk)
            print(json.dumps(hydrator.hydrate(archive, MANIFEST, ROOT, apply=True, fixtures_only=True), indent=2))
    except Exception as error:
        parser.exit(1, f'Fixture setup failed: {error}\nUse --archive with the matching offline bundle if needed.\n')
