// Stage the article only at the user-confirmed workshop path. Never push.
import {cpSync,existsSync,readFileSync} from 'node:fs';
import {resolve,dirname,join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
const root=dirname(fileURLToPath(import.meta.url));
if(!process.argv[2]) throw new Error('Usage: node stage-website.mjs /path/to/website-review-checkout');
const destination=resolve(process.argv[2]);
const git=(...args)=>execFileSync('git',args,{cwd:destination,encoding:'utf8'}).trim();
const remote=git('remote','get-url','origin');
if(!['https://gitlab.com/mech-lang/web/website.git','git@gitlab.com:mech-lang/web/website.git'].includes(remote)) throw new Error('Not the main website repository.');
const branch=git('branch','--show-current');
if(!branch || ['main','master'].includes(branch)) throw new Error('Create a review branch before staging.');
if(git('status','--porcelain')) throw new Error('Review checkout must be clean; existing work will not be overwritten.');
const target=join(destination,'public/iros-r4r-2026');
if(existsSync(target)) throw new Error('Workshop directory already exists; review its contents before updating.');
const homepage=join(destination,'public/index.html');
const hash=path=>createHash('sha256').update(readFileSync(path)).digest('hex');
const originalHomepageHash=hash(homepage);
const output=join(root,'dist');
const html=readFileSync(join(output,'index.html'),'utf8');
if(!html.includes('<link rel="canonical" href="https://mech-lang.org/iros-r4r-2026/index.html">')) throw new Error('Build does not identify the confirmed workshop destination.');
const manifest=JSON.parse(readFileSync(join(output,'build-manifest.json'),'utf8'));
for(const [path,expected] of Object.entries(manifest.files)) {
  if(hash(join(output,path))!==expected) throw new Error(`Build manifest mismatch: ${path}`);
}
cpSync(output,target,{recursive:true,errorOnExist:true,force:false});
if(hash(homepage)!==originalHomepageHash) throw new Error('Unexpected homepage change.');
console.log(`Staged public/iros-r4r-2026/ on local branch ${branch}; homepage unchanged; no remote write.`);
