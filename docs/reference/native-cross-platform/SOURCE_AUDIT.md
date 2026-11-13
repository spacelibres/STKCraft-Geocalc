# Independent source comparison

This comparison investigates the captured geometry outputs; neither GMAT nor
STK is asserted to be the WebGeocalc server implementation.

- Official [NAIF CSPICE N0067 Windows package](https://naif.jpl.nasa.gov/pub/naif/misc/toolkit_N0067/C/PC_Windows_VisualC_64bit/packages/cspice.zip),
  SHA-256 `98d60b814b412fa55294aeaaeb7dab46d849cc87a8b709ffe835d08de17625dc`:
  `bodeul.c` computes orientation angles and calls `eul2m_`; `eul2m.c` calls
  rotation routines; `rotmat.c` and `rotate.c` invoke the host `sin`/`cos`.
  The compiler may combine these into a `sincos` call. The controlled native
  experiment in the adjacent README intercepts both possibilities. The official
  Windows static `cspice.lib`, independently linked by the two reference
  harnesses, reproduces all three captured Windows matrices and all four
  captured emission-angle and tangent-altitude bit patterns.
- The official [NAIF CSPICE N0067 Linux package](https://naif.jpl.nasa.gov/pub/naif/toolkit/C/PC_Linux_GCC_64bit/packages/cspice.tar.Z),
  SHA-256 `60a95b51a6472f1afe7e40d77ebdee43c12bb5b8823676ccc74692ddfede06ce`,
  includes the prebuilt `cspice.a`. The same surface harness was linked to
  that archive for Linux and run with the recorded glibc. All four binary64
  outputs and all three rotation matrices match the independent Linux CSPICE
  shared-library capture.
- [NASA GMAT `SpiceOrbitKernelReader.cpp`](https://github.com/nasa/GMAT/blob/b8aec552a8f34a9212063adc3f249bd2315397a3/src/base/spice/SpiceOrbitKernelReader.cpp)
  delegates SPICE state and frame operations to the platform toolkit;
  geometric states use `spkgeo_c`, corrected states use `spkezr_c`, and
  frame transformations use `sxform_c` or `pxform_c`. This confirms those
  GMAT paths do not independently define WebGeocalc's limb-size algorithm
  or eliminate the host CSPICE floating-point environment.
- The locally supplied STK v10 source registers separate triaxial-ellipsoid
  intersection and tangent functions. `AgAsTriEllps.c` dispatches them to
  `AgUtTriEllps.c`, whose ray/ellipsoid intersection solves a quadratic and
  has its own direction and near-zero checks. These are useful as independent
  geometry comparisons, but they do not establish CSPICE operation order or
  WebGeocalc bit equality. The local SPK adapter calls `furnsh_c` and
  `spklef_c`. No local proprietary source is reproduced in this repository.

Local STK file SHA-256 values (paths relative to its `Source` directory):

```text
946defe7e2773522b21f91eb63e3626935f9b8dd5831082294341f931020d52b  Util/AgUtil/AgUtTriEllps.c
c38898f993185f0e855ac774ed49bca179433ef681135aa97a90d23b678c09c5  Astro/AgAstro/AgAsTriEllps.c
2b9513ea4ba6e29ae8b8e9c99bf71ca4529246171cf7fd398e02b6ce7d5394bc  AgAsSpkEphem/AgAsSpkEphem/AgSpice.c
```

The Linux library's captured host-math intervention reproduces all seven
Windows baseline records. It does not prove the WGC server's kernel bytes,
compiler, math library version, or numerical behavior for other inputs.

The separately captured deterministic 20,001-input cosine corpus compares
Rust `libm` 0.2.16 with Debian glibc 2.41. The captured troublesome input
matches, but 632 other outputs differ bitwise. Consequently, replacing the
platform cosine with that `libm` implementation solely because it fixes one
case would not establish glibc or WebGeocalc equivalence. The input and both
output files are retained and hashed in `metadata.json`. The generator used
Python `random.Random(7143)`, sampled `uniform(-pi, pi)` 20,000 times, then
appended the captured input. The reference binaries are generated from
`tools/reference_libm_probe.rs` and `tools/reference_glibc_probe.py`.
