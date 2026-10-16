// SOURCE ONLY until root reviews/copies this diagnostic into its push-only branch.
import assert from 'node:assert/strict';
import {mkdir,readFile,writeFile,realpath} from 'node:fs/promises';
import {join,resolve,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {randomBytes} from 'node:crypto';
import {capture,descriptor,ordinaryRoot} from './files.mjs';
import {verifyEditorArchive,verifyEditor} from './materials.mjs';
import {command} from './process.mjs';
import {EDITOR_ARCHIVES} from './pins.mjs';
const root=process.argv[2],repo=resolve(fileURLToPath(new URL('../../..',import.meta.url)));
assert(process.argv.length===3&&process.platform==='linux'&&process.arch==='x64'&&process.version==='v22.22.1');
assert(root===resolve(root)&&/^m6-editor-diag-[1-9][0-9]*-[1-9][0-9]*$/.test(root.split('/').at(-1)));
await ordinaryRoot(root);assert(await realpath(root)===root);
const deadline=performance.now()+600000,stage=join(root,'stage'),logs=join(root,'preparation');
await mkdir(stage,{mode:0o700});await mkdir(logs,{mode:0o700});
const options={cwd:repo,env:{...process.env},output:logs,deadline};
const captured=await capture(join(root,'material/editor.tar.gz'),join(stage,'authenticated-editor.archive'),EDITOR_ARCHIVES.linux,deadline);
const archive=await verifyEditorArchive(captured.captured.path,deadline),editorRoot=join(stage,'editor');await mkdir(editorRoot);
await command('tar',['-xf',captured.captured.path,'-C',editorRoot],{...options,name:'extract-authenticated-editor'});
const editor=await verifyEditor(editorRoot,deadline);assert(editor.productCommit==='7debcd0e2acdea1c52de81bf9ee1620444407dda');const helper=join(dirname(editor.executable),'chrome-sandbox');
assert(editor.descriptor.bytes===219843800&&editor.descriptor.sha256==='34912494509486296a3bd9aecb512694c98523d0f6f23cfceb5a95b5088c721b');
const helperPin=await descriptor(helper,deadline);assert(helperPin.bytes===15232&&helperPin.sha256==='2015304df43beff0b74409cdc4d858107efcde650d9bc8afb1288041d62cec4b');
const row=editor.inventory.entries.find(x=>x.path==='VSCode-linux-x64/chrome-sandbox');assert(row.mode===0o755);
await writeFile(join(root,'PREPARED.json'),JSON.stringify({archive,captured,editor,diagnosticOnly:true}),{flag:'wx',mode:0o400});
for(const path of ['probe','profile','extensions','tmp'])await mkdir(join(stage,path),{mode:0o700});
const source={commit:(await command('git',['rev-parse','HEAD'],{...options,name:'source-head'})).trim(),tree:(await command('git',['show','-s','--format=%T','HEAD'],{...options,name:'source-tree'})).trim()};
assert(source.commit===process.env.GITHUB_SHA);
const config={format:'private.editor-scoped-userns-diagnostic.v1',root,repo,stage,source,uid:process.getuid(),gid:process.getgid(),
 unit:`m6-editor-diag-${process.env.GITHUB_RUN_ID}-${process.env.GITHUB_RUN_ATTEMPT}-${randomBytes(6).toString('hex')}.service`,
 node:await descriptor(process.execPath,deadline),code:editor.descriptor,helper:helperPin,prepared:await descriptor(join(root,'PREPARED.json'),deadline),
 probe:await descriptor(join(repo,'scripts/m6/installed-editor/diagnostic-probe.mjs'),deadline),diagnosticOnly:true};
assert(config.uid>0&&config.gid>=0&&/^m6-editor-diag-[1-9][0-9]*-[1-9][0-9]*-[a-f0-9]{12}\.service$/.test(config.unit));
await writeFile(join(root,'CONFIG.json'),JSON.stringify(config),{flag:'wx',mode:0o400});
console.log(JSON.stringify({prepared:true,diagnosticOnly:true,source,unit:config.unit}));
