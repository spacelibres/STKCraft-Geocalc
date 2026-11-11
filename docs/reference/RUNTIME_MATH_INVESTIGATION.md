# Runtime math investigation

## Original publication blocker

The first remote acceptance run for commit
`3ea3a1c8f833e3dc9b864c3f735e3f41293a5a84` failed on Windows Server 2022:
[CI run 43023402135](https://github.com/spacelibres/STKCraft-Geocalc/actions/runs/43023402135).
Both Debug and Release reported 271 passed, 13 failed and zero ignored tests.
The local Windows build had passed all 284 tests in each configuration.
No version tag or registry publication was created from that failed commit.
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

## Remote evidence before the repair

Diagnostic commit `deeb0083224524dc2cd4c3e8783440a6dbf1baa6` was checked in
[run 43046031598](https://github.com/spacelibres/STKCraft-Geocalc/actions/runs/43046031598).
The [Windows Server 2022 capture](RUNTIME_MATH_WINDOWS_2022.json) and
[Windows Server 2025 capture](RUNTIME_MATH_WINDOWS_2025.json) are original
artifacts, retained without rewriting their bits or fields.

| Host | Default FMA | Native VSEP mismatching rows / 204 | Native PROP2B mismatching rows / 288 |
| --- | --- | --- | --- |
| Windows Server 2022, build 20348 | 1 | 3 | 8 |
| Windows Server 2025, build 26100 | 1 | 0 | 0 |
| Local capture host, build 26300 | 1 | 0 | 0 |

Every host used the same checksum-pinned native DLL and original input rows.
Both remote CPUs reported FMA3 support. Thus enabling FMA alone does not repair
the older system-UCRT path. The Server 2022 native VSEP/PROP2B differences
reproduce the corresponding Rust failure counts. The older UCRT checksum is
`d06d0a421398fb663c6c49586e09774143460fdb1dce489aa9844846b9398555`.

The ten diagnostic primitive samples match on all hosts and both link modes;
they are insufficient to establish a runtime contract. Installed static UCRT
libraries also have different hashes under the same `10.0.26100.0` version
label. The original diagnostic reports identify installed candidates, not
proof that a particular candidate was linked.

## Static reference builds and independent regression

Version 0.1.8 selects `-C target-feature=+crt-static` for the MSVC target. It
retains the algorithms and all original expectations. The scientific profile
and downstream Cargo configuration are documented in
[NUMERICS](../NUMERICS.md#windows-runtime-profile).

Before that production build change, a separate C process captured 13,909 rows
and 14,914 primitive output cells. The [capture record](RUNTIME_MATH_CAPTURE.json)
identifies diagnostic commit `deeb008`, the official N0067 library, exact SDK
library and compiler hashes, default FMA state, source/generator hashes and
fixture checksum. It also independently rechecks all original 204 VSEP and
288 PROP2B rows with zero mismatch. All inputs and outputs cross the process
boundary as binary doubles; the fixture stores their original hexadecimal bits.

The new primitive test covers signed zero, subnormals, domain endpoints and
thirteen operations. It uses runtime inputs to prevent constant folding and
compares every output cell without a tolerance. The original thirteen remote
failure tests remain mandatory. Static linking is a candidate repair until
the full Windows Server 2022 CI passes for the actual release commit.

CI additionally records the normal Windows Release executables exercised by
the integration/MCP tests. Release packaging verifies those exact bytes and
their static CRT imports, instead of rebuilding on another runner. Portable
platform builds and other CPU/runtime profiles remain separate claims.

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
Version 0.1.8 additionally records explicit library paths and checksums from
the actual MSVC linker response file, using
[/LINKREPROFULLPATHRSP](https://learn.microsoft.com/en-us/cpp/build/reference/link-repro-full-path-rsp).

The Runtime math diagnostic workflow captures the same evidence on Windows
Server 2022 and 2025. Diagnostic job success means that evidence collection
completed; it does not mean that numerical acceptance passed. The normal
strict CI remains mandatory and continues to reject all mismatches. The updated
workflow also runs the complete independent primitive guard under static CRT;
that guard alone is still not complete scientific acceptance.

## Required resolution

The release commit must pass the complete Debug/Release and MCP battery, the
native primitive guard, Python validators, MSRV, rustdoc and package checks.
The main CI source inventory and tested executable checksums must match the
tag. Preserve all original expectations, fields and failed cases. A passing
sample, changed input, excluded output, relaxed equality or unverified change
of operating system cannot close a failure. Complete option-level WebGeocalc
parity remains false while the gap register is open.
