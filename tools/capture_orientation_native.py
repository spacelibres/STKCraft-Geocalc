"""Capture CSPICE BODEUL angles and host math for cross-platform comparison.

Run with a platform-native CSPICE shared library and the same pck00011.tpc:
    python tools/capture_orientation_native.py <repository-root> <cspice-library>
"""
import ctypes as ct
import json
import math
import sys
from pathlib import Path

root = Path(sys.argv[1]).resolve(strict=True)
library = ct.CDLL(str(Path(sys.argv[2]).resolve(strict=True)))
double, integer, string = ct.c_double, ct.c_int, ct.c_char_p
pointer = ct.POINTER(double)
library.erract_c.argtypes = [string, integer, string]
library.furnsh_c.argtypes = [string]
library.bodeul_.argtypes = [ct.POINTER(integer), pointer, pointer, pointer, pointer, pointer]
library.failed_c.restype = integer
library.erract_c(b'SET', 0, b'RETURN')
library.furnsh_c(str(root / 'target/kernels/pck00011.tpc').encode())
for epoch in (0.0, -1.3365157668936716, 500000000.0):
    body, et = integer(301), double(epoch)
    ra, dec, meridian, longitude = double(), double(), double(), double()
    library.bodeul_(ct.byref(body), ct.byref(et), ct.byref(ra), ct.byref(dec),
                    ct.byref(meridian), ct.byref(longitude))
    if library.failed_c():
        raise RuntimeError('CSPICE BODEUL failed')
    angles = (ra, dec, meridian, longitude)
    print(json.dumps({'et': epoch,
                      'angles': [value.value.hex() for value in angles],
                      'sin': [math.sin(value.value).hex() for value in angles[:3]],
                      'cos': [math.cos(value.value).hex() for value in angles[:3]]}))
