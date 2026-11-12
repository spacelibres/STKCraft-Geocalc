"""Capture SRFSCC error/retry behavior in independent native processes."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import spiceypy as spice
import spiceypy.utils.libspicehelper as helper

CASES = ['initial_blank', 'initial_overflow', 'partial_code', 'blank_same', 'blank_rename']

def child(case):
    spice.kclear()
    records = []
    def query(name):
        try:
            code, found = spice.srfscc(name, 301)
            records.append({'query': name, 'code': code, 'found': found})
        except spice.utils.exceptions.SpiceyError as error:
            records.append({'query': name, 'error': error.short})
    def assign(names, codes):
        spice.pcpool('NAIF_SURFACE_NAME', names)
        spice.pdpool('NAIF_SURFACE_CODE', codes)
        spice.pipool('NAIF_SURFACE_BODY', [301, 301])
    with spice.no_found_check():
        if case == 'initial_blank':
            assign(['A',' '], [9,10])
            for name in ['A','A','10']: query(name)
        elif case == 'initial_overflow':
            assign(['A','B'], [9,2147483648])
            for name in ['A','A','10']: query(name)
        else:
            assign(['A','B'], [1,2])
            query('A')
            if case == 'partial_code':
                spice.pdpool('NAIF_SURFACE_CODE', [9,2147483648])
            elif case == 'blank_same':
                assign(['A',' '], [9,10])
            elif case == 'blank_rename':
                assign(['C',' '], [9,10])
            for name in ['A','A','B','C']: query(name)
    spice.kclear()
    print(json.dumps({'case':case,'records':records}))

if len(sys.argv) == 3 and sys.argv[1] == '--case':
    child(sys.argv[2])
else:
    records=[]
    for case in CASES:
        raw=subprocess.check_output([sys.executable,__file__,'--case',case],text=True)
        records.append(json.loads(raw))
    report={'toolkit':spice.tkvrsn('TOOLKIT'),
            'native_library_sha256':hashlib.sha256(Path(helper.libspice._name).read_bytes()).hexdigest(),
            'cases':records}
    print(json.dumps(report,indent=2))
