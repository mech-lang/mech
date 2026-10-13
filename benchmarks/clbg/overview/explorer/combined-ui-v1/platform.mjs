// Candidate only. The payload producer qualifies preserved runtime evidence.
// Names, source extensions and upstream submission IDs are never evidence.
export const platformDimension={key:'platform',label:'Platform',
  note:'The runtime or backend used for these measurements.',options:[
    {key:'jvm',label:'JVM'}, {key:'beam',label:'BEAM'}, {key:'dotnet',label:'.NET'},
    {key:'native',label:'Native'}, {key:'javascript',label:'JavaScript'},
    {key:'unknown',label:'Unclassified'},
  ]};

const backendPlatforms={
  'openjdk-jit':'jvm', 'jvm-jit':'jvm', 'jvm-interpreter':'jvm',
  'beam-jit':'beam', 'beam-interpreter':'beam',
  'dotnet-jit':'dotnet', 'dotnet-native-aot':'dotnet',
  'native-aot':'native', 'graalvm-native-aot':'native',
  'native-jit':'native', 'native-interpreter':'native',
  'native-runtime':'native',
  'javascript-jit':'javascript', 'javascript-interpreter':'javascript',
  'javascript-runtime':'javascript',
};
const executionModes={
  'openjdk-jit':'jit', 'jvm-jit':'jit', 'jvm-interpreter':'interpreted',
  'beam-jit':'jit', 'beam-interpreter':'interpreted',
  'dotnet-jit':'jit', 'dotnet-native-aot':'aot',
  'native-aot':'aot', 'graalvm-native-aot':'aot',
  'native-jit':'jit', 'native-interpreter':'interpreted',
  'native-runtime':'unknown',
  'javascript-jit':'jit', 'javascript-interpreter':'interpreted',
  'javascript-runtime':'unknown',
};
const sha=value=>typeof value==='string'&&/^[0-9a-f]{64}$/.test(value);
const sorted=values=>[...values].sort();
const same=(a,b)=>JSON.stringify(sorted(a))===JSON.stringify(sorted(b));

function qualifiedBackend(data,program) {
  const record=program?.measured_backend;
  const evidence=record?.evidence;
  if(record?.verified!==true||!backendPlatforms[record.key]||
      typeof record.label!=='string'||!record.label.trim()||
      record.execution!==executionModes[record.key]||evidence?.verified!==true)return null;
  if(Object.hasOwn(record,'confidence')) {
    const confidence=record.confidence;
    const keys=['historical_platform_verified','historical_execution_verified',
      'historical_executable_bytes_verified'];
    if(!confidence||typeof confidence!=='object'||Array.isArray(confidence)||
        !same(Object.keys(confidence),keys)||keys.some(key=>typeof confidence[key]!=='boolean')||
        confidence.historical_platform_verified!==true||
        confidence.historical_execution_verified!==(record.execution!=='unknown'))return null;
    // Missing historical executable bytes do not erase separately established
    // platform/mode facts. The producer retains that distinct uncertainty.
  }
  if(typeof evidence.report!=='string'||evidence.report!==program.report||
      !sha(evidence.report_sha256)||data.provenance?.[evidence.report]!==evidence.report_sha256)return null;
  if(!Array.isArray(evidence.command)||!evidence.command.length||
      evidence.command.some(part=>typeof part!=='string')||!evidence.command[0])return null;
  // Receipt is preserved evidence from a pinned report/build, not a current
  // executable version guessed to have produced a historical sample.
  const runtime=evidence.runtime_receipt;
  if(runtime?.verified!==true||typeof runtime.report!=='string'||
      !sha(runtime.report_sha256)||data.provenance?.[runtime.report]!==runtime.report_sha256||
      typeof runtime.field!=='string'||!runtime.field||
      typeof runtime.value!=='string'||!runtime.value)return null;
  const sources=(program.files??[]).map(key=>data.files?.[key]?.sha256);
  if(!sources.length||sources.some(value=>!sha(value))||
      !Array.isArray(evidence.source_sha256)||!same(sources,evidence.source_sha256))return null;
  if(program.execution_mode&&program.execution_mode!==record.execution)return null;
  return record;
}

export function measuredPlatform(data,language) {
  const programs=Object.values(data.programs??{}).filter(program=>
    program.language===language.key&&Number.isFinite(program.seconds)&&program.seconds>0);
  const authoritative=programs.some(program=>Object.hasOwn(program,'measured_backend'));
  if(!programs.length)return {platform:new Set(['unknown']),execution:null,execution_authoritative:false};
  const records=programs.map(program=>qualifiedBackend(data,program));
  if(records.some(record=>!record))return {platform:new Set(['unknown']),execution:null,execution_authoritative:authoritative};
  const platforms=new Set(records.map(record=>backendPlatforms[record.key]));
  const execution=new Set(records.map(record=>record.execution));
  // Platform and execution are independent facts. A proved shared platform
  // survives mixed/unknown modes; a mixed platform aggregate stays unknown.
  if(platforms.size!==1)return {platform:new Set(['unknown']),execution:null,execution_authoritative:true};
  return {platform:platforms,execution:execution.size===1?[...execution][0]:'unknown',execution_authoritative:true};
}
