"""Reproducible local conformance gate. Missing kernels/skips never pass.

This gate proves only the exercised, pinned CSPICE cases. The separate live
WebGeocalc report and remaining feature gaps must also be reviewed.
"""
from __future__ import annotations
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import struct
import sys
import tarfile
import tomllib
from datetime import datetime, timezone

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0,str(Path(__file__).resolve().parent))
from bootstrap_reference import pinned_kernel_records,resolve_reference_binary
from runtime_profile import tested_binary_inventory
DEST = ROOT / 'target/strict-verification'
REQUIRED_TESTS = [
    'native_historical_spk10_corrected_state_position_cells_exact',
    'native_latitudinal_surface_coordinates_exact',
    'native_surface_intercept_all_output_cells_exact',
    'native_dsk_point_occultation_windows_exact',
    'explicit_occultation_models_preserve_native_ellipsoid_windows',
    'native_kernel_pool_assignments_and_partial_errors_exact',
    'native_kernel_date_values_and_rejection_presence',
    'native_fixed_and_dynamic_frame_error_matrix_exact',
    'native_text_kernel_frame_appends_exact', 'native_text_kernel_pck_appends_exact',
    'native_text_kernel_fov_appends_exact', 'native_text_kernel_quoted_numeric_axes_rejected',
    'native_pck_replacements_and_failed_updates_exact',
    'native_windows_math_primitives_exact',
    'corrected_states_equal_cspice', 'segment_priority_and_center_chains_match_cspice_exactly',
    'all_getfov_boundaries_equal_cspice', 'fov_boundary_predicates_equal_cspice',
    'fov_gf_windows_exact', 'orientation_native_source_values_exact',
    'zero_valued_pck_models_are_valid', 'surface_precision_cspice',
    'tangent_exact_cspice', 'exact_cspice_position_only_inertial_frames',
    'exact_zzocced_states', 'exact_cspice_window_operations',
    'frame_two_epoch_state_transforms_exact', 'native_pckmat_type2_exact',
    'fov_hull_axis_exact_cspice', 'selectable_ellipsoid_frames_match_cspice',
    'named_mixed_segment_frames_equal_cspice', 'spk_modified_difference_lines_equal_cspice',
    'spk5_two_body_blending_equal_cspice', 'spk14_generic_chebyshev_equal_cspice',
    'spk15_j2_precession_equal_cspice', 'spk17_equinoctial_states_equal_cspice',
    'spk18_packet_interpolation_equal_cspice', 'spk19_interval_selection_equal_cspice',
    'spk20_velocity_integrals_equal_cspice', 'native_prop2b_states_equal_cspice',
    'native_earth_orientation_transforms_exact', 'native_eul2xf_all_axis_sequences_exact',
    'native_ck_encoded_tick_pointing_exact', 'native_ck_frames_use_exact_spacecraft_clock_conversion',
    'native_sclk_ticks_for_ck_frames', 'native_valid_scalar_sclk_kernel_is_accepted',
    'visibility_width_limit_errors_equal_cspice',
    'exact_cspice_gfuds_with_native_uddc', 'exact_native_uddf_and_uddc_primitives',
    'exact_cspice_native_separation_events', 'exact_cspice_native_range_schedules',
    'exact_cspice_native_singleton_confinements', 'exact_cspice_gfsubc_named_target_frames',
    'exact_cspice_gfilum_named_target_frames', 'exact_cspice_gfsntc_explicit_target_frames',
    'exact_cspice_gf_intercept_four_direction_specs',
    'native_pck_3_20_records_and_frames_exact', 'native_sgp4_teme_states_exact',
    'spk10_sgp4_j2000_states_equal_cspice', 'spk10_corrected_state_position_pairs_equal_cspice',
    'real_cassini_sclk_and_pointing_are_exact',
    'native_dynamic_euler_frames_exact', 'native_dynamic_euler_public_sxform_exact',
    'native_dynamic_constant_two_vector_frames_exact',
    'native_ck4_chebyshev_pointing_exact', 'native_ck5_packet_interpolation_exact',
    'native_ck6_piecewise_interpolation_exact', 'native_ck456_big_endian_pointing_exact',
    'native_ck456_sclk_frame_transforms_exact',
    'native_spkezp_dynamic_input_output_frames_exact',
    'native_pointing_dynamic_direction_rotations_exact',
    'native_ray_dynamic_direction_rotations_exact',
    'native_tangent_dynamic_target_frame_exact',
    'spk17_accepts_native_exact_eccentricity_boundary',
    'spk17_rejects_native_kepleq_eccentricity_boundary_error',
    'native_dynamic_product_frames_exact', 'native_switch_frames_exact',
    'native_dynamic_of_date_frames_exact', 'native_dynamic_ephemeris_two_vector_frames_exact',
    'native_frame_body_alias_append_and_precedence',
    'native_body_frame_uses_kernel_preferred_body_name',
    'exact_native_dsk_structure', 'exact_native_dsk_rays', 'exact_native_das_layouts',
    'rejects_corrupt_das_without_panics', 'exact_native_unprioritized_dsk_union',
    'exact_native_dsk_surface_geometry', 'exact_native_dynamic_surface_models',
    'exact_native_dsk_illumination', 'native_point_membership_exact', 'native_plate_nearest_exact',
    'native_volume_bounds_exact', 'native_volume_rays_exact', 'native_volume_boundary_rays_exact',
    'native_cone_quadratic_exact', 'native_cone_segment_exact',
    'native_dynamic_pointing_cli_context_exact',
    'native_dynamic_coordinate_finder_cli_context_exact',
    'mcp_stdio_initialize_discovery_and_notification_silence',
    'mcp_stdio_rejects_bad_requests_and_unknown_tools',
    'mcp_state_and_gf_preserve_native_backend_bits',
    'mcp_modern_discovery_and_finite_validation',
    'mcp_csv_adapter_validation_and_exact_output',
    'mcp_protocol_recovery_and_backend_error_isolation',
    'mcp_ray_tools_require_an_explicit_direction',
    'mcp_preserves_custom_frame_names_containing_locus_words',
    'mcp_frame_tool_default_orientation_is_supported',
    'mcp_location_rejects_invalid_values_and_recovers',
    'mcp_location_preserves_native_moving_point_bits',
    'mcp_location_preserves_native_frame_matrix_bits',
    'location_cli_fixed_point_never_uses_velocity',
    'location_cli_rejects_unknown_endpoint_kind_without_calculating',
    'location_cli_rejects_unknown_frame_type_without_calculating',
    'location_cli_rejects_unknown_frame_locus_without_calculating',
    'location_cli_rejects_invalid_orientation_flags',
    'moving_location_cli_matches_native_correction_matrix',
    'moving_location_cli_matches_native_rotating_frames',
    'fixed_location_cli_matches_native_frame_matrix',
    'moving_location_cli_matches_native_derived_fields',
    'moving_observer_cli_matches_spkcvo_bits',
    'subpoint_cli_rejects_unknown_kind_without_calculating',
    'native_dsk_illumination_missing_data_errors',
    'sub_point_methods_match_cspice',
    'native_surface_hash_two_thousand_names',
    'native_method_lexer_token_ranges', 'native_method_parser_capture',
    'native_surface_numeric_fallback_corpus', 'native_surface_numeric_extreme_exponents',
    'native_surface_long_name_and_whitespace_corpus',
    'native_named_surfaces_select_latitudinal_geometry',
    'native_latsrf_method_and_frame_validation', 'native_latsrf_without_loaded_dsk',
    'native_sincpt_method_and_error_precedence',
    'native_sincpt_inertial_target_and_missing_radii', 'native_sincpt_rejects_invalid_radii',

]

def digest(path: Path) -> str:
    result = hashlib.sha256()
    with path.open('rb') as file:
        for chunk in iter(lambda: file.read(1024*1024), b''):
            result.update(chunk)
    return result.hexdigest()


def audit_reference_files(manifest: dict, root: Path) -> list[str]:
    failures=[]
    if manifest.get('toolkit') != 'CSPICE N0067':
        failures.append('Unexpected reference toolkit')
    source=root/'target/reference/n0067/cspice/src/cspice'
    hashes=manifest.get('reference_source_sha256',{})
    if not hashes:
        failures.append('Independent C source provenance is empty')
    if len(list(source.glob('*.c'))) != manifest.get('C_source_count'):
        failures.append('Independent C source inventory differs')
    expected_files=[(root/'target/reference/cspice-n0067.zip',manifest.get('archive_sha256'))]
    expected_files += [(source/name,expected) for name,expected in hashes.items()]
    binary=manifest.get('reference_binary',{})
    if binary.get('toolkit_version') != 'CSPICE_N0067' or not binary.get('path'):
        failures.append('Independent native reference is missing or has a different toolkit version')
    else:
        try:expected_files.append((resolve_reference_binary(manifest),binary.get('sha256')))
        except RuntimeError as error:failures.append(str(error))
    for path,expected in expected_files:
        if not expected or not path.is_file() or digest(path)!=expected:
            failures.append(f'Missing or changed independent reference: {path}')
    return failures

def time_dependency(allow_local: bool = False) -> dict:
    result=subprocess.run(['cargo','metadata','--format-version=1','--locked','--all-features'],cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,encoding='utf-8',check=True)
    metadata=json.loads(result.stdout)
    owners=[p for p in metadata['packages'] if Path(p['manifest_path']).resolve()==(ROOT/'Cargo.toml').resolve()]
    if len(owners)!=1: raise RuntimeError('Cannot identify GeoCalc package in Cargo metadata')
    owner=owners[0]
    declarations=[d for d in owner['dependencies'] if d['name']=='stkcraft-time' and d.get('kind') is None]
    nodes={n['id']:n for n in metadata['resolve']['nodes']}
    aliases={(d.get('rename') or d['name']).replace('-','_') for d in declarations}
    edges=[d for d in nodes[owner['id']]['deps'] if d['name'] in aliases and any(k.get('kind') is None for k in d['dep_kinds'])]
    if len(declarations)!=1 or len(edges)!=1: raise RuntimeError('Time direct dependency is missing or ambiguous')
    matches=[p for p in metadata['packages'] if p['id']==edges[0]['pkg'] and p['name']=='stkcraft-time']
    if len(matches)!=1: raise RuntimeError('Time dependency edge does not resolve uniquely')
    package=matches[0]; directory=Path(package['manifest_path']).parent
    lock_path=Path(metadata['workspace_root'])/'Cargo.lock'
    lock=tomllib.loads(lock_path.read_text(encoding='utf-8'))
    entries=[p for p in lock['package'] if p['name']=='stkcraft-time' and p['version']==package['version'] and p.get('source')==package.get('source')]
    if len(entries)!=1: raise RuntimeError('Time dependency does not match a unique Cargo.lock entry')
    source=package.get('source'); checksum=None; archive=None
    if source and source.startswith('registry+'):
        checksum=entries[0].get('checksum')
        if not re.fullmatch('[a-f0-9]{64}',checksum or ''): raise RuntimeError('Registry package checksum is missing')
        archive=directory.parent.parent.parent/'cache'/directory.parent.name/('stkcraft-time-'+package['version']+'.crate')
        if not archive.is_file() or digest(archive)!=checksum: raise RuntimeError('Registry archive missing or checksum differs')
        with tarfile.open(archive,'r:gz') as package_files:
            expected_sources=set()
            prefix='stkcraft-time-'+package['version']+'/'
            for member in package_files.getmembers():
                if member.isfile() and member.name.startswith(prefix):
                    name=member.name[len(prefix):]
                    expected_sources.add(name)
                    local=directory/name
                    if not local.is_file() or local.read_bytes()!=package_files.extractfile(member).read():
                        raise RuntimeError('Registry Time source differs from archive: '+name)
            actual_sources={p.relative_to(directory).as_posix() for p in directory.rglob('*') if p.is_file()}
            actual_sources.discard('.cargo-ok')
            if actual_sources != expected_sources:
                raise RuntimeError('Registry Time source inventory differs from archive')
        kind='registry'
    elif source is None and allow_local:
        for target in package.get('targets',[]):
            target_path=Path(target['src_path']).resolve()
            try: relative=target_path.relative_to(directory.resolve())
            except ValueError: raise RuntimeError('Local Time target is outside its package directory')
            if relative.parts and relative.parts[0] in ('.git','target'):
                raise RuntimeError('Local Time target is inside an excluded generated directory')
        kind='local-path'
    else: raise RuntimeError('Release verification requires a registry Time dependency')
    fingerprint={'version':package['version'],'source':source,'features':sorted(nodes[package['id']]['features']),'requirement':declarations[0]['req']}
    return {'version':package['version'],'source':source,'source_kind':kind,'package_id':package['id'],
            'version_requirement':declarations[0]['req'],'manifest_path':package['manifest_path'],
            'archive_path':str(archive) if archive else None,'package_checksum':checksum,'lock_path':str(lock_path),
            'metadata_sha256':hashlib.sha256(json.dumps(fingerprint,sort_keys=True).encode()).hexdigest()}



def source_manifest(allow_local: bool = False) -> dict[str,str]:
    files = [p for directory in ['src','tests','tools','.github','.agents'] for p in sorted((ROOT/directory).rglob('*'))
             if p.is_file() and p.suffix in ['.rs','.py','.c','.csv','.tpc','.tf','.ti','.bsp','.bpc','.bc','.tsc','.tls','.bds','.dsk','.txt','.json','.md','.yml','.yaml']]
    files += [ROOT/'Cargo.toml']
    if (ROOT/'Cargo.lock').is_file(): files.append(ROOT/'Cargo.lock')
    files += [p for p in sorted((ROOT/'docs').rglob('*')) if p.is_file()]
    files += [p for p in [ROOT/'.cargo/config.toml'] if p.is_file()]
    files += [p for p in [ROOT/name for name in ['README.md','LICENSE','ATTRIBUTION.md','CHANGELOG.md','AGENTS.md','CITATION.cff','.gitattributes','.gitignore','requirements-reference.txt']] if p.is_file()]
    files += [p for p in sorted((ROOT/'.githooks').rglob('*')) if p.is_file()]
    # Python import caches are generated execution products, including when a
    # reviewed evidence script is imported. Preserve every original capture.
    files = [p for p in files if '__pycache__' not in p.relative_to(ROOT).parts
             and p.suffix not in ['.pyc', '.pyo']]
    hashes={os.path.relpath(p, ROOT):digest(p) for p in files}
    dependency=time_dependency(allow_local=True) if allow_local else time_dependency();directory=Path(dependency['manifest_path']).parent
    prefix='dependency:stkcraft-time@'+dependency['version']+'/'
    if dependency.get('source_kind') == 'local-path':
        paths=[]
        for current, subdirs, names in os.walk(directory):
            if Path(current)==directory: subdirs[:]=[name for name in subdirs if name not in ('.git','target')]
            paths.extend(Path(current)/name for name in names if (Path(current)/name).is_file())
        paths.sort()
    else:
        paths=[directory/'Cargo.toml', *([directory/'build.rs'] if (directory/'build.rs').is_file() else []), *[p for p in sorted((directory/'src').rglob('*')) if p.is_file()]]
    for path in paths:
        hashes[prefix+path.relative_to(directory).as_posix()]=digest(path)
    if dependency.get('package_checksum'): hashes[prefix+'package-checksum']=dependency['package_checksum']
    if dependency.get('metadata_sha256'): hashes[prefix+'metadata-sha256']=dependency['metadata_sha256']
    if dependency.get('lock_path'):
        workspace=Path(dependency['lock_path']).parent
        if workspace.resolve()!=ROOT.resolve():
            for path in [workspace/'Cargo.toml',workspace/'Cargo.lock',workspace/'.cargo/config.toml',workspace/'.cargo/config']:
                if path.is_file(): hashes['dependency:workspace/'+path.relative_to(workspace).as_posix()]=digest(path)
    return hashes


def audit_rust_output(output: str) -> tuple[dict,list[str]]:
    summaries = re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored',output)
    record = {key:sum(int(x[index]) for x in summaries)
              for index,key in enumerate(['passed','failed','ignored'])}
    failures = []
    if not summaries or record['passed'] < 100:
        failures.append('Rust test battery is empty or unexpectedly small')
    if record['failed']:
        failures.append('Rust tests failed')
    if record['ignored'] or re.search(r'\bskipping\b', output, re.I):
        failures.append('Skipped/ignored tests cannot establish conformance')
    # libtest writes the name and status separately. Concurrent native
    # diagnostics may appear between them; never cross the next test prefix.
    prefixes=list(re.finditer(r'(?<![A-Za-z_])test ([\w:]+) \.\.\.',output))
    results={}
    for index,prefix in enumerate(prefixes):
        end=prefixes[index+1].start() if index+1<len(prefixes) else len(output)
        tail=output[prefix.end():end].split('test result:',1)[0]
        statuses=re.findall(r'^[ \t]*(ok|FAILED|ignored)(?:, [^\r\n]*)?[ \t]*\r?$',tail,re.M)
        name=prefix[1].split('::')[-1]
        results.setdefault(name,[]).append(statuses==['ok'])
        if 'FAILED' in statuses:
            failures.append('Rust test output contains a failed status: '+prefix[1])
    for test in REQUIRED_TESTS:
        if not results.get(test) or not all(results[test]):
            failures.append(f'Required exact test did not pass: {test}')
    return record, failures

def main() -> int:
    parser=argparse.ArgumentParser()
    parser.add_argument('--integration',action='store_true',help='Verify a local Time source without qualifying a release')
    args=parser.parse_args()
    DEST.mkdir(parents=True, exist_ok=True)
    report = {'utc': datetime.now(timezone.utc).isoformat(), 'complete_webgeocalc_parity': False,
              'acceptance': 'exact reference comparisons; no skipped or ignored tests',
              'checks': [], 'failures': [], 'verification_mode': 'integration' if args.integration else 'release'}
    manifest = json.loads((ROOT/'docs/reference/CSPICE_N0067.json').read_text())
    report['failures'].extend(audit_reference_files(manifest,ROOT))
    try:
        for row in pinned_kernel_records(manifest,ROOT):
            path=ROOT/'target/kernels'/row['name']
            if not path.is_file() or digest(path)!=row['sha256']:
                report['failures'].append('Missing or different pinned kernel: '+row['name'])
    except (OSError,ValueError) as error:report['failures'].append('Pinned kernel inventory failed: '+str(error))
    env = os.environ.copy()
    # Windows cannot overwrite a test executable while another worker runs it.
    env['CARGO_TARGET_DIR'] = str(DEST/'cargo')
    for name in ['STKCRAFT_SPK','STKCRAFT_PCK','STKCRAFT_FK']:
        env.pop(name, None)
    commands = [
        ('format', ['cargo','fmt','--all','--','--check']),
        ('python-verifier', [sys.executable,'-m','unittest','discover','-s','tools/tests','-v']),
        ('rust-debug', ['cargo','test','--all-features','--all-targets','--no-fail-fast','--','--nocapture']),
        ('rust-release', ['cargo','test','--release','--all-features','--all-targets','--no-fail-fast','--','--nocapture']),
    ]
    try:
        report['time_dependency']=time_dependency(allow_local=True) if args.integration else time_dependency()
        before=source_manifest(allow_local=True) if args.integration else source_manifest()
    except (OSError,ValueError,RuntimeError,subprocess.CalledProcessError) as error:
        before={};report['failures'].append('Source/dependency provenance failed: '+str(error))
    if not report['failures']:
        for name, command in commands:
            print(f'Checking {name}', flush=True)
            result = subprocess.run(command, cwd=ROOT, env=env, stdout=subprocess.PIPE,
                                    stderr=subprocess.STDOUT, text=True, encoding='utf-8', errors='replace')
            (DEST/f'{name}.log').write_text(result.stdout, encoding='utf-8')
            record = {'name':name, 'command':command, 'exit_code':result.returncode}
            report['checks'].append(record)
            if result.returncode:
                report['failures'].append(f'{name} exited {result.returncode}; see {name}.log')
            if name.startswith('rust-'):
                counts, failures = audit_rust_output(result.stdout)
                record.update(counts)
                report['failures'].extend(f'{name}: {failure}' for failure in failures)
    if not report['failures']:
        try:
            report['release_binary_sha256'] = tested_binary_inventory(DEST/'cargo/release')
        except (OSError, ValueError, AssertionError, struct.error) as error:
            report['failures'].append('Tested Windows Release binary provenance failed: '+str(error))
    report['verified_source_sha256'] = before
    try:report['source_sha256']=source_manifest(allow_local=True) if args.integration else source_manifest()
    except (OSError,ValueError,RuntimeError,subprocess.CalledProcessError) as error:
        report['source_sha256']={};report['failures'].append('Final source/dependency provenance failed: '+str(error))
    if before != report['source_sha256']:
        report['failures'].append('Sources or path dependency changed during verification; rerun at a stable checkpoint')
    report['local_gate_passed'] = not report['failures']
    report['release_qualified'] = report['local_gate_passed'] and not args.integration and report.get('time_dependency',{}).get('source_kind') == 'registry'
    (DEST/'report.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
    print('LOCAL EXACT GATE: '+('PASS' if report['local_gate_passed'] else 'FAIL'),flush=True)
    for failure in report['failures']:print(failure)
    print('Full WebGeocalc parity remains a separate, incomplete requirement.')
    return 0 if report['local_gate_passed'] else 1

if __name__ == '__main__':
    raise SystemExit(main())
