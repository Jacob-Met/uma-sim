"""Independent, bounded CLI receiving and full static output reconciliation."""
import argparse, base64, hashlib, json, os, re, subprocess, sys
from html.parser import HTMLParser
from pathlib import Path

ROOT=Path(__file__).resolve().parent
SOURCE_SHA='f4a7aa5c1de55880f76028fdfc7d0cd32f5184f4f40d625ec73024390e985cf8'

def sha(b):return hashlib.sha256(b).hexdigest()
def dump(p,x):p.open('x').write(json.dumps(x,ensure_ascii=False,allow_nan=False,indent=2)+'\n')
def snap(p):
    s=p.lstat()
    if p.is_symlink():return dict(link=os.readlink(p),mtime_ns=s.st_mtime_ns,inode=s.st_ino)
    b=p.read_bytes();return dict(bytes=len(b),sha256=sha(b),mtime_ns=s.st_mtime_ns,inode=s.st_ino)

class Node:
    def __init__(self,tag,attrs):self.tag=tag;self.attrs=dict(attrs);self.children=[]
    def text(self):return ''.join(x if isinstance(x,str) else x.text() for x in self.children)
    def find(self,predicate):
        return ([self] if predicate(self) else [])+[y for x in self.children if isinstance(x,Node) for y in x.find(predicate)]

class Document(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True);self.root=Node('root',[]);self.stack=[self.root]
    def handle_starttag(self,tag,attrs):
        n=Node(tag,attrs);self.stack[-1].children.append(n)
        if tag not in {'meta','input','br','hr','link','img','wbr','source','area','base','embed','param','track','col'}:self.stack.append(n)
    def handle_endtag(self,tag):
        for i in range(len(self.stack)-1,0,-1):
            if self.stack[i].tag==tag:self.stack=self.stack[:i];return
    def handle_data(self,data):self.stack[-1].children.append(data)

def visible(s):
    return ''.join('⟦U+%04X⟧'%ord(c) if (ord(c)<32 and c not in '\t\n\r') or 127<=ord(c)<=159 else c for c in s)
def formatted(v):return f'{v:,.3f}'.rstrip('0').rstrip('.') if isinstance(v,float) else f'{v:,}'

def reconcile(html_bytes,raw,labels):
    supplied=json.loads(raw);p=Document();p.feed(html_bytes.decode('utf-8'));root=p.root
    def only(pred):
        found=root.find(pred);assert len(found)==1,len(found);return found[0]
    table=only(lambda n:n.attrs.get('id')=='pairs');rows=table.find(lambda n:n.tag=='tr' and 'data-index'in n.attrs)
    assert len(rows)==len(supplied['comparisons'])
    full=[]
    for i,(row,expect) in enumerate(zip(rows,supplied['comparisons'])):
        cells=row.find(lambda n:n.tag=='td');actual=[c.text() for c in cells]
        wanted=[visible(expect[k]) for k in ('scenario','trainee')]+[str(expect[k]) for k in ('seed','baseline_score','input_score')]+[f"{expect['score_delta']:+d}"]+[visible(expect[k]) for k in ('baseline_grade','input_grade')]
        assert actual==wanted,(i,actual,wanted)
        assert row.attrs=={'data-index':str(i),'data-delta':str(expect['score_delta']),'data-outcome':('wins' if expect['score_delta']>0 else 'losses' if expect['score_delta']<0 else 'ties')}
        assert all(all(isinstance(child,str) for child in c.children) for c in cells),'data became markup'
        full.append(actual)
    download=only(lambda n:n.tag=='a' and 'download'in n.attrs)
    assert download.attrs['href'].startswith('data:application/json;base64,')
    embedded=base64.b64decode(download.attrs['href'].split(',',1)[1],validate=True);assert embedded==raw
    direction=only(lambda n:n.attrs.get('class')=='direction');assert direction.text()==labels['input']+' − '+labels['baseline']
    assert all(isinstance(c,str) for c in direction.children)
    s=supplied['summary'];cards=root.find(lambda n:n.attrs.get('class')=='card')
    actual_cards=[(c.find(lambda n:n.tag=='dt')[0].text(),c.find(lambda n:n.tag=='dd')[0].text())for c in cards]
    expect_cards=list(zip(['Complete pairs','Mean score difference','Median score difference','Population standard deviation'],[formatted(s[k])for k in ['count','mean_score_delta','median_score_delta','population_stddev_score_delta']]))
    assert actual_cards==expect_cards
    legend=only(lambda n:n.attrs.get('class')=='legend');assert legend.text()==''.join(f'{s[k]:,} {k}'for k in ['wins','ties','losses'])
    stats=only(lambda n:n.attrs.get('class')=='stats');assert stats.text()==f"Lowest difference{s['min_score_delta']:+,d}Highest difference{s['max_score_delta']:+,d}"
    assert only(lambda n:n.attrs.get('id')=='match-count').text()==f"{s['count']} of {s['count']} pairs shown"
    assert only(lambda n:n.attrs.get('id')=='input-sha256').text()==sha(raw)
    assert root.find(lambda n:n.tag=='noscript')
    assert not root.find(lambda n:any((v or '').startswith(('http:','https:','//'))for k,v in n.attrs.items()if k in {'src','href','action'}))
    assert not root.find(lambda n:any(k.lower().startswith('on')for k in n.attrs))
    scripts=root.find(lambda n:n.tag=='script');assert len(scripts)==1 and scripts[0].attrs=={}
    csp=only(lambda n:n.attrs.get('http-equiv')=='Content-Security-Policy').attrs['content']
    script_pin=base64.b64encode(hashlib.sha256(scripts[0].text().encode()).digest()).decode();assert "script-src 'sha256-"+script_pin+"'" in csp
    assert not re.search(r'url\s*\(',only(lambda n:n.tag=='style').text(),re.I)
    return dict(result='PASS',html_bytes=len(html_bytes),html_sha256=sha(html_bytes),input_sha256=sha(raw),rows=full,summary_cards=actual_cards,complete_summary=s,embedded_exact=True,static_full_table=True,script_sha256=sha(scripts[0].text().encode()),scope='Static HTML reconciliation, not browser interaction.')

def main():
    ap=argparse.ArgumentParser();ap.add_argument('--source',type=Path,required=True);ap.add_argument('--saved-html',type=Path,required=True);ap.add_argument('--saved-launch',type=Path,required=True);args=ap.parse_args()
    assert sha(args.source.read_bytes())==SOURCE_SHA
    oracle=json.loads((ROOT/'ORACLE.json').read_text());manifest=json.loads((ROOT/'fixture-manifest.json').read_text())
    for n,m in manifest.items():assert snap(ROOT/'fixtures'/n)['sha256']==m['sha256']
    course=ROOT/'course';course.mkdir();logs=course/'streams';logs.mkdir();work=course/'work';work.mkdir()
    existing=work/'existing.html';existing.write_bytes(b'owned existing output sentinel 596c\n')
    out_target=work/'symlink-target.html';out_target.write_bytes(b'owned symlink target sentinel 596c\n')
    out_link=work/'output-link.html';out_link.symlink_to(out_target)
    input_link=work/'input-link.json';input_link.symlink_to(ROOT/'fixtures/mixed14.json')
    protected=[args.source,args.saved_html,args.saved_launch]+sorted((ROOT/'fixtures').iterdir())+[ROOT/'ORACLE.json',ROOT/'fixture-manifest.json',ROOT/'PROTOCOL.md',Path(__file__),existing,out_target,out_link,input_link]
    before={str(p):snap(p)for p in protected};commands=[];checks=[]
    def check(name,a,b=True):
        checks.append(dict(name=name,actual=a,expected=b));assert a==b,name
    def invoke(name,inp,out,digest=None,success=False):
        cmd=[sys.executable,'-B',str(args.source),'--input',str(inp),'--input-sha256',digest or sha(inp.read_bytes()),'--output',str(out)]
        if success:cmd+=['--baseline-label',oracle['labels']['baseline'],'--input-label',oracle['labels']['input']]
        r=subprocess.run(cmd,capture_output=True,timeout=15)
        (logs/(name+'.stdout')).write_bytes(r.stdout);(logs/(name+'.stderr')).write_bytes(r.stderr)
        row=dict(name=name,args=cmd,exit=r.returncode,stdout_bytes=len(r.stdout),stdout_sha256=sha(r.stdout),stderr_bytes=len(r.stderr),stderr_sha256=sha(r.stderr));commands.append(row)
        check(name+' exit',r.returncode,0 if success else 2)
        if success:
            check(name+' stderr',r.stderr,b'');parsed=json.loads(r.stdout);b=out.read_bytes()
            check(name+' receipt',parsed,dict(format='uma-sim.paired-html-receipt.v1',status='ok',input_sha256=sha(inp.read_bytes()),input_bytes=len(inp.read_bytes()),output_sha256=sha(b),output_bytes=len(b),pairs=14,output=str(out)))
        else:
            check(name+' stdout',r.stdout,b'');parsed=json.loads(r.stderr);check(name+' error',type(parsed)is dict and parsed.get('status')=='error' and isinstance(parsed.get('error'),str)and bool(parsed['error']))
            if out not in {existing,ROOT/'fixtures/mixed14.json',out_link}:check(name+' no output',not os.path.lexists(out))
        return row
    try:
        output=work/'mixed14.html';invoke('mixed-positive',ROOT/'fixtures/mixed14.json',output,success=True)
        mixed=reconcile(output.read_bytes(),(ROOT/'fixtures/mixed14.json').read_bytes(),oracle['labels']);dump(course/'mixed-static.json',mixed)
        saved=reconcile(args.saved_html.read_bytes(),(ROOT/'fixtures/saved16.json').read_bytes(),dict(baseline='Default (saved study)',input='Local bot (saved study)'));dump(course/'saved16-static.json',saved)
        for case in oracle['malformed_cases']:invoke(case['name'],ROOT/'fixtures'/case['file'],work/(case['name']+'.html'))
        inp=ROOT/'fixtures/mixed14.json'
        invoke('wrong-digest',inp,work/'wrong-digest.html',digest='0'*64)
        invoke('existing-output',inp,existing)
        invoke('input-alias',inp,inp)
        invoke('symlink-output',inp,out_link)
        invoke('symlink-input',input_link,work/'symlink-input.html')
        after={str(p):snap(p)for p in protected};check('complete protected custody',after,before)
        def safe(v):
            if isinstance(v,bytes):return {'bytes_hex':v.hex()}
            if isinstance(v,dict):return {k:safe(x)for k,x in v.items()}
            if isinstance(v,list):return [safe(x)for x in v]
            return v
        dump(course/'receipt.json',safe(dict(schema='uma-html-independent-cli/1',result='PASS',runtime=sys.version,source_sha256=SOURCE_SHA,commands=commands,checks=checks,before=before,after=after,saved16_generation='Author actual existing command received, no duplicate invocation',browser='Pending separate actual browser receiving')))
        print(json.dumps(dict(result='PASS',calls=len(commands),successes=1,refusals=len(commands)-1,predicates=len(checks),mixed_html_sha256=mixed['html_sha256'],saved16_html_sha256=saved['html_sha256'])))
    except BaseException as e:
        dump(course/'failure.json',dict(error_type=type(e).__name__,error=str(e),commands=commands,after={str(p):snap(p)for p in protected}));raise

if __name__=='__main__':main()
