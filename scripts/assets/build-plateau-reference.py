#!/usr/bin/env python3
"""Rebuild the bounded offline reference pack from hash-pinned local sources."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--sources', type=Path, default=Path('.tools/plateau-pilot'))
    ap.add_argument('--directory', type=Path, default=Path('validation/plateau-pilot'))
    args = ap.parse_args()
    module_path = Path(__file__).with_name('extract-plateau-crop.py')
    spec = importlib.util.spec_from_file_location('plateau_crop', module_path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    provenance = json.loads((args.directory / 'provenance.json').read_text())
    samples = []
    for entry in provenance['samples']:
        source = args.sources / f'thecity-plateau-{entry["sourceIndex"]}.gml.gz'
        if hashlib.sha256(source.read_bytes()).hexdigest() != entry['compressedSha256']:
            raise ValueError('Source hash mismatch: ' + str(source))
        crop = entry['crop']
        result = module.extract(source, crop['centerLon'], crop['centerLat'], crop['sizeM'])
        result['sampleId'] = entry['id']
        result['catalogFeatureCountMatches'] = sum(result['topLevelMemberTypeCounts'].values()) == entry['catalogBuildingFile']['features']
        result['acceptedForCalibration'] = False
        if not result['catalogFeatureCountMatches'] or result['coverageIncomplete']:
            raise ValueError('Unreconciled source geometry: ' + entry['id'])
        if not 0 < result['clippedUnionAreaM2'] <= crop['sizeM'] ** 2:
            raise ValueError('Invalid clipped union area')
        samples.append(result)
    output = {'status': 'geometry/catalog reconciliation passed; district calibration and synthetic benchmark pending', 'samples': samples}
    (args.directory / 'morphology.json').write_text(json.dumps(output, separators=(',', ':')) + '\n')
    subprocess.run([sys.executable, str(Path(__file__).with_name('render-plateau-reference.py')), '--directory', str(args.directory)], check=True)


if __name__ == '__main__':
    main()
