# Architecture and source layout

The source tree is grouped by responsibility. Public root module names remain compatible: callers can continue using stkcraft_geocalc::spk, ::frame, ::surface and the existing root re-exports.

| Directory | Problem it handles | Main modules |
| --- | --- | --- |
| src/ephemeris | Trajectories, propagation, center chains and apparent positions/states | spk, spkapo, spk_mda, spk_equinoctial, two_body, sgp4 |
| src/kernels | Binary containers, attitude records and planetary constants | daf, ck, pck, binary_pck, generic_segment |
| src/frames | Reference-frame lookup, composition and orientation models | frame, frame_calculation, orientation, earth_orientation, builtin_frames |
| src/shapes | Surface meshes, spatial lookup and visibility primitives | dsk, dsk_point, dsk_volume, fov_ellipsoid, occult_ellipsoid |
| src/geometry | Individual physical calculations and coordinate outputs | surface, tangent, pointing, direction, location, orbit, separation, coords, state_rep, fov, occult |
| src/search | Geometry Finder relations and event windows | finder, gf_constraints |
| src/mcp | Optional stdio protocol, tool catalog and checked backend adapter | MCP server modules |
| src/bin | Thin command-line and optional MCP entry points | wgc_geometry, wgc_gf, mcp_server |
| src/lib.rs | Public API assembly and compatibility re-exports | No numerical algorithms |
| src/error.rs | Shared explicit calculation and data errors | GeoError |

Each group has a mod.rs describing its purpose. Private algorithm helpers stay private through crate-level compatibility aliases; moving files does not change mathematical operation order.

## Data flow

1. Explicit kernel files enter the container/constant readers in load order.
2. Frames resolves inertial, body, instrument and dynamic references from the attached PCK, CK, SCLK and SPK context.
3. Ephemeris evaluates the selected SPK segment and exact center-chain arithmetic.
4. Geometry applies the requested corrections, surface model and output representation.
5. Search evaluates those same primitives using the native event-search schedule.
6. CLI and MCP serialize the same results; they do not supply a second numerical implementation.

Time 0.1.18 supplies ET/LSK/SCLK contracts. Production does not link CSPICE. The independent native engine, official archive and downloaded mission kernels remain outside the published runtime package.

Tests and generators live in tests/ and tools/. Large fixtures are kept in the Git repository, excluded from crates.io, and regenerated only through independent references. See [validation](VALIDATION.md).

Runtime diagnostics live in tools/diagnose_runtime_math.py and the isolated
tools/reference/runtime_math_probe.rs process. They record native/OS math
provenance; they are not production numerical implementations or acceptance
substitutes.

tools/runtime_profile.py inspects actual PE/linker provenance and validates
the tested executable inventory. tools/verify_release_ci.py validates complete
main CI evidence before packaging those same Windows binaries. The independent
tools/reference/runtime_math_oracle.c process and tools/capture_runtime_math.py
generate new native evidence; neither is linked into production.
