"""Exercise an extracted Cargo package using only its own fixtures and local harnesses."""
from pathlib import Path
import os
import shutil
import subprocess
import tarfile
import tempfile

root=Path(__file__).resolve().parents[1]
subprocess.run(['cargo','package','--allow-dirty'],cwd=root,check=True)
archive=root/'target/package/tezbar-diff-checker-0.1.0.crate'
with tarfile.open(archive) as tar:
    names=tar.getnames()
    assert not any('/node_modules/' in n or '/.tmp/' in n or '/target/' in n for n in names)
    expected=['LICENSE-MIT','LICENSE-BSD-3-CLAUSE','LICENSE-APACHE-2.0','provenance.json','verification.json','src/tezbar_adapter.rs','fixtures/diff-8.0.3/package/dist/diff.js','fixtures/diff-8.0.3/package/dist/diff.min.js']
    for f in expected: assert 'tezbar-diff-checker-0.1.0/'+f in names,f
    tempbase=root/'.tmp'
    tempbase.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='package-',dir=tempbase) as temp:
        for member in tar.getmembers():
            assert '..' not in Path(member.name).parts and not Path(member.name).is_absolute()
            if member.isfile():
                dest=Path(temp)/member.name
                dest.parent.mkdir(parents=True,exist_ok=True)
                dest.write_bytes(tar.extractfile(member).read())
        extracted=Path(temp)/'tezbar-diff-checker-0.1.0'
        for f in (root/'fixtures').rglob('*'):
            if f.is_file():assert (extracted/f.relative_to(root)).read_bytes()==f.read_bytes()
        # Cargo intentionally removes unversioned local path-only DEV dependencies.
        # Restore its included original manifest so extracted tests use the INCLUDED
        # byte-exact beta + test-only host/Tauri crates rather than registry packages.
        shutil.copyfile(extracted/'Cargo.toml.orig',extracted/'Cargo.toml')
        shutil.copyfile(extracted/'fixtures/build/Cargo.lock',extracted/'Cargo.lock')
        # Cargo excludes nested package manifests; their exact templates and code
        # are explicitly carried in fixtures/harness.
        for name in ['beta-host','tauri-stub']:
            dest=extracted/'tests'/name/'Cargo.toml'
            dest.parent.mkdir(parents=True,exist_ok=True)
            shutil.copyfile(extracted/('fixtures/harness/Cargo.'+name+'.toml'),dest)
        env=dict(os.environ,CARGO_TARGET_DIR=str(root/'target/extracted-package'))
        for cmd in [['cargo','test','--all-targets','--locked','--offline'],['cargo','+1.85.0','test','--all-targets','--locked','--offline']]:
            subprocess.run(cmd,cwd=extracted,env=env,check=True)
        subprocess.run(['npm','ci','--prefix','tools','--ignore-scripts'],cwd=extracted,check=True)
        subprocess.run(['npm','test','--prefix','tools'],cwd=extracted,env=env,check=True)
print('Cargo archive verified; extracted stable + 1.85 Rust tests and original-source/render/schema oracles passed.')
