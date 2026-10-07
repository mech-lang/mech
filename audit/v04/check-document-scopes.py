"""Exercise document scope execution and canonical in-document outputs in Chrome."""
import base64,json,os,sys,tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tests'))
from browser.harness.chrome import ChromeSession
out=ROOT/'audit/v04/evidence/document-scopes'
out.mkdir(parents=True,exist_ok=True)
records=[]
source='Calculation\r\n===========\r\n\r\nThe result is published by the final expression.\r\n\r\n```mech:foo\r\nanswer := 123\r\n```\r\n\r\n```mech:bar\r\nanswer := 456\r\n```\r\n\r\n1. Section One\r\n------------------------------\r\n\r\nThis is the first section.\r\n'
with tempfile.TemporaryDirectory(prefix='mech-document-scopes-') as profile:
 with ChromeSession(None,profile,out/'chrome.log',window_size=(1440,1250)) as browser:
  browser.navigate(f'http://127.0.0.1:{int(os.environ.get("MECH_EDITOR_TEST_PORT","8764"))}/audit/v04/site/streaming.html')
  browser.wait_for('window.mechDocumentReady','editor ready',timeout=120)
  def edit(text):
   browser.evaluate("(() => {const a=document.getElementById('document-source');a.value="+json.dumps(text)+";a.dispatchEvent(new Event('input',{bubbles:true}));a.focus();})()")
   browser.wait_for("document.getElementById('document-preview-status').textContent.startsWith('Current source') && document.getElementById('document-colors').textContent.trim() === document.getElementById('document-source').value.trim()",'current syntax preview',timeout=30)
  def run(text):
   edit(text)
   return browser.evaluate('window.compileMechDocument()')['result']
  def state():
   return browser.evaluate("({fences:[...document.querySelectorAll('#document-preview figure.mech-code-block')].map(n=>({scope:n.dataset.mechScope,code:n.querySelector('pre code')?.textContent,kind:n.querySelector('.mech-output-kind')?.textContent,output:n.querySelector('figcaption.mech-output')?.textContent})),preview:document.getElementById('document-preview').textContent,outputs:document.getElementById('document-output').textContent,errors:document.getElementById('document-error-count').textContent,callouts:[...document.querySelectorAll('.preview-callout')].map(n=>n.textContent),marks:[...document.querySelectorAll('.preview-source-error')].map(n=>n.textContent),state:document.getElementById('document-preview').dataset.state})")
  def record(name,result=None):
   records.append({'case':name,'passed':True,'state':state(),**({'result':result} if result is not None else {})})
  def screenshot(name):
   image=browser.call('Page.captureScreenshot',{'format':'png','captureBeyondViewport':False})
   (out/name).write_bytes(base64.b64decode(image['data']))
  edit(source)
  browser.call('Input.dispatchKeyEvent',{'type':'keyDown','key':'Enter','code':'Enter','windowsVirtualKeyCode':13,'nativeVirtualKeyCode':13,'modifiers':2})
  browser.call('Input.dispatchKeyEvent',{'type':'keyUp','key':'Enter','code':'Enter','windowsVirtualKeyCode':13,'nativeVirtualKeyCode':13,'modifiers':2})
  browser.wait_for('window.mechDocumentLastRun?.runNumber === 1','Ctrl+Enter document execution',timeout=60)
  result=browser.evaluate('window.mechDocumentLastRun.result')
  assert result['diagnostics']==[] and result['stages']['execution']=='completed',result
  assert [(v['scope'],v['text']) for v in result['values']]==[('foo','123'),('bar','456')],result
  observed=state()
  assert [(v['scope'],v['kind'],v['output']) for v in observed['fences']]==[('foo','f64','f64123'),('bar','f64','f64456')],observed
  assert 'This is the first section.' in observed['preview'] and observed['errors']=='0',observed
  record('user document executes independent named scopes and displays typed fence outputs',result)
  screenshot('desktop.png')
  result=browser.evaluate('window.compileMechDocument()')['result']
  assert [v['text'] for v in result['values']]==['123','456'] and len(state()['fences'])==2,result
  record('repeat compile preserves two output mounts',result)
  edit(source.replace('123','789'))
  assert all('output' not in v for v in state()['fences']),state()
  result=browser.evaluate('window.compileMechDocument()')['result']
  assert [v['text'] for v in result['values']]==['789','456'],result
  record('edits clear old fence outputs until compilation',result)
  repeated='answer := 9\n\n```mech:foo\n~answer := 10\nanswer += 1\nanswer\n```\n\n```mech:bar\nanswer := 20\n```\n\n```mech:foo\nanswer += 2\nanswer\n```\n'
  result=run(repeated)
  assert [v['text'] for v in result['values']]==['9','11','20','13'],result
  assert [v['output'] for v in state()['fences']]==['f6411','f6420','f6413'],state()
  assert browser.evaluate("document.querySelectorAll('#document-preview .mech-program-output').length")==0
  assert result['values'][0]['text']=='9' and 'root' in state()['outputs']
  record('root bindings are isolated and repeated scope fences share state in source order',result)
  result=browser.evaluate('window.compileMechDocument()')['result']
  assert [v['text'] for v in result['values']]==['9','11','20','13'],result
  record('each run resets named state',result)
  table='|x<f64> y<f64>| 1 2 | 3 4 |\n'
  result=run(table)
  observed=browser.evaluate("({code:document.querySelector('#document-preview pre code').textContent,previewTables:document.querySelectorAll('#document-preview table').length,outputTables:document.querySelectorAll('#document-output table').length,cells:[...document.querySelectorAll('#document-output table td')].map(n=>n.textContent)})")
  assert observed['code']==table and observed['previewTables']==0 and observed['outputTables']==1 and observed['cells']==['1','2','3','4'],observed
  record('unfenced table retains source presentation and publishes to the output panel',result)
  screenshot('unfenced-table.png')
  result=run('```mech:example\n'+table+'```\n\n9\n')
  observed=browser.evaluate("({fenceTables:document.querySelectorAll('#document-preview figcaption.mech-output table').length,previewTables:document.querySelectorAll('#document-preview table').length,aggregate:document.querySelectorAll('#document-preview .mech-program-output').length,kind:document.querySelector('#document-preview .mech-output-kind').textContent,cells:[...document.querySelectorAll('#document-preview figcaption table td')].map(n=>n.textContent)})")
  assert observed['fenceTables']==1 and observed['previewTables']==1 and observed['aggregate']==0 and observed['kind']=='table' and observed['cells']==['1','2','3','4'],observed
  record('fenced table uses its dedicated output channel',result)
  screenshot('fenced-table.png')
  result=run('data := '+table+'\nThe data is {data}.\n\n42\n')
  observed=browser.evaluate("({previewTables:document.querySelectorAll('#document-preview table').length,aggregate:document.querySelectorAll('#document-preview .mech-program-output').length,cells:[...document.querySelectorAll('#document-preview table td')].map(n=>n.textContent)})")
  assert observed['previewTables']==1 and observed['aggregate']==0 and observed['cells']==['1','2','3','4'],observed
  assert any(value['text']=='42' for value in result['values']),result
  record('explicit inline table reference owns its document value location',result)
  result=run('```mech:foo\nanswer := math/sub(right: 3, left: 10)\n```\n')
  assert result['diagnostics']==[] and result['values'][0]['text']=='7',result
  record('named scopes use the standard library catalog',result)
  result=run('answer := 42\n\nThe answer is {answer}.\n\n```mech:foo\nanswer := 123\n```\n')
  assert 'The answer is 42.' in state()['preview'] and state()['fences'][0]['output']=='f64123',state()
  record('inline expressions render root values alongside named outputs',result)
  result=run('```mech:matrix\nanswer := [1 2;\n3 4]\n```\n\n```mech:record\nanswer := {x: 3, y: 4}\n```\n')
  observed=browser.evaluate("({matrix:[...document.querySelectorAll('#document-preview figcaption .mech-matrix td')].map(n=>n.textContent),record:[...document.querySelectorAll('#document-preview figcaption .mech-record tr')].map(n=>n.textContent),kinds:[...document.querySelectorAll('#document-preview .mech-output-kind')].map(n=>n.textContent)})")
  assert observed['matrix']==['1','2','3','4'] and observed['record']==['x3','y4'],observed
  assert len(observed['kinds'])==2 and all('Bits(' not in v for v in observed['kinds']),observed
  record('matrix and record fence values use canonical HTML formatting',result)
  result=run('```mech:hidden\nanswer := 1\n```\n\n```mech:disabled\nanswer := 999\n```\n\n```rust\nthis is source text\n```\n\n```mech:foo{output: false}\nanswer := 2\n```\n\n```mech:bar\nanswer := 3\n```\n')
  assert result['diagnostics']==[] and [(v['scope'],v['text']) for v in result['values']]==[('bar','3')],result
  assert sum('output' in v for v in state()['fences'])==1,state()
  record('disabled inert hidden and suppressed-output fences retain their semantics',result)
  invalid='```mech:foo\nanswer := 10⟨u8:1..10⟩\n```\n\n```mech:bar\nanswer := 456\n```\n'
  result=run(invalid)
  browser.wait_for('document.querySelectorAll(".preview-connector").length===1','semantic error annotation',timeout=10)
  assert result['diagnostics'][0]['code']=='source-semantics/integer-interval-violation' and state()['state']=='error',result
  assert [(v['scope'],v['text']) for v in result['values']]==[('bar','456')],result
  assert state()['marks'] and len(state()['callouts'])==1,state()
  browser.evaluate('document.querySelector(".preview-callout").click()')
  selected=browser.evaluate("document.getElementById('document-source').value.slice(document.getElementById('document-source').selectionStart,document.getElementById('document-source').selectionEnd)")
  assert '10' in selected,selected
  record('named semantic error retains precise source navigation and independent scope result',result)
  result=run('```mech:foo\nanswer := [1 2\n```\n\n1. Next\n---\n\nA later paragraph.\n')
  assert result['scopes']==[] and state()['callouts'] and 'A later paragraph.' in state()['preview'],result
  record('syntax recovery retains diagnostics and later document sections',result)
  result=run('Calculation\n===========\n\nA paragraph.\n')
  assert result['diagnostics'][0]['severity']=='info' and state()['state']=='',result
  record('prose-only document retains information severity',result)
  result=run('```mech:foo\nanswer := 123\n```\n\n~∘~⸢```mech:foo\nanswer := 456\n```\n⸥\n')
  assert [v['text'] for v in result['values']]==['123','456'] and len(state()['fences'])==2,result
  assert result['scopes'][0]['owner']!=result['scopes'][1]['owner'],result
  record('Mika-local named scopes retain their owners',result)
  browser.call('Emulation.setDeviceMetricsOverride',{'width':390,'height':1200,'deviceScaleFactor':1,'mobile':False})
  result=run(source)
  browser.evaluate('document.querySelector(".document-rendered").scrollIntoView({block:"start"})')
  observed=browser.evaluate("({width:document.documentElement.scrollWidth,viewport:innerWidth,fences:[...document.querySelectorAll('#document-preview figcaption')].map(n=>n.getBoundingClientRect().right),preview:document.querySelector('.document-rendered').getBoundingClientRect().bottom,output:document.querySelector('.document-results').getBoundingClientRect().top})")
  assert observed['width']<=observed['viewport'] and all(right<=observed['viewport'] for right in observed['fences']) and observed['output']>=observed['preview'],observed
  record('phone layout preserves fence output placement and program output below editor',result)
  screenshot('phone.png')
report={'status':'passed','cases':len(records),'records':records}
(out/'browser-results.json').write_text(json.dumps(report,indent=2,ensure_ascii=False)+'\n')
print(json.dumps({'status':'passed','cases':len(records)}))
