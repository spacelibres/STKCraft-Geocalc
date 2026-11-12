"""Capture native LATSRF before implementing shape-aware coordinate facades."""
from pathlib import Path
import argparse,subprocess,json,zipfile
from datetime import datetime,timezone
import gen_kernel_pool as native

ROOT=Path(__file__).resolve().parents[1]

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output-dir',type=Path,required=True)
    dest=parser.parse_args().output_dir.resolve();dest.mkdir(parents=True,exist_ok=False)
    native.SOURCE=ROOT/'tools/reference/surface_coordinates_oracle.c'
    record=native.compile_native(dest)
    result=subprocess.run([str(dest/'kernel-pool-oracle.exe')],cwd=ROOT,capture_output=True)
    (dest/'native-stdout.bin').write_bytes(result.stdout);(dest/'native-stderr.bin').write_bytes(result.stderr)
    if result.returncode or not result.stderr.startswith(b'CRT_FMA:1\r\n'):
        raise RuntimeError('Independent native surface-coordinate capture failed')
    rows=result.stdout.decode('ascii').splitlines()
    if len(rows)!=2352:raise ValueError('Native operation count differs')
    source_hashes={}
    with zipfile.ZipFile(ROOT/'target/reference/cspice-n0067.zip') as archive:
        for name in ['latsrf.c','latsrf_c.c','latrec.c','edpnt.c','zzgfocu.c','srfs2c.c','zzsrftrn.c','zzsrfker.c']:
            path=ROOT/'target/reference/n0067/cspice/src/cspice'/name
            if path.read_bytes()!=archive.read('cspice/src/cspice/'+name):raise ValueError('Official source differs: '+name)
            source_hashes[name]=native.digest(path)
    record.update(captured_utc=datetime.now(timezone.utc).isoformat(),wrapper_sha256=native.digest(Path(__file__)),
        operations=len(rows),numeric=sum('|N|' in row for row in rows),errors=sum('|E|' in row for row in rows),
        source_sha256_additional=source_hashes,
        input_sha256={name:native.digest(ROOT/name) for name in ['target/kernels/de440s.bsp','target/kernels/pck00011.tpc','tests/golden/dsk/frames.tf','tests/golden/dsk/optimized.bds','tests/golden/dsk/dense_lat.bds']},
        stdout_sha256=native.digest(dest/'native-stdout.bin'),stderr_sha256=native.digest(dest/'native-stderr.bin'))
    (dest/'capture.json').write_text(json.dumps(record,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(dict(operations=len(rows),numeric=record['numeric'],errors=record['errors'])))

if __name__=='__main__':main()
