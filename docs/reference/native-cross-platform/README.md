# Subpoint and tangent native comparison

The two captures use the same hashed local kernels and separate Windows and
Linux CSPICE N0067 libraries. They record intermediate binary64 values for
the Sub-observer Point and Tangent Point examples that differ from WGC output.
Linux reproduces the observed WGC near-zero emission angles and tangent
surface altitudes, while Windows reproduces the existing Windows-profile
implementation. At the recorded epoch, three J2000-to-IAU_MOON rotation
matrix entries differ by one ULP. This observation does not prove that all
differences have the same cause or that the server's kernels are byte-identical.
The captured BODEUL angles have identical bits on both platforms. At the
nonzero epoch, Python's platform cosine of the common prime-meridian angle
differs by one ULP. `python tools/compare_orientation_math.py` reconstructs
both captured rotation matrices exactly by changing only this cosine input.
This proves the origin of those three matrix differences for this case, but
does not by itself isolate every later operation in the surface calculations.
An additional intervention loads the Linux CSPICE library with
`tools/reference_cos_override.rs`, overriding only the exact captured cosine
input in both `cos` and `sincos` to the Windows value. All seven intermediate
records, including both emission angles and both tangent altitudes, then
equal the Windows capture. `python tools/check_cos_override_evidence.py`
checks the captured results. This is a controlled experiment for these
specific inputs, not a production numerical method or a general proof for all
epochs and geometries.
Build the intervention as a Linux `cdylib` with a Linux Rust target and a
cross linker, then pass it with `ld-linux --preload` when running
`tools/capture_subpoint_native.py` with the recorded Linux CSPICE library and
kernel files. This experiment used Rust 1.99.1, `-C panic=abort`, and
`rust-lld` in GNU linker mode. The LD_PRELOAD library intentionally changes
one value and must not be loaded in production calculations.
For this input, `python tools/check_orientation_cosine.py` independently
evaluates cosine using a 100-digit decimal Taylor series: rounding that result
to binary64 selects the captured Linux value. This single observation is not
a claim that either platform's cosine is correctly rounded for every input.
Both native ET2LST calls using the untransformed requested-frame longitude
return `20:11:19`; WGC returns `20:11:20`. ET2LST uses the body-associated
frame, so a point expressed in IAU_MOON must first be transformed to MOON_ME.
With that transformation at the surface epoch, native ET2LST matches all 60
retained time-grid values. A further 90 WGC values exercise two bodies, three
frames, two surface methods and five corrections. The CLI applies the same
frame transformation and surface epoch for sub-observer solar time.

With the same kernel files, reproduce the raw native values with
`python tools/capture_subpoint_native.py <repository-root> <CSPICE-library>`.
The script emits JSON lines and must run with a library for its host platform.
`metadata.json` records kernel, library and capture hashes.

The Linux calculator replay uses `tools/replay_wgc_wsl.py` to route the
ordinary, structured location, and structured frame-representation requests
to the Linux binary. The replay aborts if the comparator attempts any other
untracked subprocess, and records backend-mode counts and runtime hashes.
An earlier exploratory Linux replay inadvertently ran the structured
location and frame cases through Windows; its mixed-platform result was
discarded. The retained Linux report uses the corrected routing.
