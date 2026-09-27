/** Workshop development timeline, adapted from the poster's milestones. */
import {mkdirSync, writeFileSync} from 'node:fs';
import {join} from 'node:path';

export const TIMELINE_MILESTONES = Object.freeze([
  {version:'Start', label:'Implementation begins', lines:['Implementation','begins'], date:'2018', status:'completed'},
  {version:'v0.0.1', label:'First alpha release', lines:['First alpha','release'], date:'2019', status:'completed'},
  {version:'v0.1', label:'Proof of concept', lines:['Proof of','concept'], date:'2023', status:'completed'},
  {version:'v0.2', label:'Data specification', lines:['Data','specification'], date:'2025', status:'completed'},
  {version:'v0.3', label:'Program specification', lines:['Program','specification'], date:'2026', status:'completed'},
  {version:'v0.4', label:'Reactive Runtime', lines:['Reactive','Runtime'], date:'In progress', status:'current'},
  {version:'v0.5', label:'Distribution & concurrency', lines:['Distribution','& concurrency'], date:'2027 (planned)', status:'planned'},
  {version:'v1.0', label:'Release Candidate', lines:['Release','Candidate'], date:'2028 (planned)', status:'planned'},
].map(item=>Object.freeze({...item,lines:Object.freeze(item.lines)})));

const COLORS = {
  background:'#101719', text:'#e9eeef', muted:'#b0c2c9',
  completed:'#4cac97', current:'#f6c04e', planned:'#cd8dca',
};
const escape = value => String(value).replace(/[&<>"']/g, character => ({
  '&':'&amp;', '<':'&lt;', '>':'&gt;', '"':'&quot;', "'":'&apos;',
}[character]));
const text = (x,y,value,attributes='') => `<text x="${x}" y="${y}" ${attributes}>${escape(value)}</text>`;

export function timelineSvg({mobile=false}={}) {
  const id = `mech-development-timeline${mobile?'-mobile':''}`;
  const width = mobile?440:1100, height = mobile?680:250;
  const nodes = TIMELINE_MILESTONES.map((item,index)=>({
    ...item, x:mobile?30:70+index*(960/7), y:mobile?82+index*76:82,
  }));
  const description = 'Milestones from the workshop poster, ordered by development stage rather than elapsed time. ' +
    TIMELINE_MILESTONES.map(item=>`${item.version}: ${item.label}, ${item.date}, ${item.status}.`).join(' ') +
    ' Future dates are planned, not completed releases.';
  const content = [text(mobile?20:25,mobile?33:33,'Mech development','class="title"')];
  for(let index=1;index<nodes.length;index++) {
    const previous=nodes[index-1],next=nodes[index];
    // The segment into v0.4, not only its marker, identifies current work.
    const color=COLORS[next.status];
    content.push(`<line x1="${previous.x}" y1="${previous.y}" x2="${next.x}" y2="${next.y}" stroke="${color}" stroke-width="3" aria-hidden="true"/>`);
  }
  for(const node of nodes) {
    const {version,label,lines,date,status,x,y}=node;
    const color=COLORS[status], radius=mobile?11:10;
    content.push(`<g role="group" data-milestone="${escape(version)}" data-status="${status}" aria-label="${escape(`${version}: ${label}, ${date}, ${status}`)}">`);
    content.push(`<circle cx="${x}" cy="${y}" r="${radius}" fill="${status==='completed'?color:COLORS.background}" stroke="${color}" stroke-width="2.3"/>`);
    if(status==='completed') content.push(`<path d="M${x-4.5} ${y} l3 3.5 l6 -7" fill="none" stroke="${COLORS.background}" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round"/>`);
    if(status==='current') content.push(`<circle cx="${x}" cy="${y}" r="3.7" fill="${color}"/>`);
    if(mobile) {
      content.push(text(62,y-9,version,`class="version" style="fill:${color}"`));
      content.push(text(62,y+14,label,'class="label"'));
      content.push(text(62,y+35,date,`class="date"${status!=='completed'?` style="fill:${color}"`:''}`));
    } else {
      content.push(text(x,y+38,version,`class="version" text-anchor="middle" style="fill:${color}"`));
      lines.forEach((line,index)=>content.push(text(x,y+67+index*20,line,'class="label" text-anchor="middle"')));
      content.push(text(x,y+115,date,`class="date" text-anchor="middle"${status!=='completed'?` style="fill:${color}"`:''}`));
    }
    content.push('</g>');
  }
  return `<svg xmlns="http://www.w3.org/2000/svg" id="${id}" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}" role="img" aria-labelledby="${id}-title ${id}-description">
<title id="${id}-title">Mech development milestones</title>
<desc id="${id}-description">${escape(description)}</desc>
<style>
#${id} text { font-family: Inter, Arial, sans-serif; fill: ${COLORS.text}; }
#${id} .title, #${id} .version { font-family: 'Fira Code', monospace; font-weight: 600; }
#${id} .title { font-size: ${mobile?22:20}px; }
#${id} .version { font-size: 20px; }
#${id} .label { font-size: ${mobile?19:18}px; }
#${id} .date { font-size: ${mobile?17:16}px; fill: ${COLORS.muted}; }
</style>
<rect width="${width}" height="${height}" rx="8" fill="${COLORS.background}"/>
${content.join('\n')}
</svg>\n`;
}

export function buildTimeline(destination) {
  mkdirSync(destination,{recursive:true});
  return [false,true].map(mobile=>{
    const filename=join(destination,`timeline${mobile?'-mobile':''}.svg`);
    writeFileSync(filename,timelineSvg({mobile}));
    return filename;
  });
}
