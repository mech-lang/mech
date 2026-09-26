// Browser presentation and synthetic sensor source, not an EKF implementation.
// All estimated state and covariance are supplied by compiled Mech.
const NS = 'http://www.w3.org/2000/svg';
const el = (tag, attrs = {}) => {
  const node = document.createElementNS(NS, tag);
  for (const [key, value] of Object.entries(attrs)) node.setAttribute(key, value);
  return node;
};
export class RobotScene {
  constructor(svg) {
    this.svg = svg;
    this.reset();
  }
  reset() {
    this.truth = [55, 25, 0.4];
    this.turn = 0;
    this.truthTrail = [];
    this.estimateTrail = [];
    this.draw([55, 25, 0.4], [100,0,0,0,100,0,0,0,0.15]);
  }
  observation(velocity, omega, noise, instances, invalid = false) {
    const [x,y,heading] = this.truth;
    const next = [x + velocity * 0.1 * Math.cos(heading), y + velocity * 0.1 * Math.sin(heading), heading + omega * 0.1];
    const bearing = Math.atan2(12-next[1], 140-next[0]) - next[2];
    const readings = Float32Array.from({length:instances}, (_,i) => bearing + noise * Math.sin((this.turn+1)*1.73 + i*0.37));
    if (invalid) readings[instances - 1] = NaN;
    return {next, inputs:{bearing:readings, v:[velocity], w:[omega]}};
  }
  accepted(next, state, covariance) {
    this.truth = next;
    this.turn++;
    this.truthTrail.push(next.slice(0,2));
    this.estimateTrail.push(Array.from(state).slice(0,2));
    if (this.truthTrail.length > 350) {this.truthTrail.shift();this.estimateTrail.shift();}
    this.draw(state,covariance);
  }
  draw(state,covariance) {
    this.svg.replaceChildren();
    const grid=el('g',{stroke:'#243139','stroke-width':'.25'});
    for(let x=0;x<=200;x+=10) grid.append(el('path',{d:'M'+x+' 0V130'}));
    for(let y=0;y<=130;y+=10) grid.append(el('path',{d:'M0 '+y+'H200'}));
    this.svg.append(grid);
    const paths = [[this.truthTrail,'#acb9c1'],[this.estimateTrail,'#f6c04e']];
    for(const [points,color] of paths) this.svg.append(el('polyline',{points:points.map(p=>p[0]+','+(130-p[1])).join(' '),fill:'none',stroke:color,'stroke-width':'.6'}));
    const pxx=covariance[0], pxy=(covariance[1]+covariance[3])/2, pyy=covariance[4];
    const radius=Math.hypot(pxx-pyy,2*pxy);
    const major=Math.sqrt(Math.max(0,(pxx+pyy+radius)/2))*2;
    const minor=Math.sqrt(Math.max(0,(pxx+pyy-radius)/2))*2;
    const degrees=-Math.atan2(2*pxy,pxx-pyy)*90/Math.PI;
    if ([major,minor,state[0],state[1]].every(Number.isFinite)) this.svg.append(el('ellipse',{cx:state[0],cy:130-state[1],rx:major,ry:minor,transform:'rotate('+degrees+' '+state[0]+' '+(130-state[1])+')',fill:'#c08cc755',stroke:'#c08cc7','stroke-width':'.55'}));
    this.svg.append(el('circle',{cx:140,cy:118,r:1.8,fill:'#62b4a4'}));
    const label=el('text',{x:143,y:119,fill:'#91cabc','font-size':3});label.textContent='landmark';this.svg.append(label);
    for(const [pose,color,filled] of [[this.truth,'#acb9c1',false],[state,'#f6c04e',true]]) {
      const g=el('g',{transform:'translate('+pose[0]+' '+(130-pose[1])+') rotate('+(-pose[2]*180/Math.PI)+')'});
      g.append(el('circle',{r:1.7,fill:filled?color:'#10171a',stroke:color,'stroke-width':'.45'}),el('path',{d:'M0 0L3.1 0',stroke:color,'stroke-width':'.7'}));
      this.svg.append(g);
    }
    const scale=el('text',{x:5,y:125,fill:'#a9bcc5','font-size':3});scale.textContent='World coordinates · grid spacing 10';this.svg.append(scale);
  }
}
