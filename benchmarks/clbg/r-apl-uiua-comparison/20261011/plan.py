"""Scheduling-only short probes in separate reports; never chart eligible."""
import argparse,json
from prepare import G,LANGUAGES
from run import PinGuard,run_pilots,install_interrupt_handlers

def main():
 install_interrupt_handlers()
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--languages',default='r');parser.add_argument('--timeout',type=int,default=14400);parser.add_argument('--max-rss-mib',type=int,default=10240);args=parser.parse_args()
 for language in args.languages.split(','):
  assert language in LANGUAGES
  audit=json.loads((G/('source-audit-'+language+'.json')).read_text())
  path=G/'results'/('runtime-planning-'+language+'.json')
  report=json.loads(path.read_text()) if path.exists() else {'language':language,'planning_only':True,'never_chart_eligible':True,'programs':{r['key']:r for r in audit['programs']},'runtime_planning_pilots':[]}
  report['programs']={r['key']:r for r in audit['programs']}
  run_pilots({'audit':audit,'report':report,'path':path,'guard':PinGuard(audit['pins'])},args)
if __name__=='__main__':main()
