import hashlib,json,sys
b=sys.argv[1].encode()
print(json.dumps({'sha256':hashlib.sha256(b).hexdigest(),'git_blob':hashlib.sha1(b'blob '+str(len(b)).encode()+b'\0'+b).hexdigest(),'bytes':len(b)}))
