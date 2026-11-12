"""Capture independent historical SPK10 state/position outputs before repair."""
from pathlib import Path
import argparse,datetime,json,subprocess,zipfile
import gen_kernel_pool as native
ROOT=Path(__file__).resolve().parents[1]
def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--output-dir',type=Path,required=True)
    dest=parser.parse_args().output_dir.resolve();dest.mkdir(parents=True,exist_ok=False)
    native.SOURCE=ROOT/'tools/reference/spk10_historical_oracle.c'
    record=native.compile_native(dest)
    kernel=dest/'historical-spk10.bsp'
    result=subprocess.run([str(dest/'kernel-pool-oracle.exe'),str(kernel)],cwd=ROOT,capture_output=True)
    (dest/'native-stdout.bin').write_bytes(result.stdout);(dest/'native-stderr.bin').write_bytes(result.stderr)
    if result.returncode or not result.stderr.startswith(b'CRT_FMA:1\r\n'):raise RuntimeError('Native historical SPK10 capture failed')
    rows=result.stdout.decode('ascii').splitlines();assert len(rows)==33696,len(rows)
    additional={}
    with zipfile.ZipFile(ROOT/'target/reference/cspice-n0067.zip') as archive:
        for name in ['spkw10.c','spke10.c','zzsgp4.c','zzteme.c','ttrans.c','unitim.c']:
            path=ROOT/'target/reference/n0067/cspice/src/cspice'/name
            assert path.read_bytes()==archive.read('cspice/src/cspice/'+name)
            additional[name]=native.digest(path)
    record.update(captured_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),
        generator_sha256=native.digest(Path(__file__)),segments=52,queries=len(rows),
        numeric=sum('|N|' in row for row in rows),errors=sum('|E|' in row for row in rows),
        stdout_sha256=native.digest(dest/'native-stdout.bin'),stderr_sha256=native.digest(dest/'native-stderr.bin'),
        generated_spk_sha256=native.digest(kernel),source_sha256_additional=additional,
        input_sha256={name:native.digest(ROOT/name) for name in ['target/kernels/naif0012.tls','target/kernels/de440s.bsp','target/kernels/pck00011.tpc']})
    (dest/'capture.json').write_text(json.dumps(record,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({key:record[key] for key in ['segments','queries','numeric','errors']}))
if __name__=='__main__':main()
