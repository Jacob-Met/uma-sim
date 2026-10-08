import json,urllib.request,subprocess,hashlib,os,signal,time
from pathlib import Path
r=Path('/Users/me/uma-conditions-20261008-5f566b5ec8ef');binary=r/'target/debug/uma-sim-api';pid=47181
assert hashlib.sha256(binary.read_bytes()).hexdigest()=='283c46e2a698dd1e258ef726ffd3816536c2f1f8fec7240168a4b0063e16a69a'
identity=subprocess.run(['/bin/ps','-ww','-p',str(pid),'-o','comm='],capture_output=True,text=True,check=True).stdout.strip()
assert identity==str(binary),identity
def request(path,data=None):
 body=None if data is None else json.dumps(data).encode()
 req=urllib.request.Request('http://127.0.0.1:61097'+path,data=body,headers={} if data is None else {'Content-Type':'application/json'})
 with urllib.request.urlopen(req,timeout=10) as response:return json.load(response)
before=request('/v1/run/state')
assert before['state']['fans']==100123 and len(before['state']['statuses'])==46
choices=request('/v1/run/choices')
receipt={'purpose':'Diagnose the owned candidate receiver action timeout and terminate only its orphan API','owned_pid':pid,'binary':str(binary),'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'before':before,'choices_before':choices}
try:
 action=request('/v1/run/action',{'action':'race'})
 after=request('/v1/run/state');after_choices=request('/v1/run/choices')
 receipt.update({'race_action':action,'after':after,'choices_after':after_choices})
finally:
 os.kill(pid,signal.SIGTERM)
 for _ in range(30):
  if subprocess.run(['/bin/ps','-p',str(pid),'-o','pid='],capture_output=True,text=True).returncode!=0:break
  time.sleep(.05)
 receipt['orphan_exited']=subprocess.run(['/bin/ps','-p',str(pid),'-o','pid='],capture_output=True,text=True).returncode!=0
 out=r/'evidence/candidate-race-boundary-probe.json';out.write_text(json.dumps(receipt,indent=2)+'\n')
print(json.dumps({'receipt':str(out),'sha256':hashlib.sha256(out.read_bytes()).hexdigest(),'phase_before':before['state']['phase'],'choices_before':choices,'phase_after':receipt.get('after',{}).get('state',{}).get('phase'),'turn_after':receipt.get('after',{}).get('state',{}).get('turn'),'career_complete_after':receipt.get('after',{}).get('state',{}).get('careerComplete'),'choices_after':receipt.get('choices_after'),'orphan_exited':receipt['orphan_exited']},indent=2))
