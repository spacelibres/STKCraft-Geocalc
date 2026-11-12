"""Check version/changelog/package invariants, optionally before a commit."""
from __future__ import annotations
import argparse
import json
from pathlib import Path
import re
import subprocess
import sys
import tomllib

ROOT=Path(__file__).resolve().parents[1]

def version_tuple(value: str) -> tuple[int,int,int]:
    if not re.fullmatch(r'(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)',value):
        raise ValueError('Version must be MAJOR.MINOR.PATCH without leading zeroes')
    return tuple(map(int,value.split('.')))

def validate_increment(previous: str | None, current: str, breaking_review: bool=False) -> list[str]:
    try:
        now=version_tuple(current)
        before=version_tuple(previous) if previous is not None else None
    except ValueError as error:
        return [str(error)]
    reviewed_minor=(before is not None and before[0]==0 and
                    now==(0,before[1]+1,0) and breaking_review)
    if before is not None and now!=(before[0],before[1],before[2]+1) and not reviewed_minor:
        return ['Each normal update must increment exactly one patch version; a minor/major release needs an explicitly reviewed policy change']
    return []

def breaking_release_review(changelog_section: str, release_notes: str) -> bool:
    """Require an explicit breaking-change marker and a migration section."""
    migration=re.search(r'^## Migration[ \t]*\r?\n(.*?)(?=^##[ \t]|\Z)',release_notes,re.M|re.S)
    return bool(re.search(r'^- \*\*Breaking:\*\*\s+\S',changelog_section,re.M) and
                migration and migration[1].strip())

def git(*args: str, required: bool=True) -> str:
    result=subprocess.run(['git',*args],cwd=ROOT,text=True,encoding='utf-8',capture_output=True)
    if required and result.returncode:
        raise RuntimeError(result.stderr.strip())
    return result.stdout if not result.returncode else ''

def check(staged: bool=False, require_gate: bool=False) -> list[str]:
    failures=[]
    package=tomllib.loads((ROOT/'Cargo.toml').read_text())['package']
    version=package['version']
    failures.extend(validate_increment(None,version))
    lock=tomllib.loads((ROOT/'Cargo.lock').read_text())
    own=[p for p in lock['package'] if p['name']==package['name']]
    if len(own)!=1 or own[0]['version']!=version:
        failures.append('Cargo.lock package version differs')
    changelog=(ROOT/'CHANGELOG.md').read_text(encoding='utf-8')
    heading=re.search(r'^## \[([^]]+)\] - (\d{4}-\d{2}-\d{2})$',changelog,re.M)
    if not heading or heading[1]!=version:
        failures.append('First changelog release must match the package and use an ISO date')
    elif not re.search(r'^### (Added|Changed|Deprecated|Removed|Fixed|Security)$',changelog[heading.end():].split('\n## ',1)[0],re.M):
        failures.append('Current changelog needs standard change categories')
    if '[Unreleased]' in changelog:
        failures.append('Every update needs a dated release; Unreleased is not accepted')
    if not (ROOT/f'docs/releases/v{version}.md').is_file():
        failures.append('Current version is missing release notes')
    citation=(ROOT/'CITATION.cff').read_text()
    if f'version: "{version}"' not in citation:
        failures.append('Citation version differs')
    for field in ['description','license','repository','homepage','documentation','readme']:
        if not package.get(field):failures.append('Missing package metadata: '+field)
    for path in [*ROOT.glob('*.md'),*(ROOT/'docs').rglob('*.md'),*(ROOT/'.agents').rglob('*.md')]:
        if re.search('[\u3400-\u9fff]',path.read_text(encoding='utf-8')):
            failures.append('Maintained documentation must be English: '+str(path.relative_to(ROOT)))
    if staged:
        previous=git('show','HEAD:Cargo.toml',required=False)
        if previous:
            section=changelog[heading.end():].split('\n## ',1)[0] if heading else ''
            notes_path=ROOT/f'docs/releases/v{version}.md'
            notes=notes_path.read_text(encoding='utf-8') if notes_path.is_file() else ''
            reviewed=breaking_release_review(section,notes)
            failures.extend(validate_increment(tomllib.loads(previous)['package']['version'],version,reviewed))
            changed=set(git('diff','--cached','--name-only').splitlines())
            for path in ['Cargo.toml','Cargo.lock','CHANGELOG.md','CITATION.cff',f'docs/releases/v{version}.md']:
                if path not in changed:failures.append('Versioned commit must stage '+path)
        for path in ['Cargo.toml','Cargo.lock','CHANGELOG.md','CITATION.cff',f'docs/releases/v{version}.md']:
            if not (ROOT/path).is_file() or git('show',':'+path,required=False)!= (ROOT/path).read_text(encoding='utf-8'):
                failures.append('Staged metadata differs from checked working file: '+path)
    if require_gate:
        import verify_strict
        report_path=ROOT/'target/strict-verification/report.json'
        report=json.loads(report_path.read_text()) if report_path.is_file() else {}
        if not report.get('local_gate_passed'):
            failures.append('A passing strict gate report is required')
        if report.get('source_sha256')!=verify_strict.source_manifest():
            failures.append('Strict gate source inventory is stale')
        if staged:
            for path in report.get('source_sha256',{}):
                candidate=Path(path)
                if candidate.is_absolute() or path.startswith('..') or path.startswith('dependency:'):
                    continue
                result=subprocess.run(['git','diff','--quiet','--',path],cwd=ROOT)
                if result.returncode:failures.append('Checked source is not fully staged: '+path)
    return failures

def main() -> int:
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--staged',action='store_true')
    parser.add_argument('--require-gate',action='store_true')
    args=parser.parse_args()
    failures=check(args.staged,args.require_gate)
    for failure in failures:print('FAIL: '+failure)
    print('RELEASE CHECK: '+('FAIL' if failures else 'PASS'))
    return int(bool(failures))

if __name__=='__main__':sys.exit(main())
