"""Capture private N0067 ZZPRSMET output before implementing its parser."""
import ctypes as ct
import random
import itertools
import spiceypy as spice
from spiceypy.utils.libspicehelper import libspice

integer = ct.c_int
func = libspice.zzprsmet_
func.argtypes = [ct.POINTER(integer), ct.c_char_p, ct.POINTER(integer),
                 ct.c_char_p, ct.c_char_p, ct.POINTER(integer), ct.POINTER(integer),
                 ct.POINTER(integer), ct.c_char_p, ct.c_char_p,
                 integer, integer, integer, integer, integer]
func.restype = integer

methods = [
    'DSK/UNPRIORITIZED', 'UNPRIORITIZED/DSK', 'DSK', 'DSK/PRIORITIZED',
    'DSK/UNPRIORITIZED/SURFACES=100',
    'DSK / UNPRIORITIZED / SURFACES = "Alpha Surface", 100',
    'DSK/UNPRIORITIZED/SURFACES="UNKNOWN"',
    'DSK/UNPRIORITIZED/SURFACES=', 'DSK/UNPRIORITIZED/SURFACES=100,',
    'DSK/UNPRIORITIZED/SURFACES=100,,100',
    'DSK/UNPRIORITIZED/SURFACES=""', 'DSK/UNPRIORITIZED/SURFACES="A',
    'DSK/UNPRIORITIZED/SURFACES=ALPHA SURFACE',
    'DSK/UNPRIORITIZED/SURFACES="Alpha Surface"/SURFACES=100',
    'DSK/UNPRIORITIZED/UNPRIORITIZED', 'DSK/DSK/UNPRIORITIZED',
    'INTERCEPT/DSK/UNPRIORITIZED', 'NADIR/DSK/UNPRIORITIZED',
    'ELLIPSOID', 'INTERCEPT/ELLIPSOID', 'NEAR POINT/ELLIPSOID',
    'ELLIPSOID/DSK/UNPRIORITIZED', 'DSK//UNPRIORITIZED',
    'DSK/UNPRIORITIZED/SURFACES=100/INTERCEPT',
    'ELLIPSOID/NEAR', 'NEAR/ELLIPSOID', 'ELLIPSOID/NEAR/NADIR',
    'ELLIPSOID/NEAR/TANGENT', 'ELLIPSOID/NEAR/NEAR',
    'DSK/UNPRIORITIZED/SURFACES=' + 'A' * 971 + 'é',
]
rng = random.Random(20261113)
clauses = ['DSK', 'ELLIPSOID', 'UNPRIORITIZED', 'INTERCEPT', 'NADIR',
           'TANGENT', 'GUIDED', 'UMBRAL', 'PENUMBRAL', 'NEAR POINT',
           'SURFACES=100', 'SURFACES="Alpha Surface"', 'SURFACES=UNKNOWN',
           'SURFACES=100,100', 'SURFACES=100,', 'SURFACES=', '', 'BOGUS']
for _ in range(2000):
    methods.append('/'.join(rng.choice(clauses) for _ in range(rng.randrange(1, 6))))
for first, second in itertools.product(clauses, repeat=2):
    for shape in ['DSK', 'ELLIPSOID']:
        methods.append('/'.join([shape, first, second]))
        methods.append('/'.join([first, shape, second]))
surface_tokens = ['100', '1e2', '"Alpha Surface"', '"UNKNOWN"', '""', '"A""B"',
                  '/', ',', '=', ':', '100 100', 'UNKNOWN', ' ', '"A']
for _ in range(4000):
    members = ','.join(rng.choice(surface_tokens) for _ in range(rng.randrange(1, 5)))
    suffix = rng.choice(['', '/NADIR', '/INTERCEPT', '/SURFACES=100', '/UNPRIORITIZED', '/'])
    methods.append('DSK/UNPRIORITIZED/SURFACES=' + members + suffix)
for _ in range(4000):
    parts = [rng.choice(clauses) for _ in range(rng.randrange(1, 6))]
    method = rng.choice(['DSK', 'ELLIPSOID'])
    for part in parts:
        method += rng.choice(['/', ' ', '  ', ':', ',']) + part
    methods.append(method)
spice.kclear()
spice.pcpool('NAIF_SURFACE_NAME', ['Alpha Surface', 'A' * 36])
spice.pipool('NAIF_SURFACE_CODE', [100, 7])
spice.pipool('NAIF_SURFACE_BODY', [301, 301])
cases = [(method, 100) for method in methods]
for maximum in [0, 1, 2]:
    for suffix in ['100', '100,100', '100,UNKNOWN', 'UNKNOWN,100']:
        cases.append(('DSK/UNPRIORITIZED/SURFACES=' + suffix, maximum))
print('maximum|method|result')
for method, capacity in cases:
    body, maximum = integer(301), integer(capacity)
    priority, count = integer(), integer()
    codes = (integer * 100)()
    shape, subtype, point, terminator = (ct.create_string_buffer(32) for _ in range(4))
    data = method.encode('utf-8')
    func(ct.byref(body), data, ct.byref(maximum), shape, subtype,
         ct.byref(priority), ct.byref(count), codes, point, terminator,
         len(data), 32, 32, 32, 32)
    if spice.failed():
        result = 'E|' + spice.getmsg('SHORT', 1841)
        spice.reset()
    else:
        result = 'N|' + '|'.join([shape.value.decode().strip(), subtype.value.decode().strip(),
                                  str(priority.value), ','.join(map(str, codes[:count.value])),
                                  point.value.decode().strip(), terminator.value.decode().strip()])
    print(str(capacity) + '|' + method + '|' + result)
spice.kclear()
