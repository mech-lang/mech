import lldb,collections,json,pathlib,time
counts=collections.Counter()
started=time.monotonic()
def hit(frame,bp_loc,internal_dict):
    site=frame.FindRegister('x1').GetValueAsUnsigned()
    counts[site]+=1
    return False
def dump():
    pathlib.Path('/private/tmp/mech-helper-costs-round16-20261029/site-counts.json').write_text(json.dumps({'diagnostic_only':True,'register':'aarch64 x1 (CallCacheLane::invoke site argument)','counts':dict(counts),'elapsed_debugger_seconds':time.monotonic()-started},indent=2)+'\n')
def __lldb_init_module(debugger,internal_dict):
    target=debugger.GetSelectedTarget()
    bp=target.BreakpointCreateByName('_RNvMNtNtNtNtCs29QkOtpKx6e_8mech_gpu15native_artifact7program7runtime7sessionNtB2_13CallCacheLane6invoke')
    print('SITE_TRACE_LOCATIONS',bp.GetNumLocations())
    assert bp.GetNumLocations()==1
    bp.SetScriptCallbackFunction('site_probe.hit')
