"""Require successful main CI evidence for the exact release commit."""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools'))
import verify_strict
from runtime_profile import BINARY_NAMES, binary_inventory, validate_binaries

REQUIRED_JOBS = {
    'Build, docs and package (ubuntu-latest)',
    'Build, docs and package (macos-15-intel)',
    'Build, docs and package (windows-2022)',
    'MSRV 1.75',
    'Windows pinned CSPICE exactness gate',
}


def validate_run(run: dict, jobs: list[dict], repository: str, sha: str) -> None:
    if not re.fullmatch(r'[0-9a-f]{40}', sha):
        raise ValueError('A full commit SHA is required')
    if (run.get('head_sha') != sha or run.get('head_branch') != 'main'
            or run.get('event') != 'push'
            or run.get('path') != '.github/workflows/ci.yml'
            or run.get('repository', {}).get('full_name') != repository
            or run.get('head_repository', {}).get('full_name') != repository
            or run.get('status') != 'completed'
            or run.get('conclusion') != 'success'):
        raise ValueError('Release requires successful main push CI for this exact commit')
    names = [job.get('name') for job in jobs]
    if len(names) != len(REQUIRED_JOBS) or set(names) != REQUIRED_JOBS:
        raise ValueError('CI job inventory is missing, duplicated or unexpected')
    if any(job.get('status') != 'completed' or job.get('conclusion') != 'success'
           for job in jobs):
        raise ValueError('Every required CI job must succeed; skipped jobs fail acceptance')


def normalized_inventory(inventory: dict) -> dict:
    result = {}
    for key, value in inventory.items():
        normalized = key.replace('\\', '/')
        if normalized in result:
            raise ValueError('Duplicate normalized source key')
        result[normalized] = value
    return result


def validate_evidence(report: dict, logs: dict[str, str], current: dict) -> None:
    if report.get('verification_mode') != 'release' or report.get('release_qualified') is not True or report.get('time_dependency',{}).get('source_kind') != 'registry':
        raise ValueError('Release requires registry-qualified evidence')
    if report.get('local_gate_passed') is not True or report.get('failures') != []:
        raise ValueError('Strict gate report did not pass')
    binary_inventory(report)
    expected = normalized_inventory(current)
    if not expected or any(normalized_inventory(report.get(key, {})) != expected
                           for key in ['verified_source_sha256', 'source_sha256']):
        raise ValueError('CI source inventory differs from the release checkout')
    checks = report.get('checks', [])
    required = {'format', 'python-verifier', 'rust-debug', 'rust-release'}
    if len(checks) != len(required) or {row.get('name') for row in checks} != required:
        raise ValueError('Strict gate command inventory is incomplete')
    for row in checks:
        if row.get('exit_code') != 0:
            raise ValueError('A strict gate command failed')
        name = row['name']
        if name not in logs or not logs[name].strip():
            # cargo fmt intentionally produces an empty success log.
            if name != 'format' or name not in logs:
                raise ValueError('Missing strict gate log: ' + name)
        if name.startswith('rust-'):
            counts, failures = verify_strict.audit_rust_output(logs[name])
            if failures or any(row.get(key) != value for key, value in counts.items()):
                raise ValueError('Raw Rust log does not establish exactness: ' + name)
        elif name == 'python-verifier':
            if not re.search(r'^Ran [1-9][0-9]* tests? in .+$', logs[name], re.M) or not re.search(r'^OK$', logs[name], re.M):
                raise ValueError('Python verification log did not pass')
            if re.search(r'\b(?:skipped|FAILED|ERROR)\b', logs[name]):
                raise ValueError('Python verification log contains failed or skipped tests')


def api(path: str) -> dict:
    return json.loads(subprocess.check_output(['gh', 'api', path], text=True))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--select', action='store_true')
    parser.add_argument('--run-id', type=int)
    parser.add_argument('--evidence', type=Path)
    parser.add_argument('--binaries', type=Path)
    args = parser.parse_args()
    repository = os.environ['GITHUB_REPOSITORY']
    if not re.fullmatch(r'[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+', repository):
        raise ValueError('Invalid repository identifier')
    sha = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
    prefix = 'repos/' + repository + '/actions/'
    if args.select:
        runs = api(prefix + 'workflows/ci.yml/runs?branch=main&event=push&status=success&head_sha=' + sha + '&per_page=100')['workflow_runs']
        if not runs:
            raise ValueError('No successful main CI run exists for this exact commit')
        run = runs[0]
    else:
        if not args.run_id or not args.evidence or not args.binaries:
            parser.error('--run-id, --evidence and --binaries are required when auditing downloaded evidence')
        run = api(prefix + 'runs/' + str(args.run_id))
    jobs = api(prefix + 'runs/' + str(run['id']) + '/jobs?filter=latest&per_page=100')
    if jobs.get('total_count') != len(jobs['jobs']):
        raise ValueError('CI job inventory was truncated')
    validate_run(run, jobs['jobs'], repository, sha)
    if args.select:
        with open(os.environ['GITHUB_OUTPUT'], 'a', encoding='utf-8') as output:
            output.write('run_id=' + str(run['id']) + '\n')
    else:
        reports = list(args.evidence.rglob('report.json'))
        if len(reports) != 1:
            raise ValueError('Exactly one strict gate report is required')
        report = json.loads(reports[0].read_text(encoding='utf-8'))
        logs = {name: (reports[0].parent / (name + '.log')).read_text(encoding='utf-8')
                for name in ['format', 'python-verifier', 'rust-debug', 'rust-release']}
        validate_evidence(report, logs, verify_strict.source_manifest())
        validate_binaries(args.binaries, report)
    print(json.dumps({'verified_main_ci': run['html_url'], 'commit': sha, 'evidence_audited': not args.select}))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
