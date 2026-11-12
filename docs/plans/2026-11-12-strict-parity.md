# Remaining strict conformance work

The baseline is published version 0.1.8. Complete WebGeocalc conformance is
false. This plan does not convert representative coverage into exhaustive
acceptance. Every repaired calculation requires an independent native
regression before the production change.

1. Reproduce text-kernel assignment semantics with the pinned N0067 C library:
   ordered replacement and append, case-sensitive names, quoted character
   types, malformed assignments, and calculations after repeated loads.
   Apply the verified parser to frame, planetary-constant and instrument
   contexts. Preserve all native output fields and every recorded error.
2. Record native kernel-date inputs, binary64 outputs and errors. Verify the
   available STKCraft-time API before integration. Keep Time-owned parser or
   conversion defects as evidence handoffs under the existing ownership rule.
3. Complete DSK surface selection and integration into point-location,
   direction, illumination and subpoint facades; then cover native
   DSK-versus-point occultation behavior. Reuse the already verified direct
   DSK algorithms without an ellipsoid fallback.
4. Enumerate the WebGeocalc option, representation, flag and error cases for
   each calculation family. Identify kernel bytes and load order. Distinguish
   original native doubles from NASA transport serialization; preserve both
   passing and failing original captures.
5. Check every implemented option through the Rust, CSV and applicable typed
   MCP paths. Keep unsupported or unverified cases visible in the gap register.
6. For each verified update, synchronize the patch version and English release
   records, run the complete required checks, and commit. Publication requires
   successful main CI, an annotated tag, and separate distribution, registry and
   rustdoc verification. Published tags and package bytes are immutable.

An unavailable external reference, an unimplemented option or a failed
comparison remains an open item. No guessed result, relaxed comparison or
changed input can establish parity.
