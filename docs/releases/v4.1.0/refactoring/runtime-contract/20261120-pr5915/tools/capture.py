#!/usr/bin/env python3
"""Architecture evidence capture only; no worker or scheduler calls."""
import datetime
import hashlib
import json
import pathlib
import subprocess
import sys
import urllib.request

PACKET = pathlib.Path(__file__).resolve().parents[1]
OUT = PACKET / 'evidence'
OUT.mkdir(exist_ok=True)

def stamp():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()

def save(name, obj):
    data = json.dumps(obj, ensure_ascii=False, indent=2).encode() + b'\n'
    p = OUT / (name + '.json')
    p.write_bytes(data)
    print(json.dumps({'file':str(p),'sha256':hashlib.sha256(data).hexdigest(),
                      'exit_code':obj.get('exit_code')},ensure_ascii=False))

meta = {'source_agent':'Codex GPT-6.1 Sol','source_run':'RUNTIME-5915-20261120',
        'conflict_resolution':'Source-bound recommendation; Owner approval pending; no core changes or workers'}
if sys.argv[1] == 'record':
    name,cwd,args=sys.argv[2],sys.argv[3],sys.argv[4:]
    started=stamp()
    try:
        p=subprocess.run(args,cwd=cwd,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=900)
        output=p.stdout;code=p.returncode
    except subprocess.TimeoutExpired as e:
        output=e.stdout or b'';code=124
    log=OUT/(name+'.log');log.write_bytes(output)
    save(name,{**meta,'started_at':started,'timestamp':stamp(),'cwd':cwd,'argv':args,
               'exit_code':code,'raw_log':log.name,'evidence_hash':'sha256:'+hashlib.sha256(output).hexdigest()})
elif sys.argv[1] == 'api':
    name,path=sys.argv[2:4]
    url='http://192.168.0.252:3000/api/v1/'+path
    with urllib.request.urlopen(url,timeout=20) as r:
        save(name,{**meta,'timestamp':stamp(),'url':url,'http_status':r.status,'data':json.load(r)})
