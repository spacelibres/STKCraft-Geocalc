"""Capture native ZZLEXMET token byte ranges and error symbols."""
import ctypes as ct
import random
import spiceypy as spice
from spiceypy.utils.libspicehelper import libspice

integer = ct.c_int
func = libspice.zzlexmet_
func.argtypes = [ct.c_char_p, ct.POINTER(integer), ct.POINTER(integer),
                 ct.POINTER(integer), ct.POINTER(integer), integer]
func.restype = integer
samples = [
    '', ' ', 'DSK/UNPRIORITIZED', ' DSK / SURFACES = 100, 200 ',
    'DSK/UNPRIORITIZED/SURFACES="Alpha / Beta",100',
    'DSK/UNPRIORITIZED/SURFACES=""', 'DSK/UNPRIORITIZED/SURFACES="A',
    'DSK/UNPRIORITIZED/SURFACES="A""B"', 'A::B', '/ , = :',
    'A"B C"D', 'A\tB', 'A  B', 'A' * 1000,
    '/'.join(['A'] * 51), '/'.join(['A'] * 50),
]
rng = random.Random(20261112)
for _ in range(2000):
    samples.append(''.join(rng.choice('AB12 /,=:"\t') for _ in range(rng.randrange(80))))
print('method|result')
for sample in samples:
    raw = sample.encode('ascii')
    maximum, count = integer(100), integer()
    starts, ends = (integer * 100)(), (integer * 100)()
    func(raw, ct.byref(maximum), ct.byref(count), starts, ends, len(raw))
    if spice.failed():
        result = 'E|' + spice.getmsg('SHORT', 1841)
        spice.reset()
    else:
        result = 'N|' + ','.join(f'{starts[i]}:{ends[i]}' for i in range(count.value))
    print(sample.replace('\t', '\\t') + '|' + result)
