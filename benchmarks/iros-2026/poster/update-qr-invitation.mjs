// Scoped QR and motivation revision. Uses Artifact Tool for editable changes,
// then preserves untouched OOXML parts byte-for-byte, including charts and QR pixels.
// Final exported revision: IROS-2026-Mech-Poster-qr-v2.{pptx,pdf,png}.
// Candidate output must be finalized and rendered with the presentation skill.
import fs from 'node:fs/promises';
import {createRequire} from 'node:module';
import path from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
const modules=process.env.RUNTIME_NODE_MODULES;
if(!modules || !process.argv[2])throw Error('Set RUNTIME_NODE_MODULES and pass a private build directory.');
const {FileBlob,PresentationFile}=await import(pathToFileURL(path.join(modules,'@oai/artifact-tool/dist/artifact_tool.mjs')).href);
const root=path.resolve(process.argv[2]);
const source=fileURLToPath(new URL('./IROS-2026-Mech-Poster-prose-v3.pptx',import.meta.url));
const req=createRequire(import.meta.url),mods=modules;
const JSZip=req(mods+'/jszip'),{xml2js,js2xml}=req(mods+'/xml-js');
const p=await PresentationFile.importPptx(await FileBlob.load(source)),s=p.slides.items[0];
const shapes=new Map(s.shapes.items.map(x=>[x.name,x]));
const invitation='Scan the QR at the bottom for a demo.';
const prefix='Embedded numerical kernels are one use of Mech, which also has first-class state machines and pattern matching. ';
const rust=String(shapes.get('Motivation Rust').text).replace('Numerical kernels can still require','Still, numerical kernels can require');
const mech='Mech is an open-source, Rust-native embeddable numerical language. For robotics, its reactive execution handles sensor updates and retains accepted state after rejected turns. The EKF results below show SIMD throughput comparable to Rust.';
shapes.get('Motivation Rust').text=rust;
shapes.get('Motivation Mech').text=mech;
shapes.get('Motivation scope and demo').text=prefix+invitation;
shapes.get('Motivation scope and demo').text.get(invitation).bold=true;
shapes.get('Motivation scope and demo').text.get(invitation).fill='#F6C04E';
const url=s.shapes.add({name:'QR human-readable URL',geometry:'textbox',position:{left:402,top:4369,width:1150,height:34},fill:'none',line:{fill:'none',width:0}});
url.text='mech-lang.org/iros-r4r-2026/';
url.text.style={typeface:'Avenir Next',fontSize:28,color:'#F6C04E',wrap:'none',autoFit:'none',verticalAlignment:'top',insets:{top:0,left:0,right:0,bottom:0}};
await fs.mkdir(root+'/build',{recursive:true});await fs.mkdir(root+'/output',{recursive:true});
await (await PresentationFile.exportPptx(p)).save(root+'/build/api-candidate.pptx');
await fs.writeFile(root+'/build/after.layout.json',await (await s.export({format:'layout'})).text());
const parse=x=>xml2js(x,{compact:false,captureSpacesBetweenElements:true}),ser=x=>js2xml(x,{compact:false});
const desc=(e,n)=>(e.elements??[]).flatMap(c=>[...(c.name===n?[c]:[]),...desc(c,n)]);
const z=await JSZip.loadAsync(await fs.readFile(source)),draft=await JSZip.loadAsync(await fs.readFile(root+'/build/api-candidate.pptx'));
const slidePath='ppt/slides/slide1.xml';let xml=await z.file(slidePath).async('string');
const tree=desc(parse(xml),'p:spTree')[0];const changed=[];
for(const node of tree.elements){
 const name=desc(node,'p:cNvPr')[0]?.attributes?.name;
 if(!['Motivation Rust','Motivation Mech','Motivation scope and demo'].includes(name))continue;
 const before=ser({elements:[node]});
 if(name==='Motivation Rust')desc(node,'a:t')[0].elements=[{type:'text',text:rust}];
 if(name==='Motivation Mech')desc(node,'a:t')[0].elements=[{type:'text',text:mech}];
 if(name==='Motivation scope and demo'){
  const ts=desc(node,'a:t');if(ts.length!==2)throw Error('Expected two callout runs');
  ts[0].elements=[{type:'text',text:prefix}];ts[1].elements=[{type:'text',text:invitation}];
 }
 if(!xml.includes(before))throw Error('Unmatched source object '+name);
 xml=xml.replace(before,ser({elements:[node]}));changed.push(name);
}
const newTree=desc(parse(await draft.file(slidePath).async('string')),'p:spTree')[0];
const urlNode=newTree.elements.find(e=>desc(e,'p:cNvPr')[0]?.attributes?.name==='QR human-readable URL');
if(!urlNode)throw Error('Missing new URL object');
const nv=desc(urlNode,'p:cNvPr')[0];nv.attributes.id=String(Math.max(...desc(tree,'p:cNvPr').map(x=>Number(x.attributes.id)))+1);
nv.elements??=[];nv.elements.unshift({type:'element',name:'a:hlinkClick',attributes:{'r:id':'rIdPosterDemo','xmlns:a':'http://schemas.openxmlformats.org/drawingml/2006/main','xmlns:r':'http://schemas.openxmlformats.org/officeDocument/2006/relationships'}});
xml=xml.replace('</p:spTree>',ser({elements:[urlNode]})+'</p:spTree>');
z.file(slidePath,xml);
const relPath='ppt/slides/_rels/slide1.xml.rels',rels=parse(await z.file(relPath).async('string'));
rels.elements.find(e=>e.name==='Relationships').elements.push({type:'element',name:'Relationship',attributes:{Type:'http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink',Target:'https://mech-lang.org/iros-r4r-2026/index.html',TargetMode:'External',Id:'rIdPosterDemo'}});
z.file(relPath,ser(rels));
await fs.writeFile(root+'/build/candidate.pptx',await z.generateAsync({type:'nodebuffer'}));
await fs.writeFile(root+'/build/changes.json',JSON.stringify({source,changed,added:['QR human-readable URL'],urlDecodedFrom:'ppt/media/image2.png',decodedURL:'https://mech-lang.org/iros-r4r-2026/index.html',displayURL:'mech-lang.org/iros-r4r-2026/',shortURLHttpStatus:200},null,2));
console.log(root+'/build/candidate.pptx');
