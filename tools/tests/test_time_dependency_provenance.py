"""Realistic Cargo graph/source regressions, separate from numerical oracles."""
import hashlib, importlib.util, io, json, subprocess, tarfile, tempfile, unittest
from pathlib import Path
from unittest.mock import patch

spec=importlib.util.spec_from_file_location('time_provenance_gate',Path(__file__).resolve().parents[1]/'verify_strict.py')
gate=importlib.util.module_from_spec(spec);spec.loader.exec_module(gate)
REGISTRY='registry+https://github.com/rust-lang/crates.io-index'

class TimeDependencyProvenance(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        base=Path(self.temp.name);self.workspace=base/'workspace';self.root=self.workspace/'geocalc';self.root.mkdir(parents=True)
        (self.root/'Cargo.toml').write_text('[package]\nname="stkcraft-geocalc"\nversion="0.1.12"\n[dependencies]\nstkcraft-time="0.1.31"\n')
        (self.root/'Cargo.lock').write_text('version=3\n')
        self.directory=base/'registry/src/index/stkcraft-time-0.1.31';(self.directory/'src').mkdir(parents=True)
        files={'Cargo.toml':b'[package]\nname="stkcraft-time"\nversion="0.1.31"\n', 'src/lib.rs':b'// selected registry source\n', 'src/naif0012.tls':b'original embedded kernel\n'}
        archive=base/'registry/cache/index/stkcraft-time-0.1.31.crate';archive.parent.mkdir(parents=True)
        with tarfile.open(archive,'w:gz') as tar:
            for name,raw in files.items():
                path=self.directory/name;path.write_bytes(raw)
                member=tarfile.TarInfo('stkcraft-time-0.1.31/'+name);member.size=len(raw);tar.addfile(member,io.BytesIO(raw))
        self.checksum=hashlib.sha256(archive.read_bytes()).hexdigest()
        self.local=base/'local-time';(self.local/'src').mkdir(parents=True)
        (self.local/'Cargo.toml').write_bytes(files['Cargo.toml']);(self.local/'src/lib.rs').write_bytes(b'// local original source\n')
        self.metadata={'workspace_root':str(self.workspace),'packages':[
            {'id':'time-local','name':'stkcraft-time','version':'0.1.31','source':None,'manifest_path':str(self.local/'Cargo.toml')},
            {'id':'time-registry','name':'stkcraft-time','version':'0.1.31','source':REGISTRY,'manifest_path':str(self.directory/'Cargo.toml')},
            {'id':'geo','name':'stkcraft-geocalc','version':'0.1.12','source':None,'manifest_path':str(self.root/'Cargo.toml'),
             'dependencies':[{'name':'stkcraft-time','rename':None,'kind':None,'req':'^0.1.31'}]}],
            'resolve':{'nodes':[{'id':'geo','features':[],'deps':[{'name':'stkcraft_time','pkg':'time-registry','dep_kinds':[{'kind':None,'target':None}]}]},
                                {'id':'time-registry','features':['default','std'],'deps':[]}, {'id':'time-local','features':['default','std'],'deps':[]}]}}
        self.write_lock()
    def write_lock(self):
        (self.workspace/'Cargo.lock').write_text('version=3\n[[package]]\nname="stkcraft-time"\nversion="0.1.31"\n\n[[package]]\nname="stkcraft-time"\nversion="0.1.31"\nsource="'+REGISTRY+'"\nchecksum="'+self.checksum+'"\n')
    def resolve(self,**kwargs):
        output=subprocess.CompletedProcess([],0,json.dumps(self.metadata),'')
        with patch.object(gate,'ROOT',self.root),patch.object(gate.subprocess,'run',return_value=output):
            return gate.time_dependency(**kwargs)
    def test_compatible_range_uses_actual_direct_edge_and_workspace_lock(self):
        record=self.resolve()
        self.assertEqual(record['package_id'],'time-registry')
        self.assertEqual(record['version_requirement'],'^0.1.31')
        self.assertEqual(record['package_checksum'],self.checksum)
        self.assertEqual(record['source_kind'],'registry')
        self.assertEqual(Path(record['lock_path']),self.workspace/'Cargo.lock')
    def test_renamed_dependency_still_follows_its_normal_edge(self):
        self.metadata['packages'][-1]['dependencies'][0]['rename']='time_core'
        self.metadata['resolve']['nodes'][0]['deps'][0]['name']='time_core'
        self.assertEqual(self.resolve()['package_id'],'time-registry')
    def test_ambiguous_or_missing_direct_edge_is_rejected(self):
        self.metadata['resolve']['nodes'][0]['deps']=[]
        with self.assertRaises(RuntimeError):self.resolve()
        self.metadata['resolve']['nodes'][0]['deps']=[{'name':'stkcraft_time','pkg':name,'dep_kinds':[{'kind':None}]} for name in ['time-local','time-registry']]
        with self.assertRaises(RuntimeError):self.resolve()
    def test_local_override_is_explicit_and_cannot_be_release_evidence(self):
        self.metadata['resolve']['nodes'][0]['deps'][0]['pkg']='time-local'
        with self.assertRaises(RuntimeError):self.resolve()
        record=self.resolve(allow_local=True)
        self.assertEqual(record['package_id'],'time-local')
        self.assertEqual(record['source_kind'],'local-path')
        self.assertIsNone(record['package_checksum'])
        self.assertIsNone(record['archive_path'])
    def test_modified_embedded_kernel_and_missing_registry_data_fail(self):
        kernel=self.directory/'src/naif0012.tls';kernel.write_bytes(b'changed kernel')
        with self.assertRaises(RuntimeError):self.resolve()
        kernel.unlink()
        with self.assertRaises(RuntimeError):self.resolve()
    def test_extra_registry_source_is_rejected(self):
        (self.directory/'src/injected.rs').write_text('unexpected source')
        with self.assertRaises(RuntimeError): self.resolve()
    def test_extra_build_script_is_rejected(self):
        (self.directory/'build.rs').write_text('unexpected build script')
        with self.assertRaises(RuntimeError): self.resolve()
    def test_workspace_without_member_lock_and_custom_local_sources(self):
        (self.root/'Cargo.lock').unlink()
        self.metadata['resolve']['nodes'][0]['deps'][0]['pkg']='time-local'
        (self.local/'lib.rs').write_text('first')
        (self.local/'src/target').mkdir()
        (self.local/'src/target/mod.rs').write_text('nested source')
        record=self.resolve(allow_local=True)
        with patch.object(gate,'ROOT',self.root), patch.object(gate,'time_dependency',return_value=record):
            before=gate.source_manifest(allow_local=True)
            (self.local/'lib.rs').write_text('second')
            after=gate.source_manifest(allow_local=True)
        self.assertNotEqual(before,after)
        self.assertIn('dependency:stkcraft-time@0.1.31/src/target/mod.rs',before)
        self.assertIn('dependency:workspace/Cargo.lock',before)
    def test_local_target_outside_package_is_rejected(self):
        self.metadata['resolve']['nodes'][0]['deps'][0]['pkg']='time-local'
        self.metadata['packages'][0]['targets']=[{'src_path':str(self.workspace/'other.rs')}]
        with self.assertRaises(RuntimeError): self.resolve(allow_local=True)
        self.metadata['packages'][0]['targets']=[{'src_path':str(self.local/'target/generated.rs')}]
        with self.assertRaises(RuntimeError): self.resolve(allow_local=True)
    def test_feature_changes_change_dependency_fingerprint(self):
        first=self.resolve()['metadata_sha256']
        self.metadata['resolve']['nodes'][1]['features'].append('mcp')
        self.assertNotEqual(first,self.resolve()['metadata_sha256'])

if __name__=='__main__':unittest.main()
