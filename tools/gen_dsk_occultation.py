"""Capture independent DSK-versus-point GFOCLT, including both depth orders."""
from pathlib import Path
import argparse,subprocess,json,zipfile
from datetime import datetime,timezone
import gen_kernel_pool as native
ROOT=Path(__file__).resolve().parents[1]
def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output-dir',type=Path,required=True)
    dest=parser.parse_args().output_dir.resolve();dest.mkdir(parents=True,exist_ok=False)
    native.SOURCE=ROOT/'tools/reference/dsk_occultation_oracle.c';record=native.compile_native(dest)
    spk=dest/'moving-point.bsp'
    result=subprocess.run([str(dest/'kernel-pool-oracle.exe'),str(spk)],cwd=ROOT,capture_output=True)
    for name,raw in [('stdout',result.stdout),('stderr',result.stderr)]:(dest/('native-'+name+'.bin')).write_bytes(raw)
    if result.returncode or not result.stderr.startswith(b'CRT_FMA:1\r\n'):raise RuntimeError('Native DSK occultation capture failed')
    rows=result.stdout.decode('ascii').splitlines()
    if len(rows)!=864:raise ValueError('Native row count differs')
    extra={}
    with zipfile.ZipFile(ROOT/'target/reference/cspice-n0067.zip') as archive:
        for name in ['zzgfocu.c','gfoclt.c','gfoclt_c.c','zzgfsolv.c','sincpt.c']:
            path=ROOT/'target/reference/n0067/cspice/src/cspice'/name
            if path.read_bytes()!=archive.read('cspice/src/cspice/'+name):raise ValueError('Native source differs: '+name)
            extra[name]=native.digest(path)
    record.update(captured_utc=datetime.now(timezone.utc).isoformat(),wrapper_sha256=native.digest(Path(__file__)),
        operations=len(rows),windows=sum('|W|' in r for r in rows),nonempty=sum('|W|' in r and not r.endswith('|W|0') for r in rows),errors=sum('|E|' in r for r in rows),
        source_sha256_additional=extra,input_sha256={name:native.digest(ROOT/name) for name in ['target/kernels/de440s.bsp','target/kernels/pck00011.tpc','tests/golden/dsk/frames.tf','tests/golden/dsk/optimized.bds']},
        generated_spk_sha256=native.digest(spk),stdout_sha256=native.digest(dest/'native-stdout.bin'),stderr_sha256=native.digest(dest/'native-stderr.bin'))
    (dest/'capture.json').write_text(json.dumps(record,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({key:record[key] for key in ['operations','windows','nonempty','errors']}))
if __name__=='__main__':main()
