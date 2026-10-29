#!/usr/bin/env python3
import hashlib,json,os,subprocess,sys
from pathlib import Path
base=Path(__file__).resolve().parent
binary=base/'bin/infigraph-mcp'
expected='1a688148e32a71876adaa1c9ad393240b0ed9b1331ea3fa9ec432d2495e6073c'
assert hashlib.sha256(binary.read_bytes()).hexdigest()==expected
allowed={'diagnose','search_symbols','get_symbols_in_file','get_code_snippet','symbol_context','trace_callers','trace_callees','transitive_impact','get_file_deps','get_stats'}
project=Path(sys.argv[1]).resolve(); tool=sys.argv[2]; args=json.loads(sys.argv[3]) if len(sys.argv)>3 else {}
assert project in {(base/'smoke/repo').resolve(), *((base/'lanes'/n/'repo').resolve() for n in ('ice-council','nightwire','giglab'))}
assert tool in allowed and isinstance(args,dict) and 'path' not in args
args['path']=str(project)
home=base/'pilot-home';home.mkdir(exist_ok=True)
env={'HOME':str(home),'INFIGRAPH_REGISTRY_HOME':str(home),'INFIGRAPH_MCP_PROFILE':'read-only','INFIGRAPH_NO_WATCH':'1','RAYON_NUM_THREADS':'2','PATH':'/usr/bin:/bin','LC_ALL':'C','HF_HUB_OFFLINE':'1'}
requests=[{'jsonrpc':'2.0','id':1,'method':'initialize','params':{'protocolVersion':'2024-11-05','capabilities':{},'clientInfo':{'name':'pilot','version':'1'}}},{'jsonrpc':'2.0','id':2,'method':'tools/list'},{'jsonrpc':'2.0','id':3,'method':'tools/call','params':{'name':tool,'arguments':args}}]
p=subprocess.Popen([str(binary),'--mcp'],cwd=project,env=env,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,start_new_session=True)
try:
 executable=Path('/proc',str(p.pid),'exe').resolve()
 assert executable==binary.resolve()
 out,err=p.communicate(''.join(json.dumps(x)+'\n' for x in requests),timeout=60)
except BaseException:
 p.kill();p.wait();raise
if p.returncode: raise SystemExit('candidate process failed: '+str(p.returncode))
r={x['id']:x for x in map(json.loads,out.splitlines()) if 'id' in x}
assert set(t['name'] for t in r[2]['result']['tools'])==allowed
print(json.dumps(r[3],separators=(',',':')))
