"""Capture independent CSPICE PHASEQ bits for the expanded WGC matrix.

Requires SpiceyPy and the same SPK order used by the Rust replay. The output
records the loaded CSPICE library hash and does not query the WGC server.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct

import spiceypy as spice
from replay_wgc_phase_matrix import CASES, CORRECTIONS, TIMES


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--spk', required=True, help='comma-separated kernels in load order')
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    kernels = [Path(part).resolve(strict=True) for part in args.spk.split(',')]
    for kernel in kernels:
        spice.furnsh(str(kernel))
    rows = []
    for _, target, _, illuminator, _, observer in CASES:
        for correction in CORRECTIONS:
            for et in TIMES:
                angle = spice.phaseq(et, str(target), str(illuminator), str(observer), correction)
                rows.append({'query': [target, illuminator, observer, et, correction],
                             'native_rad_hex': angle.hex(),
                             'native_rad_bits': f"{struct.unpack('>Q', struct.pack('>d', angle))[0]:016x}"})
    library = Path(spice.utils.libspicehelper.libspice._name).resolve(strict=True)
    report = {'cspice': spice.tkvrsn('TOOLKIT'), 'spiceypy': spice.__version__,
              'native_library_sha256': digest(library),
              'kernels_in_load_order': [{'name': p.name, 'sha256': digest(p)} for p in kernels],
              'rows': rows}
    output = args.report.resolve()
    reference = (Path(__file__).resolve().parents[1] / 'docs/reference').resolve()
    if output == reference or reference in output.parents:
        parser.error('write the report outside the reference directory')
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2)+'\n', encoding='utf8')
    print(f'{len(rows)} native PHASEQ rows captured')


if __name__ == '__main__':
    main()
