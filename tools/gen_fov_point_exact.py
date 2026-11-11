"""Boundary and adjacent-float FOVRAY reference predicates, independent of Rust."""
from pathlib import Path
import csv, math, re
import numpy as np
import spiceypy as s

root = Path(__file__).resolve().parents[1]
with (root / 'tests/golden/fov_point_exact.csv').open('w', newline='') as file:
    out = csv.writer(file)
    out.writerow(['kernel', 'instrument', 'x', 'y', 'z', 'visible'])
    for name in ['fov.tf', 'fov_ellipsoid.tf']:
        s.kclear()
        path = root / 'tests/golden' / name
        s.furnsh(str(path))
        instruments = sorted(set(map(int, re.findall(r'INS(-?\d+)_FOV_SHAPE', path.read_text()))))
        # All shapes and orientations, including exact edges and the nearest
        # representable values on both sides. No geometric epsilon is used.
        for instrument in instruments:
            shape, frame, bore, count, bounds = s.getfov(instrument, 100)
            vectors = [bore, -bore]
            for point in bounds:
                vectors.append(point)
                for coordinate in range(3):
                    for toward in [-math.inf, math.inf]:
                        moved = point.copy()
                        moved[coordinate] = np.nextafter(moved[coordinate], toward)
                        vectors.append(moved)
                vectors.extend([point * 1e-100, point * 1e100])
            for vector in vectors:
                visible = s.fovray(str(instrument), vector, frame, 'NONE', 'EARTH', 0.0)
                out.writerow([name, instrument, *[repr(float(x)) for x in vector], int(visible)])
s.kclear()
