// Expected algorithms are executed from byte-exact OWN TS and npm diff fixtures.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath, pathToFileURL} from 'node:url';
import {spawnSync} from 'node:child_process';
import {build} from 'esbuild';
const here=path.dirname(fileURLToPath(import.meta.url)),root=path.resolve(here,'..');
process.chdir(root);fs.mkdirSync(path.join(here,'node_modules/.cache'),{recursive:true});
await build({stdin:{contents:`export {getDiffText} from './fixtures/own/src/utils'; export {computeDiff} from './fixtures/own/src/lib/diff'; export {validateJSON} from './fixtures/own/src/lib/validate'`,resolveDir:root,loader:'ts'},bundle:true,format:'esm',platform:'node',outfile:path.join(here,'node_modules/.cache/original.mjs'),plugins:[{name:'pinned-diff',setup(b){b.onResolve({filter:/^diff$/},()=>({path:path.join(root,'fixtures/diff-8.0.3/package/libesm/index.js')}))}}]});
const {getDiffText,computeDiff}=await import(pathToFileURL(path.join(here,'node_modules/.cache/original.mjs')));
let seed=0x936bad;function random(n){seed=(Math.imul(seed,1664525)+1013904223)>>>0;return seed%n;}
const cases=[], expected=[];
function add(kind,a,b=''){cases.push({kind,a,b});expected.push(kind==='text'?getDiffText(a,b):kind==='json'?computeDiff(JSON.parse(a.trim()),JSON.parse(b.trim())):JSON.stringify(JSON.parse(a.trim()),null,2));}
const atoms=['','a','b','a',' ', '\t', 'Қазақ', '😀', 'e\u0301','\r','```','````','![](https://invalid.example/payload)','<script>x</script>'];
const endings=['','\n','\r\n','\n\n'];
for(const a of ['', '\n','\n\n','a','a\n','a\r\n','\r','a\nb','b\na','a\na\nb','a\nb\na'])for(const b of ['', '\n','a','b\n','a\r\n','b\na','a\na\nb'])add('text',a,b);
for(let i=0;i<6000;i++) {
 const lines=()=>Array.from({length:random(26)},()=>atoms[random(atoms.length)]).join(random(4)===0?'\r\n':'\n')+endings[random(4)];
 const a=lines();let b=lines();if(i%3===0){const x=a.split('\n');x.splice(random(x.length+1),random(3),atoms[random(atoms.length)]);b=x.join('\n');}add('text',a,b);
}
const special=['null','true','false','-0','1e400','-1e400','1e-400','9007199254740993','1000000000000000128','5e-324','1e-7','1e-6','1e20','1e21','1.234567890123456789','"\\ud800"','"\\udfff"','"\\ud83d\\ude00"','"😀Қазақ\\u0000\\b\\f\\n\\r\\t"','{"b":1,"2":0,"1":5,"a":2,"b":7,"01":8,"4294967294":9,"4294967295":10}','{"__proto__":1,"constructor":0,"toString":2}','{"\\ud800":1,"😀":2,"\\ud800":3}','[1,null,{},[]]'];
for(const a of special) {add('format',a);for(const b of special)add('json',a,b);}
function val(depth=0){if(depth>3)return random(200)-100;switch(random(7)){case 0:return null;case 1:return !!random(2);case 2:return (random(200)-100)*10**(random(40)-20);case 3:return atoms[random(atoms.length)];case 4:return Array.from({length:random(7)},()=>val(depth+1));default:{const o={};for(let n=random(8);n--;)o[['b','2','0','a','02','4294967295','😀'][random(7)]]=val(depth+1);return o;}}}
for(let i=0;i<2200;i++){const a=JSON.stringify(val()),b=JSON.stringify(val());add('json',a,b);}
for(let i=0;i<1200;i++){const digits=Array.from({length:1+random(60)},()=>random(10)).join('');const n=(random(2)?'-':'')+(digits.replace(/^0/,'1'))+(random(2)?'.'+digits:'')+'e'+(random(800)-400);add('format',n);}
for(let gap=0;gap<22;gap++)for(let edge=0;edge<10;edge++) {
 const a=Array.from({length:50+edge},(_,i)=>i),b=[...a];b[edge]=999;b[Math.min(49,edge+gap+8)]=888;add('json',JSON.stringify(a),JSON.stringify(b));
}
const buildResult=spawnSync('cargo',['build','--locked','--example','oracle'],{stdio:'inherit'});assert.equal(buildResult.status,0);
const result=spawnSync(path.join(process.env.CARGO_TARGET_DIR || path.join(root,'target'),'debug/examples/oracle'),[],{input:cases.map(c=>JSON.stringify(c)).join('\n')+'\n',encoding:'utf8',maxBuffer:160*1024*1024});assert.equal(result.status,0,result.stderr);
const actual=result.stdout.trimEnd().split('\n').map(JSON.parse);assert.equal(actual.length,cases.length);
for(let i=0;i<cases.length;i++){assert.equal(actual[i].ok,true,JSON.stringify(cases[i]));assert.deepEqual(actual[i].value,expected[i],`case ${i}: ${JSON.stringify(cases[i])}`);}
console.log(`Original OWN TS + diff@8.0.3 oracle: ${cases.length} exact comparisons passed (seed 0x936bad).`);
