import subprocess
commands=[['python3','scripts/'+name] for name in ['check-production-resident-routing.py','check-source-catalog-entrypoints.py','check-native-host-catalog.py','check-native-build-analysis-layout.py']]
commands += [['/private/tmp/mech-assertion-ci-python/bin/python','-c','import sys;sys.path.insert(0,"scripts");from ci_test_inventory import inventory_gaps;g=inventory_gaps();print(g);assert not g'],['cargo','fmt','--all','--check'],['git','diff','--check']]
for command in commands:subprocess.run(command,check=True)
