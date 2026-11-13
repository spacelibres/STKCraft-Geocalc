"""Capture CSPICE EDLIMB/ZZASRYEL results for the WGC discrepancy.

Uses a caller-supplied CSPICE shared library and the pinned local kernels;
the result is independent native evidence, not a WGC response.
"""
import argparse
import ctypes as c
import hashlib
import json
import math
import platform
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    h = hashlib.sha256()
    with path.open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--library', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    library, output = args.library.resolve(strict=True), args.output.resolve()
    reference = (ROOT / 'docs/reference').resolve()
    if output == reference or reference in output.parents:
        parser.error('write the capture outside committed references')
    kernels = [ROOT/'target/kernels/de442.bsp', ROOT/'target/kernels/pck00011.tpc',
               ROOT/'target/kernels/mars_iau2000_v1.tpc']
    lib = c.CDLL(str(library))
    double, integer, string = c.c_double, c.c_int, c.c_char_p
    vector, matrix = double * 3, (double * 3) * 3

    class Ellipse(c.Structure):
        _fields_ = [('center', vector), ('semiMajor', vector), ('semiMinor', vector)]

    pointer = c.POINTER(double)
    lib.erract_c.argtypes = [string, integer, string]
    lib.furnsh_c.argtypes = [string]
    lib.failed_c.restype = integer
    lib.getmsg_c.argtypes = [string, integer, string]
    lib.tkvrsn_c.argtypes = [string]
    lib.tkvrsn_c.restype = string
    lib.spkpos_c.argtypes = [string, double, string, string, string, pointer, pointer]
    lib.pxform_c.argtypes = [string, string, double, c.POINTER(vector)]
    lib.mxv_c.argtypes = [c.POINTER(vector), pointer, pointer]
    lib.bodvrd_c.argtypes = [string, string, integer, c.POINTER(integer), pointer]
    lib.edlimb_c.argtypes = [double, double, double, pointer, c.POINTER(Ellipse)]
    lib.zzasryel_.argtypes = [string, c.POINTER(Ellipse), pointer, pointer, pointer, pointer, integer]
    lib.erract_c(b'SET', 0, b'RETURN')
    for kernel in kernels:
        lib.furnsh_c(str(kernel).encode())
    radii, count = vector(), integer()
    lib.bodvrd_c(b'EARTH', b'RADII', 3, c.byref(count), radii)
    rows = []
    for et, correction in [(0., b'NONE'), (0., b'LT+S'), (5e8, b'LT+S')]:
        center, light_time = vector(), double()
        lib.spkpos_c(b'EARTH', et, b'J2000', correction, b'SUN', center, c.byref(light_time))
        rotation = matrix()
        lib.pxform_c(b'J2000', b'IAU_EARTH', et if correction == b'NONE' else et-light_time.value, rotation)
        view = vector()
        lib.mxv_c(rotation, center, view)
        view = vector(*[-value for value in view])
        limb, direction, angle, point = Ellipse(), vector(*[-value for value in view]), double(), vector()
        lib.edlimb_c(*radii, view, c.byref(limb))
        lib.zzasryel_(b'MIN', c.byref(limb), view, direction, c.byref(angle), point, 3)
        if lib.failed_c():
            message = c.create_string_buffer(256)
            lib.getmsg_c(b'SHORT', 256, message)
            raise RuntimeError(message.value.decode())
        rows.append({'et':et, 'correction':correction.decode(), 'angle_hex':angle.value.hex(),
                     'full_extent_degrees':2*abs(angle.value)*180/math.pi})
    result = {'platform':platform.platform(), 'cspice':lib.tkvrsn_c(b'TOOLKIT').decode(),
              'library_sha256':digest(library),
              'kernels':{path.name:digest(path) for path in kernels}, 'rows':rows}
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(result, indent=2)+'\n', encoding='utf8')


if __name__ == '__main__':
    main()
