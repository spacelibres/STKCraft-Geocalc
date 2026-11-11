---
name: stkcraft-geocalc
description: Calculate or develop SPICE geometry and event searches with STKCraft-Geocalc, explicit kernels, frame context, and exact reference verification. Use for states, pointing, surfaces, FOV, orbital elements, and Geometry Finder work; use STKCraft-time for time conversion itself.
---

# STKCraft Geocalc

Work from the repository checkout when using its tools or fixtures. Read docs/API.md for signatures, docs/ARCHITECTURE.md for the source layout, and docs/STRICT_REMAINING_GAPS.md for supported boundaries. Older WGC audit reports preserve historical findings and are not the current capability inventory.

## Choose the calculation

- A position request uses the position-only SPKPOS/SPKEZP path, including its native iteration and frame semantics. Prefer SpkKernel::position_corrected_in_frame.
- A six-component state or velocity uses the state/SPKEZR path, such as apparent_state_in with FrameSource::Named. Dropping velocity from a corrected state is not always bit-identical to a position-only call.
- Use pointing_calculation with PointingRequest for POSITION, VELOCITY, arbitrary VECTOR, instrument boundaries, magnitude preservation, and coordinate output. A velocity direction includes the input frame rate when deriving velocity, then rotates that velocity as a vector.
- Use the explicit target-frame surface/tangent entry points when a caller supplies a target frame. The frame defines the ellipsoid geometry; rotating an IAU answer afterward can change the result.
- Event requests use the Geometry Finder APIs and gf_constraints window processing. Search step, convergence, scalar conditions, cyclic coordinates, complement, interval adjustment and filtering are computational inputs.

## Establish the kernel and time context

Use the caller's kernel files and loading order. Later SPK files/segments and text-kernel assignments can override earlier data; a similarly named subset is not a substitute for the selected ephemeris. Identify exact bytes with the provenance manifests when reproducing a reference.

- Load SPKs with SpkKernel::open_all; load PCKs with Pck::open_all; furnish FKs through Frames.
- Attach the same SPK context to Frames with attach_spk(Arc<SpkKernel>) for ephemeris-defined dynamic frames.
- CK frames need their actual CK and SCLK kernels plus an LSK. Attach them through Frames::attach_ck and attach_sclk. Encoded clock ticks and ET are different inputs.
- SPK type 10 requires an explicit LSK attached to SpkKernel with attach_lsk.
- Resolve default body frames through Frames::body_frame. A loaded OBJECT_*_FRAME association can select a frame such as MOON_ME instead of the IAU text-PCK frame.
- Missing coverage, orientation data, unsupported parameters, or invalid models must remain explicit errors. Do not replace them with identity matrices, zero states, another kernel, or an approximate model.

Legacy f64 epochs are NAIF ET seconds past J2000, not UTC seconds or a Unix timestamp. Convert a UTC/SCLK request through STKCraft-time using the supplied LSK/SCLK context, then pass its checked ET to geometry. Keep Time's scientific TDB and NAIF ET contracts distinct. Do not modify the sibling Time project for a geometry task unless that work was requested.

Core distances are normally kilometres and angular calculations use radians; output representations and CLI columns can use degrees. Check the specific API rather than inferring units from field names.

## Use the existing local interfaces

For agent tool calls, use the stkcraft-geocalc-mcp skill and discover the actual tool schema. Direct Rust is appropriate for calculations beyond a tool's exposed fields.

The wgc_geometry and wgc_gf binaries are CSV bridges, not NASA HTTP replicas. Read their mode documentation before forming input. STKCRAFT_SPK selects comma-separated SPKs in load order; optional STKCRAFT_CK, STKCRAFT_SCLK and STKCRAFT_LSK supply clock context. SCLK entries use spacecraft_id|path. The geometry bridge's pointing_full mode accepts a full typed direction; the finder bridge requires the documented 33-field query and emits BEGIN/ROW/END records. Preserve nonempty results and row order; an empty or ERROR record is not a valid numerical answer.

## Verify claims at the right precision

Apply the Windows static-CRT reference profile in docs/NUMERICS.md. Registry
installation and downstream projects do not inherit this repository's Cargo
configuration. Match the floating-point environment as well as kernel bytes;
portable build success does not establish native bit equality.

For development, add an independent native CSPICE regression that fails before the repair. Preserve source arithmetic order when fixing a last-bit difference; do not widen thresholds or round the internal result.

Run the applicable focused tests, then the repository's required checks. The full scientific gate is:

    python tools/verify_strict.py

Missing required resources, ignored/skipped comparisons, NaN, wrong counts, wrong column IDs/types/order, or omitted GM/metadata must not produce a parity pass.

The live comparators distinguish two claims:

- Raw/native equality compares the actual reference values without a tolerance; native bit tests also preserve signed zero.
- Explicit --wgc-output-exact checks an independently established NASA presentation rule. It does not establish equality of the unrounded native doubles.

Record kernel provenance, exact inputs, all output columns, and the comparison mode. Report tested cases and remaining limits rather than extrapolating a passing sample to every mission or parameter combination.

## Repository changes

Keep source comments and documentation in English. Follow the checkout's AGENTS.md and release documentation for version, Cargo.lock, changelog, verification, and commit requirements. Preserve the user's requested scope; installing a skill or making a calculation does not itself authorize a package release or remote publication.
