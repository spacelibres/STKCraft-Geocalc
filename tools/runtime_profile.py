"""Inspect the Windows binaries used by exactness tests and release packaging."""
from __future__ import annotations

import hashlib
from pathlib import Path
import re
import struct

BINARY_NAMES = {'wgc_geometry.exe', 'wgc_gf.exe', 'stkcraft-geocalc-mcp.exe'}


def pe_imports(path: Path) -> list[str]:
    """Read direct import DLL names from a PE64 executable."""
    data = path.read_bytes()
    assert data[:2] == b'MZ'
    header = struct.unpack_from('<I', data, 60)[0]
    assert data[header:header + 4] == b'PE\0\0'
    assert struct.unpack_from('<H', data, header + 4)[0] == 0x8664
    count = struct.unpack_from('<H', data, header + 6)[0]
    optional_size = struct.unpack_from('<H', data, header + 20)[0]
    optional = header + 24
    assert struct.unpack_from('<H', data, optional)[0] == 0x20b
    sections = []
    for index in range(count):
        entry = optional + optional_size + index * 40
        virtual_size, address, raw_size, raw_offset = struct.unpack_from('<IIII', data, entry + 8)
        sections.append((address, max(virtual_size, raw_size), raw_offset))

    def offset(rva: int) -> int:
        for address, length, raw_offset in sections:
            if address <= rva < address + length:
                return raw_offset + rva - address
        raise ValueError('Import RVA is outside PE sections')

    import_rva = struct.unpack_from('<I', data, optional + 120)[0]
    result = []
    if import_rva:
        entry = offset(import_rva)
        while any(data[entry:entry + 20]):
            name = offset(struct.unpack_from('<I', data, entry + 12)[0])
            result.append(data[name:data.index(b'\0', name)].decode('ascii'))
            entry += 20
    return result


def binary_inventory(report: dict) -> dict[str, str]:
    inventory = report.get('release_binary_sha256', {})
    if set(inventory) != BINARY_NAMES or any(not isinstance(value, str) or not re.fullmatch(r'[0-9a-f]{64}', value)
                                            for value in inventory.values()):
        raise ValueError('The tested Release binary inventory is missing or malformed')
    return inventory


def validate_binaries(directory: Path, report: dict) -> None:
    for name, expected in binary_inventory(report).items():
        path = directory / name
        if path.is_symlink() or not path.is_file() or path.stat().st_size < 10000:
            raise ValueError('A tested Windows Release executable is missing: ' + name)
        if hashlib.sha256(path.read_bytes()).hexdigest() != expected:
            raise ValueError('Downloaded executable differs from the tested binary: ' + name)
        imports = pe_imports(path)
        if any('ucrt' in item.lower() or item.lower().startswith('api-ms-win-crt-')
               or 'vcruntime' in item.lower() or item.lower() == 'msvcrt.dll' for item in imports):
            raise ValueError('Tested executable imports dynamic CRT: ' + name)


def tested_binary_inventory(directory: Path) -> dict[str, str]:
    """Hash the normal executables exercised through CARGO_BIN_EXE by the gate."""
    inventory = {name: hashlib.sha256((directory / name).read_bytes()).hexdigest()
                 for name in sorted(BINARY_NAMES)}
    validate_binaries(directory, {'release_binary_sha256': inventory})
    return inventory


def linked_libraries(response_path: Path) -> list[dict[str, str]]:
    """Record explicit library inputs reported by MSVC's actual linker."""
    data = response_path.read_bytes()
    response = data.decode('utf-16' if data.startswith(b'\xff\xfe') else 'utf-8-sig')
    libraries = {}
    for line in response.splitlines():
        value = line.strip().strip('"')
        if value.lower().startswith('/defaultlib:'):
            value = value[len('/defaultlib:'):]
        path = Path(value)
        if path.suffix.lower() == '.lib':
            if not path.is_absolute() or not path.is_file():
                raise ValueError('The actual linker library path is unavailable: ' + value)
            libraries[str(path)] = hashlib.sha256(path.read_bytes()).hexdigest()
    return [{'path': path, 'sha256': digest} for path, digest in sorted(libraries.items())]
