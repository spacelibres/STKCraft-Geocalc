# WebGeocalc feature-by-feature verification

This ledger follows the 24 geometry features named in the
[NASA WebGeocalc API](https://wgc2.jpl.nasa.gov:8443/webgeocalc/documents/api-info.html).
It records this verification pass; existing native-reference tests and earlier
evidence remain documented in [STRICT_PARITY_STATUS.md](STRICT_PARITY_STATUS.md).
An observed pass applies only to the specified cases. WGC decimal output and
unrounded CSPICE binary64 comparisons are different evidence.

| Feature | Current pass | Evidence and outstanding work |
| --- | --- | --- |
| State Vector | In progress | 135 object rows, 2,184 coordinate cells and 29 point-location rows match captured WGC formatted values; native exact tests pass on their fixtures. Untested input combinations remain. |
| Angular Separation | In progress | 42 captured two-target WGC rows match after the coincident-point fix. Four retained two-direction WGC cases match displayed values and native binary64 angle references. The point/sphere and Finder distinction has dedicated evidence. Other direction and option combinations remain. |
| Angular Size | In progress | The former center-ray ZZASRYEL extrema differed from WGC's displayed limb sizes. Ellipse-axis angular widths match the 54-row sample, a 61-epoch Earth/Sun grid, 45 rows spanning three targets and five corrections, 45 Mars/Phobos/Deimos rows, and 18 IAU_EARTH/ITRF93 rows. The explicit-frame API and MCP expose the requested frame. Native center-ray extrema remain available separately. WGC's unrounded internal results are unavailable; other cases remain untested. |
| Frame Transformation | In progress | 87 orientation/angular-velocity cases match captured WGC formatted output. Other frames, aberration corrections and options remain. |
| Illumination Angles | In progress | Nine ellipsoid surface-point rows match captured WGC formatted output, including angles, time, altitude and local solar time. Other methods, coordinates and corrections remain. |
| Phase Angle | In progress | Eighteen original WGC rows match for three illuminator/target/observer combinations and two corrections. A further 27 Moon/Earth/Mars rows with LT, CN and CN+S match WGC display values and independent CSPICE angle bits with the recorded SPK load order. Other combinations remain. |
| Pointing Direction | In progress | Seven captured WGC cases match for position, velocity and explicit vectors with selected frames and representations. Other sources and options remain. |
| Sub-solar Point | In progress | Twelve WGC rows match for two ellipsoid methods and two lunar frames. Other options remain. |
| Sub-observer Point | In progress | Two near-zero emission angles differ under the default Windows math profile. The explicitly enabled glibc R3 profile and recorded Linux profile match these NASA display values and the official Linux CSPICE raw results. Local solar time matches 60 retained time-grid values and a further 90 WGC values across two bodies, three frames, two methods and five corrections. Other cases remain to be tested. |
| Surface Intercept Point | In progress | Six captured WGC ellipsoid and body-directed ray rows match. Other ray sources, shapes and misses remain. |
| Tangent Point | In progress | Two near-zero surface-point altitude cells differ under the default Windows math profile. The explicitly enabled glibc R3 profile and recorded Linux profile match these NASA display values and the official Linux CSPICE raw results. Other targets, correction loci and runtime operations remain to be tested. |
| Orbital Elements | In progress | Six WGC rows match all captured output columns after WGC serialization for two body pairs. Other GM modes, frames and corrections remain. |
| Longitude of the Sun | In progress | Six WGC rows match for the Earth and Moon with no correction. Other target frames and corrections remain. |
| Position Finder | In progress | One captured window matches both WGC output and native endpoint bits; other options remain. |
| Angular Separation Finder | In progress | Two windows match WGC output and native endpoint bits; the recorded coincident-body request rejects with SPICE(BODIESNOTDISTINCT). Other options remain. |
| Distance Finder | In progress | One standard window and five postprocessed windows match WGC output and native bits. Other options remain. |
| Sub-point Finder | In progress | One captured window matches WGC output and native bits. Other options remain. |
| Occultation Finder | In progress | Twelve correction/type combinations match WGC and native windows, including three empty results; other shapes and frames remain. |
| Surface Intercept Finder | In progress | One captured window matches WGC output and native bits. Other directions and conditions remain. |
| Target in Field of View Finder | In progress | Cassini examples with NONE, LT and CN+S match WGC output and native bits. Other targets and instruments remain. |
| Ray in Field of View Finder | In progress | Cassini examples with NONE, S and XS match WGC output and native window bits. The captured LT and CN+S requests return SPICE(INVALIDOPTION), matching the native error symbol. Other rays and instruments remain. |
| Range Rate Finder | In progress | Two captured windows match WGC output and native bits. Other conditions remain. |
| Phase Angle Finder | In progress | One captured window matches WGC output and native bits. Other illuminators and conditions remain. |
| Illumination Angles Finder | In progress | One captured window matches WGC output and native bits. Other points and conditions remain. |

The replayable live responses are in [wgc-calculators](reference/wgc-calculators/).
Run `python tools/replay_wgc_calculators.py --spk <de442.bsp> --pck
<pck00011.tpc> --frame-pck <ordered-comma-separated-PCKs> --report
<output.json>`. It intentionally exits with failure
while the Sub-observer Point and Tangent Point differences remain. The server-managed kernel
set's bytes have not been independently retrieved; local kernel hashes are
recorded in the report and are not asserted to be identical to the server's.
The specific Windows/Linux CSPICE comparison is recorded in
[native cross-platform evidence](reference/native-cross-platform/README.md).
On the recorded Linux glibc profile, all 19 retained calculator replay
groups pass their displayed-value checks, including the four cells that fail
on the recorded Windows profile. This is not certification for all kernel
sets or input options; see the retained
[Linux replay report](reference/native-cross-platform/wgc-linux-calculators.json).
The same profile also passes the 12 retained finder cases against WGC's
displayed values and the recorded Windows CSPICE endpoint bits; see the
[Linux finder report](reference/native-cross-platform/wgc-linux-finders.json).
On Windows, the explicitly enabled `wgc-glibc-rotation` profile also passes the
19 retained calculator groups, the 18 Earth-frame and 45 Mars-family limb
rows, and the 12 finder cases. The default Windows profile still fails the
four near-zero cells. The selectable profile changes R3 trigonometry only and
is not a claim of full glibc or WebGeocalc equivalence. See
[the Windows profile report](reference/native-cross-platform/wgc-windows-glibc-calculators.json)
and [numerical contract](NUMERICS.md#linux-webgeocalc-observations).

The 11 finder-family captures and their native endpoint references can be
replayed with `python tools/replay_wgc_finders.py --binary <wgc_gf executable>
--report <output.json>`; see [finder evidence](reference/wgc-finders/README.md).

An additional matrix covers nine relation/correction combinations for each
of Distance, Phase Angle, Range Rate, Angular Separation, Sub-point,
Illumination Angles, Position and Surface Intercept Finder. All 72 cases pass
both displayed WGC values and native endpoint/duration bits on the recorded
Windows native and glibc-R3 profiles and the recorded Linux glibc profile. The captured pre-fix results retain three failing
LT surface-intercept longitude cases. The derivative callbacks now take their
coordinate from the intercept state, as in NAIF's ZZGFCOSD/ZZGFCOCD; the
independent SINCPT position remains the value callback. Nine native windows
are also covered by a Rust regression. Reproduce the complete matrix with
`python tools/replay_wgc_finder_matrix.py --binary <wgc_gf> --glibc-rotation
--report <output.json>`. This remains representative coverage; the server's
kernel bytes and unrounded outputs are not independently known.
The Linux results and binary/runtime hashes are in
[the matrix report](reference/native-cross-platform/wgc-linux-finder-matrix.json).
The stellar-only ray cases and the rejected light-time corrections can be
replayed using `python tools/replay_wgc_ray_stellar.py --binary <wgc_gf>
--report <output.json>` with the recorded Cassini kernels.
The retained empty-correction observations distinguish the C wrapper's
`SPICE(EMPTYSTRING)` from WGC's earlier Java argument-validation error; a
nonempty blank correction produces `SPICE(INVALIDOPTION)` in the C wrapper.
Use the same command with `--target-fov` for the three target-in-FOV cases.
The 12 occultation cases use `python tools/replay_wgc_finder_matrix.py
--occultation --binary <wgc_gf> --report <output.json>`; the three expected
empty windows are checked as explicit empty results, not counted as endpoint
comparisons.

The Mars and triaxial satellite angular-size cases require an additional
Mars satellite SPK and can be replayed with `python tools/replay_wgc_mars.py
--spk <de442,mar099s> --pck <pck00011,mars_iau2000_v1> --report
<output.json>`. The local kernel hashes are recorded in the report; server
kernel identity has not been independently verified.

The expanded Phase Angle matrix is replayed with
`python tools/replay_wgc_phase_matrix.py --binary <wgc_geometry> --spk
<mar099s,de442> --native-capture
docs/reference/wgc-calculators/phase-angle-expanded-native.json --report
<output.json>`. Add `--glibc-rotation` when using that profile. The native
capture comparison checks all 27 unrounded angle bit patterns and verifies the
kernel hashes and order. Replaying the reversed order with
`phase-angle-expanded-native-reversed.json` also checks all 27 native bits;
the command returns failure for its three WGC display differences. The order
is significant: loading
DE442 after mar099s makes its overlapping segments higher priority. Reversing
the order produces three WGC display differences in these 27 samples even
though both orders independently match CSPICE PHASEQ angle bits. Native
captures, hashes and both orders are recorded in the calculator evidence.
The observed agreement does not establish the server's kernel bytes or load
order, and it does not establish equality of WGC's unrounded angles.

The four independently captured two-direction cases can be rechecked with
`python tools/replay_wgc_directions.py --report <output.json>`. This runs the
native-bit regression before comparing WGC's displayed values.

The two Earth target-frame samples use an Earth binary PCK and can be replayed
with `python tools/replay_wgc_earth_frames.py --spk <de442> --pck
<pck00011,earth-binary-pck> --report <output.json>`. Matching formatted
values does not establish server kernel identity or unrounded equality.
