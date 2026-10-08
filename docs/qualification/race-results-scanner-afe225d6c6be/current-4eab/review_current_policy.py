import pathlib, json, hashlib, subprocess, tomllib, time, shutil
root=pathlib.Path("/var/tmp/hamon-gitleaks-afe225d6c6be-yy3nog5g")
out=root/"native-current-4eab"
out.mkdir()
def sha(data):return hashlib.sha256(data).hexdigest()
def blob(data):return hashlib.sha1(f"blob {len(data)}\0".encode()+data).hexdigest()
payload=json.loads((root/"continuity-4eab-input.json").read_text())
current=payload["current_policy"].encode()
candidate=payload["final_policy"].encode()
prior=(root/"policies/current.toml").read_bytes()
old_candidate=(root/"policies/candidate.toml").read_bytes()
assert blob(current)==payload["current_blob"]
assert current.startswith(prior)
assert candidate==current+old_candidate[len(prior):]
old_receipt=root/"native-review-r1/evidence/receipt.json"
assert sha(old_receipt.read_bytes())=="16b86f1fba6bf84237436b5c1ef31dd89b1b1df76e017b77190ab9bdfbace37e"
parsed_current=tomllib.loads(current.decode())
parsed_final=tomllib.loads(candidate.decode())
parsed_old=tomllib.loads(old_candidate.decode())
assert parsed_final["extend"]=={"useDefault":True}
assert len(parsed_final["rules"])==1
old_rules=parsed_current["rules"][0]["allowlists"]
assert parsed_final["rules"][0]["allowlists"]==old_rules+parsed_old["rules"][0]["allowlists"][-3:]
assert len(old_rules)==5
bound=json.loads((root/"source-bindings.json").read_text())
fixture=root/"native-review-r1/candidate-exact-inputs"
files={}
for path,expected in bound["file_blobs"].items():
    source=(root/"source"/path).read_bytes()
    assert blob(source)==expected
    assert (fixture/path).read_bytes()==source
    files[path]={"git_blob":expected,"sha256":sha(source),"bytes":len(source)}
scanner=root/"release/gitleaks"
assert sha(scanner.read_bytes())==bound["scanner_sha256"]
version=subprocess.check_output([str(scanner),"version"],text=True).strip()
assert version=="8.24.3"
current_path=out/"current.toml"
candidate_path=out/"candidate.toml"
current_path.write_bytes(current)
candidate_path.write_bytes(candidate)
report=out/"scanner-report.json"
argv=[str(scanner),"detect","--redact","-v","--exit-code=2","--report-format=json",f"--report-path={report}","--log-level=debug","--log-opts=-1",f"--config={candidate_path}"]
assert shutil.disk_usage(out).free>1024**3
start=time.strftime("%Y-%m-%dT%H:%M:%SZ",time.gmtime())
result=subprocess.run(argv,cwd=fixture,capture_output=True,text=True,timeout=30)
log=out/"scanner.log"
log.write_text(result.stdout+result.stderr)
observed=json.loads(report.read_text()) or []
receipt={"started_utc":start,"finished_utc":time.strftime("%Y-%m-%dT%H:%M:%SZ",time.gmtime()),"current_main":payload["current_commit"],"prior_native_receipt_sha256":sha(old_receipt.read_bytes()),"prior_policy_git_blob":blob(prior),"current_policy":{"git_blob":blob(current),"sha256":sha(current),"bytes":len(current)},"final_policy":{"git_blob":blob(candidate),"sha256":sha(candidate),"bytes":len(candidate)},"current_prefix_preserved":True,"all_five_inherited_allowances_preserved":True,"same_three_qualified_allowances_appended":True,"source_files":files,"scanner_version":version,"scanner_sha256":sha(scanner.read_bytes()),"command":argv,"exit_code":result.returncode,"observed_findings":len(observed),"report_sha256":sha(report.read_bytes()),"log_sha256":sha(log.read_bytes()),"original_five_files_unchanged":all((fixture/p).read_bytes()==(root/"source"/p).read_bytes() for p in files),"free_after_bytes":shutil.disk_usage(out).free,"pass":result.returncode==0 and not observed,"scope":"One exact-five-files scan after preserving the entire new current policy; ten original controls remain separately frozen."}
raw=(json.dumps(receipt,indent=2)+"\n").encode()
(out/"continuity-receipt.json").write_bytes(raw)
print(json.dumps({"status":"PASS" if receipt["pass"] else "FAIL","receipt_sha256":sha(raw),"final_policy":receipt["final_policy"],"observed_findings":len(observed),"free_after_bytes":receipt["free_after_bytes"]},indent=2))
raise SystemExit(0 if receipt["pass"] else 1)
