# Runtime math investigation

## Publication blocker

The first remote acceptance run for commit
`3ea3a1c8f833e3dc9b864c3f735e3f41293a5a84` failed on Windows Server 2022:
[CI run 43023402135](https://github.com/spacelibres/STKCraft-Geocalc/actions/runs/43023402135).
Both Debug and Release reported 271 passed, 13 failed and zero ignored tests.
The local Windows build had passed all 284 tests in each configuration.
No version tag or registry publication was performed after this failure.
The [original failure report](CI_43023402135_FAILURE.json) is preserved
byte-for-byte, including its checked source inventory.

Failures include SGP4, PROP2B, VSEP and two-direction separation, INELPL,
angular extrema, DSK illumination, orbital elements, Euler descriptions,
native separation events and ellipsoid illumination. The original captures
remain unchanged. These are unrounded numerical mismatches, not formatting
differences that can be accepted through a tolerance.

## Evidence collected before repair

The native N0067 DLL and the Rust executable both import transcendental
functions from the Windows UCRT. The reference manifest records the capture
host as Windows build 26300; the failed CI job uses Windows Server 2022.
Pinning the CSPICE DLL does not also pin its operating-system math library.

An isolated local native probe changed only `_set_FMA3_enable`, restored the
original setting and reused the original fixture inputs and expectations.
With FMA enabled, all 204 VSEP and 288 PROP2B rows matched. Disabling FMA
produced three VSEP and twelve PROP2B row mismatches. In particular, the
first failed CI VSEP input produced exactly the CI result under this local
non-FMA native branch. This demonstrates a runtime-sensitive path; it does
not yet establish that every remote failure has the same cause.

Microsoft documents both [CPU/OS-dependent UCRT math results](https://learn.microsoft.com/en-us/cpp/c-runtime-library/floating-point-support)
and [different FMA3 transcendental algorithms](https://learn.microsoft.com/en-us/cpp/c-runtime-library/reference/get-fma3-enable-set-fma3-enable).
No global runtime change has been added to the production library.

## Reproduce the diagnostic

```powershell
python -m pip install --only-binary=:all: -r tools/requirements-reference.txt
python tools/diagnose_runtime_math.py
```

The tool checks the native DLL checksum, records the UCRT checksum and host,
compiles isolated dynamically and statically linked Rust math probes, records
their actual linker arguments and DLL imports, hashes installed static UCRT
libraries, captures the default FMA setting in each process, and records
original/native fixture bit patterns with FMA
unchanged, disabled and enabled. It writes only under `target/runtime-math`.
The optional `--output` argument selects a different report destination.

The Runtime math diagnostic workflow captures the same evidence on Windows
Server 2022 and 2025. Diagnostic job success means that evidence collection
completed; it does not mean that numerical acceptance passed. The normal
strict CI remains mandatory and continues to reject all mismatches.

## Required resolution

Establish whether remote Rust and native CSPICE differ in the same runtime,
whether the stored native outputs themselves drift with the runtime, and
which primitives cause the first divergent operation. Add independent native
regressions before a production repair. Preserve all original expectations,
all fields and all failed cases. Do not force a passing comparison by
changing inputs, excluding outputs, relaxing equality or moving the gate to
a different environment without an explicit numerical contract.
