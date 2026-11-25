#!/usr/bin/env python3
"""HARA-410 local evidence capture. No repository or service mutation commands."""
import datetime
import hashlib
import json
import pathlib
import subprocess
import sys
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parents[4]
OUT = pathlib.Path(__file__).resolve().parent

def stamp():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()

def save(name, obj):
    data = json.dumps(obj, ensure_ascii=False, indent=2).encode()
    dest = OUT / (name + '.json')
    dest.write_bytes(data)
    print(json.dumps({'file': str(dest), 'sha256': hashlib.sha256(data).hexdigest(),
                      'exit': obj.get('exit'), 'bytes': len(data)}, ensure_ascii=False))

def record(name, args):
    started = stamp()
    try:
        p = subprocess.run(args, cwd=ROOT, stdout=subprocess.PIPE,
                           stderr=subprocess.STDOUT, timeout=1800)
        result = {'exit': p.returncode, 'output': p.stdout.decode(errors='replace')}
    except subprocess.TimeoutExpired as e:
        result = {'exit': 124, 'output': (e.stdout or b'').decode(errors='replace'),
                  'status': 'NOT-RUN-completion-timeout'}
    save(name, {'source_agent': 'Codex GPT-6.1 Sol', 'source_run': 'HARA-410-20261120',
                'started': started, 'timestamp': stamp(), 'cwd': str(ROOT),
                'argv': args, **result})

def api():
    base = 'http://192.168.0.252:3000/api/v1/repos/openclaw/sqlrustgo/'
    for key, path in [('issues', 'issues?state=open&type=issues'),
                      ('pulls', 'pulls?state=open'), ('releases', 'releases'), ('tags', 'tags')]:
        rows, pages = [], []
        for page in range(1, 101):
            url = base + path + ('&' if '?' in path else '?') + f'limit=50&page={page}'
            with urllib.request.urlopen(url, timeout=20) as resp:
                data = json.load(resp)
                pages.append({'url': url, 'status': resp.status, 'count': len(data),
                              'headers': dict(resp.headers)})
            rows.extend(data)
            if len(data) < 50:
                break
        save('gitea-' + key, {'source_agent': 'Codex GPT-6.1 Sol',
             'source_run': 'HARA-410-20261120', 'timestamp': stamp(),
             'pages': pages, 'count': len(rows), 'data': rows})

def mcp_list(tool=None, arguments=None):
    messages = [
        {'jsonrpc':'2.0','id':1,'method':'initialize','params':{
            'protocolVersion':'2025-11-25','capabilities':{},
            'clientInfo':{'name':'hara410-audit','version':'1'}}},
        {'jsonrpc':'2.0','method':'notifications/initialized'},
        {'jsonrpc':'2.0','id':2,'method':'tools/list','params':{}}]
    if tool:
        messages[2] = {'jsonrpc':'2.0','id':2,'method':'tools/call',
                       'params':{'name':tool,'arguments':arguments}}
    p = subprocess.Popen(['aoci','--repo',str(ROOT),'mcp'],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        text=True)
    responses = []
    for msg in messages:
        p.stdin.write(json.dumps(msg)+'\n')
        p.stdin.flush()
        if 'id' in msg:
            while True:
                line = p.stdout.readline()
                if not line:
                    raise RuntimeError('AOCI closed unexpectedly')
                response = json.loads(line)
                responses.append(response)
                if response.get('id') == msg['id']:
                    break
    p.terminate()
    p.communicate(timeout=5)
    save(tool or 'aoci-tools', {'timestamp': stamp(), 'responses': responses})

if __name__ == '__main__':
    if sys.argv[1] == 'api':
        api()
    elif sys.argv[1] == 'mcp-list':
        mcp_list()
    elif sys.argv[1] == 'mcp-call':
        mcp_list(sys.argv[2], json.loads(sys.argv[3]))
    elif sys.argv[1] == 'show':
        data = json.loads((OUT / (sys.argv[2] + '.json')).read_text())
        print(data.get('output', json.dumps(data, ensure_ascii=False)))
    else:
        record(sys.argv[1], sys.argv[2:])
