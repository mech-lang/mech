import base64,json,os,sys,tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tests'))
from browser.harness.chrome import ChromeSession
out=ROOT/'audit/v04/evidence/preview-annotations'
out.mkdir(parents=True,exist_ok=True)
records=[]
fixture=(ROOT/'src/syntax/tests/fixtures/document/recovery/fenced-unclosed-matrix.mec').read_text()
with tempfile.TemporaryDirectory(prefix='mech-preview-annotations-') as profile:
 with ChromeSession(None,profile,out/'chrome.log',window_size=(1440,1150)) as browser:
  browser.navigate(f'http://127.0.0.1:{int(os.environ.get("MECH_EDITOR_TEST_PORT","8764"))}/audit/v04/site/streaming.html')
  browser.wait_for('window.mechDocumentReady','editor ready',timeout=120)
  def edit(source):
   browser.evaluate("(() => {const a=document.getElementById('document-source');a.value="+json.dumps(source)+";a.dispatchEvent(new Event('input',{bubbles:true}));})()")
   try:
    browser.wait_for("document.getElementById('document-preview-status').textContent.startsWith('Current source') && document.getElementById('document-colors').textContent.trim() === document.getElementById('document-source').value.trim()",'current parse',timeout=30)
   except Exception:
    print(browser.evaluate("({status:document.getElementById('document-status').textContent,previewStatus:document.getElementById('document-preview-status').textContent,source:document.getElementById('document-source').value,preview:document.getElementById('document-preview').innerHTML})"),flush=True)
    raise
  def state():
   return browser.evaluate("({state:document.getElementById('document-preview').dataset.state,callouts:[...document.querySelectorAll('.preview-callout')].map(n=>({text:n.textContent,label:n.getAttribute('aria-label')})),marks:[...document.querySelectorAll('.preview-source-error,.preview-source-point')].map(n=>({text:n.textContent,number:n.dataset.error})),arrows:[...document.querySelectorAll('.preview-connector')].map(n=>n.getAttribute('d')),consoleInPreview:!!document.querySelector('#document-preview .diagnostic-source,#document-preview .preview-error-source'),code:[...document.querySelectorAll('#document-preview pre code')].map(n=>n.textContent),reports:[...document.querySelectorAll('#document-errors .diagnostic-source')].map(n=>n.textContent),html:document.getElementById('document-preview').innerHTML})")
  def snapshot(name):
   image=browser.call('Page.captureScreenshot',{'format':'png','captureBeyondViewport':False})
   (out/name).write_bytes(base64.b64decode(image['data']))
  source='x := [1, +, 2]\ny := [3, *, 4]\n'
  edit(source)
  browser.wait_for('document.querySelectorAll(".preview-connector").length===2','connector drawing',timeout=10)
  observed=state()
  assert len(observed['callouts'])==2 and observed['state']=='error' and len(observed['arrows'])==2,observed
  assert observed['marks'] and not observed['consoleInPreview'],observed
  assert observed['code']==[source],observed
  assert len(observed['reports'])==2 and all('^' in text for text in observed['reports']),observed
  assert all('NaN' not in path for path in observed['arrows']),observed
  browser.evaluate('document.querySelectorAll(".preview-callout")[1].click()')
  selection=browser.evaluate("({start:document.getElementById('document-source').selectionStart,end:document.getElementById('document-source').selectionEnd})")
  assert source[selection['start']:selection['end']]=='*, 4',selection
  records.append({'case':'two errors annotate original code and numbered callouts navigate source','passed':True,'state':observed,'selection':selection})
  snapshot('desktop.png')
  for _ in range(2):
   browser.evaluate('window.compileMechDocument()')
   assert len(state()['callouts'])==2 and state()['code']==[source],state()
  records.append({'case':'repeated compilation restores source before annotation','passed':True})
  edit(fixture)
  browser.wait_for('document.querySelectorAll(".preview-connector").length===1','fenced arrow',timeout=10)
  observed=state()
  assert observed['marks'][0]['text']=='[' and 'Section One' in observed['html'],observed
  assert all('document:' not in n['text'] and '|' not in n['text'] for n in observed['callouts']),observed
  records.append({'case':'fenced missing delimiter points to bracket and retains later section','passed':True,'state':observed})
  snapshot('fenced.png')
  edit(fixture.replace('5\n','5]\n'))
  result=browser.evaluate('window.compileMechDocument()')
  assert not state()['callouts'] and not state()['marks'] and not state()['arrows'],state()
  assert result['result']['stages']['execution']=='completed',result
  records.append({'case':'repair removes annotations and runs program','passed':True})
  for source in ['x := [1, +, 2] -- **Context** for this expression.\nx\n','-- **Context** for this expression.\nx := [1, +, 2]\nx\n']:
   edit(source)
   browser.wait_for('document.querySelectorAll(".preview-connector").length===1','comment context connectors',timeout=10)
   assert state()['marks'][0]['text']=='+, 2' and not state()['consoleInPreview'],state()
   records.append({'case':'code ranges retain their anchors across rendered rich comments','passed':True,'state':state()})
  edit('answer := 1 +')
  browser.wait_for('document.querySelectorAll(".preview-connector").length===1','EOF arrow',timeout=10)
  assert any(n['text']=='' for n in state()['marks']),state()
  records.append({'case':'insertion point at EOF is marked and connected','passed':True,'state':state()})
  for source in ['```mech{output: }\nanswer := 1\n```\n', 'Calculation\n===========\n\nanswer := [1 @ [2\n\n1. Next\n---\n\nA later paragraph.\n']:
   edit(source)
   browser.wait_for('document.querySelectorAll(".preview-connector").length>0','recovered source connectors',timeout=10)
   assert state()['marks'] and not state()['consoleInPreview'],state()
   records.append({'case':'recovered fence header or title metadata retains source marks','passed':True,'state':state()})
  source='Café 👩‍💻\n=========\n\n```mech\nanswer := [1 2\n```\n'
  edit(source)
  browser.wait_for('document.querySelectorAll(".preview-connector").length===1','Unicode arrow',timeout=10)
  assert state()['marks'][0]['text']=='[',state()
  browser.evaluate('document.querySelector(".preview-callout").click()')
  selected=browser.evaluate("document.getElementById('document-source').value.slice(document.getElementById('document-source').selectionStart,document.getElementById('document-source').selectionEnd)")
  assert selected=='[',selected
  records.append({'case':'Unicode source offsets preserve exact marked range','passed':True,'state':state()})
  edit('answer := 10⟨u8:1..10⟩\nanswer\n')
  browser.evaluate('window.compileMechDocument()')
  browser.wait_for('document.querySelectorAll(".preview-connector").length===1','type error arrow',timeout=10)
  assert state()['marks'] and state()['reports'] and not state()['consoleInPreview'],state()
  records.append({'case':'semantic diagnostic annotates rendered code after compilation','passed':True,'state':state()})
  edit('Calculation\n===========\n\nA paragraph.\n')
  browser.evaluate('window.compileMechDocument()')
  assert not state()['callouts'] and not state()['state'],state()
  records.append({'case':'information diagnostic preserves clean rendered document','passed':True})
  edit('answer := [1 @ [2\n\n1. Next\n---\n\nA later paragraph.\n')
  browser.wait_for('document.querySelectorAll(".preview-connector").length>0','deep recovery arrow',timeout=10)
  assert state()['marks'] and 'A later paragraph.' in state()['html'],state()
  records.append({'case':'deep unfenced recovery attaches callouts to retained source','passed':True,'state':state()})
  edit('x := [1, +, 2]\ny := [3, *, 4]\n')
  browser.wait_for('document.querySelectorAll(".preview-connector").length===2','resize base',timeout=10)
  old=state()['arrows']
  browser.call('Emulation.setDeviceMetricsOverride',{'width':1000,'height':1150,'deviceScaleFactor':1,'mobile':False})
  browser.wait_for('JSON.stringify([...document.querySelectorAll(".preview-connector")].map(n=>n.getAttribute("d")))!=='+json.dumps(json.dumps(old,separators=(',',':'))),'resize connectors',timeout=10)
  layout=browser.evaluate("(() => {const c=document.querySelector('.preview-annotation-content').getBoundingClientRect(),n=document.querySelector('.preview-annotation-notes').getBoundingClientRect();return {codeBottom:c.bottom,notesTop:n.top,width:document.documentElement.scrollWidth,viewport:innerWidth};})()")
  assert layout['notesTop']>=layout['codeBottom'] and layout['width']<=layout['viewport'],layout
  records.append({'case':'narrow preview stacks callouts and redraws arrows','passed':True,'layout':layout})
  browser.call('Emulation.setDeviceMetricsOverride',{'width':700,'height':1150,'deviceScaleFactor':1,'mobile':False})
  browser.evaluate('document.querySelector(".document-rendered").scrollIntoView({block:"start"})')
  layout=browser.evaluate("({width:document.documentElement.scrollWidth,viewport:innerWidth,notes:[...document.querySelectorAll('.preview-callout')].every(n=>n.getBoundingClientRect().right<=innerWidth)})")
  assert layout['width']<=layout['viewport'] and layout['notes'],layout
  snapshot('mobile.png')
  records.append({'case':'mobile document preview retains code annotations within pane','passed':True,'layout':layout})
  browser.call('Emulation.setDeviceMetricsOverride',{'width':390,'height':1150,'deviceScaleFactor':1,'mobile':False})
  browser.wait_for('document.querySelector(".preview-annotation-notes").getBoundingClientRect().top >= document.querySelector(".preview-annotation-content").getBoundingClientRect().bottom','phone callouts',timeout=10)
  layout=browser.evaluate("({width:document.documentElement.scrollWidth,viewport:innerWidth,notes:[...document.querySelectorAll('.preview-callout')].every(n=>n.getBoundingClientRect().right<=innerWidth)})")
  assert layout['width']<=layout['viewport'] and layout['notes'],layout
  snapshot('phone.png')
  records.append({'case':'phone callouts stack with connectors routed outside the source','passed':True,'layout':layout})

  browser.call('Emulation.setDeviceMetricsOverride',{'width':1440,'height':1150,'deviceScaleFactor':1,'mobile':False})
  browser.evaluate('window.scrollTo(0,0)')
  edit('x := [1, +, 2]\ny := [3, *, 4]\n')
  browser.wait_for('document.querySelectorAll(".preview-connector").length===2','desktop restored',timeout=10)
  records.append({'case':'loading the same source restores syntax colors and annotations','passed':True,'state':state()})
  browser.call('Emulation.setDeviceMetricsOverride',{'width':1440,'height':1150,'deviceScaleFactor':1,'mobile':False,'scale':2})
  assert not state()['consoleInPreview'],state()
  records.append({'case':'console source reports remain confined to diagnostics','passed':True})
report={'status':'passed','cases':len(records),'records':records}
(out/'browser-results.json').write_text(json.dumps(report,indent=2,ensure_ascii=False)+'\n')
print(json.dumps({'status':'passed','cases':len(records)}))
