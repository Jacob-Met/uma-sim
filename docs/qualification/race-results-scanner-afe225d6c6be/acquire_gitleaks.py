import pathlib, tempfile, urllib.request, hashlib, tarfile, json, os, shutil, subprocess, time, traceback
root=pathlib.Path(tempfile.mkdtemp(prefix="hamon-gitleaks-afe225d6c6be-",dir="/var/tmp"))
os.chmod(root,0o700)
floor=1024**3
start=time.strftime("%Y-%m-%dT%H:%M:%SZ",time.gmtime())
manifest={"started_utc":start,"owned_root":str(root),"source":"official GitHub release v8.24.3","capacity_floor_bytes":floor,"free_before":shutil.disk_usage(root).free}
print(json.dumps({"owned_root":str(root),"free_before":manifest["free_before"],"capacity_floor":floor}),flush=True)
try:
    assert manifest["free_before"]>floor+64*1024**2,"insufficient capacity"
    base="https://github.com/gitleaks/gitleaks/releases/download/v8.24.3/"
    files={}
    for name,limit in (("gitleaks_8.24.3_checksums.txt",65536),("gitleaks_8.24.3_linux_x64.tar.gz",10*1024**2)):
        req=urllib.request.Request(base+name,headers={"User-Agent":"HAMON independent native receiving"})
        path=root/name
        size=0
        with urllib.request.urlopen(req,timeout=25) as response, path.open("xb") as out:
            final_url=response.url
            while True:
                block=response.read(65536)
                if not block: break
                size+=len(block)
                assert size<=limit,"download exceeds bound"
                assert shutil.disk_usage(root).free>floor+len(block),"capacity floor"
                out.write(block)
        data=path.read_bytes()
        files[name]={"url":base+name,"final_url":final_url,"bytes":len(data),"sha256":hashlib.sha256(data).hexdigest()}
        print(json.dumps({"downloaded":name,"bytes":len(data),"sha256":files[name]["sha256"]}),flush=True)
    archive_name="gitleaks_8.24.3_linux_x64.tar.gz"
    checks=(root/"gitleaks_8.24.3_checksums.txt").read_text()
    entries=[line.split() for line in checks.splitlines() if line.strip()]
    wanted=[entry[0] for entry in entries if len(entry)==2 and entry[1].lstrip("*")==archive_name]
    assert len(wanted)==1 and len(wanted[0])==64,"missing/ambiguous checksum"
    assert files[archive_name]["sha256"]==wanted[0],"official checksum mismatch"
    manifest["downloads"]=files
    manifest["official_checksum_verified"]=True
    extracted=root/"release"
    extracted.mkdir(mode=0o700)
    members=[]
    total=0
    with tarfile.open(root/archive_name,"r:gz") as archive:
        infos=archive.getmembers()
        assert len(infos)<30,"too many members"
        seen=set()
        for info in infos:
            p=pathlib.PurePosixPath(info.name)
            assert not p.is_absolute() and ".." not in p.parts and p.parts,"unsafe archive path"
            assert info.name not in seen,"duplicate archive member"
            seen.add(info.name)
            assert info.isfile() or info.isdir(),"archive link/device rejected"
            total+=info.size
            assert total<=50*1024**2,"expanded size bound"
        for info in infos:
            out=extracted.joinpath(*pathlib.PurePosixPath(info.name).parts)
            if info.isdir():
                out.mkdir(parents=True,exist_ok=True)
                continue
            out.parent.mkdir(parents=True,exist_ok=True)
            assert shutil.disk_usage(root).free>floor+info.size,"capacity floor"
            with archive.extractfile(info) as src:
                payload=src.read(info.size+1)
            assert len(payload)==info.size
            with out.open("xb") as f: f.write(payload)
            out.chmod(0o400)
            members.append({"name":info.name,"bytes":len(payload),"sha256":hashlib.sha256(payload).hexdigest()})
    executable=extracted/"gitleaks"
    assert executable.is_file() and executable.read_bytes()[:4]==b"\x7fELF","expected ELF"
    executable.chmod(0o500)
    proc=subprocess.run([str(executable),"version"],capture_output=True,text=True,timeout=10,cwd=root)
    assert proc.returncode==0 and proc.stdout.strip()=="8.24.3",proc.stdout+proc.stderr
    manifest.update({"members":members,"executable":str(executable),"version":proc.stdout.strip(),"version_stderr":proc.stderr,"free_after":shutil.disk_usage(root).free,"finished_utc":time.strftime("%Y-%m-%dT%H:%M:%SZ",time.gmtime())})
    assert manifest["free_after"]>floor
    raw=(json.dumps(manifest,indent=2)+"\n").encode()
    (root/"acquisition-receipt.json").write_bytes(raw)
    print(json.dumps({"status":"PASS","receipt":str(root/"acquisition-receipt.json"),"receipt_sha256":hashlib.sha256(raw).hexdigest(),"manifest":manifest},indent=2),flush=True)
except Exception as exc:
    failure={"type":type(exc).__name__,"message":str(exc),"root":str(root),"time":time.strftime("%Y-%m-%dT%H:%M:%SZ",time.gmtime())}
    try:(root/"acquisition-failure.json").write_text(json.dumps(failure,indent=2)+"\n")
    except Exception as secondary: failure["receipt_error"]=repr(secondary)
    print(json.dumps({"status":"FAIL","failure":failure}),flush=True)
    raise
