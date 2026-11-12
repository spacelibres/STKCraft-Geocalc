"""Capture native ZZHASH2 surface-name buckets for deterministic ASCII names."""
import ctypes
import spiceypy.utils.libspicehelper as helper
function = helper.libspice.zzhash2_
function.argtypes = [ctypes.c_char_p, ctypes.POINTER(ctypes.c_int), ctypes.c_int]
function.restype = ctypes.c_int
size = ctypes.c_int(2003)
print('name,bucket')
for number in range(2000):
    name = f'NAME{number:04d}'
    value = function(name.encode('ascii').ljust(36, b' '), ctypes.byref(size), 36) - 1
    print(f'{name},{value}')
