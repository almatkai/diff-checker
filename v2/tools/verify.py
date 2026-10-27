"""Offline fixture, manifest, license, icon and Git-publication verification."""
from pathlib import Path
import hashlib
import json
import subprocess

root = Path(__file__).resolve().parents[1]
p = json.loads((root / 'provenance.json').read_text())
actual = {str(f.relative_to(root)) for f in (root / 'fixtures').rglob('*') if f.is_file()}
assert actual == set(p['files']), 'Fixture inventory mismatch'
for name, info in p['files'].items():
    data = (root / name).read_bytes()
    assert len(data) == info['bytes'], name
    assert hashlib.sha256(data).hexdigest() == info['sha256'], name
assert p['own']['commit'] == 'b9a47b252ba9f8704bd83802ede71f407de27c49'
assert p['beta']['commit'] == 'fd300e276356587726566026f072563eb9c34cb1'
assert p['diff']['version'] == '8.0.3' and p['diff']['license'] == 'BSD-3-Clause'
assert (root/'LICENSE-BSD-3-CLAUSE').read_bytes() == (root/'fixtures/diff-8.0.3/package/LICENSE').read_bytes()
assert (root/'LICENSE-APACHE-2.0').read_bytes() == (root/'fixtures/beta/LICENSE').read_bytes()
assert (root/'LICENSE-MIT').read_text() == (root/'fixtures/own/LICENSE').read_text().replace('\\n','\n')
assert 'MIT AND BSD-3-Clause AND Apache-2.0' in (root/'Cargo.toml').read_text()
icon = p['icon']
assert icon['bytes'] == 201232
assert hashlib.sha256((root/icon['source']).read_bytes()).hexdigest() == icon['sha256']
assert (root/icon['source']).read_bytes() == (root/'fixtures/raycast/assets/icon.png').read_bytes()
manifest = json.loads((root/'manifest.json').read_text())
assert manifest['icon'] == icon['url']
assert icon['destination'] == 'src/renderer/public/extensions/icons/diff-checker.png'
assert manifest['id'] == 'com.tezbar.native.diff-checker'
assert manifest['replaces'] == ['raycast.diff-checker']
assert [(c['name'],c['title']) for c in manifest['commands']] == [('check-diff','Check Diff'),('compare-json','Compare JSON')]
assert all(c['runtime']=='rust' and c['platforms']==['macOS','Windows'] for c in manifest['commands'])
assert [(v['platform'],v['state']) for v in manifest['verification']] == [('macOS','not-run'),('Windows','not-run')]
assert 'destiner' in manifest['description'] and 'hayk_hayrapetian' in manifest['description']
for name in ['beta-host','tauri-stub']:
    manifest_path=root/'tests'/name/'Cargo.toml'
    if manifest_path.exists():assert manifest_path.read_bytes()==(root/('fixtures/harness/Cargo.'+name+'.toml')).read_bytes()
# Use the complete actual pinned beta schema, including additionalProperties=false.
script = """import fs from 'node:fs';import Ajv from 'ajv/dist/2020.js';
const schema=JSON.parse(fs.readFileSync('../fixtures/beta/schemas/extension-v2.schema.json'));
const manifest=JSON.parse(fs.readFileSync('../manifest.json'));
const validate=new Ajv({strict:true}).compile(schema);
if(!validate(manifest))throw new Error(JSON.stringify(validate.errors));
"""
subprocess.run(['node','--input-type=module','-e',script], cwd=root/'tools',check=True)
# Verify publication includes every fixture, rather than merely checking local ignored files.
git = subprocess.run(['git','rev-parse','--show-toplevel'],cwd=root,capture_output=True,text=True)
if git.returncode == 0 and Path(git.stdout.strip()) == root.parent:
    repo = root.parent
    listing = subprocess.check_output(['git','ls-files','--cached','--others','--exclude-standard','--','v2'],cwd=repo,text=True).splitlines()
    published = set(listing)
    assert all('v2/'+f in published for f in actual), 'Required fixture excluded by Git ignore rules'
    assert 'v2/fixtures/diff-8.0.3/package/dist/diff.js' in published
    print('Git-publication set includes every fixture, including npm dist (no force-add needed).')
print('Full beta schema, %d fixture hashes, exact icon and source/dependency licenses verified.' % len(actual))
