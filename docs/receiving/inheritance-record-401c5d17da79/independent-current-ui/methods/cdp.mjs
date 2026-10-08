import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { once } from 'node:events';

export class CDP {
  constructor(socket) {
    this.socket=socket; this.sequence=0; this.waiters=new Map(); this.listeners=new Map();
    socket.addEventListener('message',event=>{
      const message=JSON.parse(String(event.data));
      if(message.id){const waiter=this.waiters.get(message.id);if(!waiter)return;
        clearTimeout(waiter.timer);this.waiters.delete(message.id);
        message.error?waiter.reject(new Error(JSON.stringify(message.error))):waiter.resolve(message.result??{});
      } else for(const fn of this.listeners.get(message.method)??[])fn(message.params??{});
    });
  }
  static async connect(url) {
    const socket=new WebSocket(url);
    await new Promise((resolve,reject)=>{socket.addEventListener('open',resolve,{once:true});socket.addEventListener('error',reject,{once:true});});
    return new CDP(socket);
  }
  on(method,fn){this.listeners.set(method,[...(this.listeners.get(method)??[]),fn]);}
  send(method,params={}) {
    const id=++this.sequence;
    return new Promise((resolve,reject)=>{
      const timer=setTimeout(()=>{this.waiters.delete(id);reject(new Error('CDP timeout: '+method));},15000);
      this.waiters.set(id,{resolve,reject,timer});this.socket.send(JSON.stringify({id,method,params}));
    });
  }
  async evaluate(expression) {
    const result=await this.send('Runtime.evaluate',{expression,returnByValue:true,awaitPromise:true});
    if(result.exceptionDetails)throw new Error(JSON.stringify(result.exceptionDetails));
    return result.result.value;
  }
  async wait(expression,timeout=15000) {
    const start=Date.now();
    while(Date.now()-start<timeout){if(await this.evaluate(expression))return;await new Promise(r=>setTimeout(r,50));}
    throw new Error('Page condition timed out: '+expression);
  }
  async click(selector) {
    const xy=await this.evaluate('(()=>{const e=document.querySelector('+JSON.stringify(selector)+');if(!e)throw Error("missing control");e.scrollIntoView({block:"center"});const r=e.getBoundingClientRect();return {x:r.x+r.width/2,y:r.y+r.height/2};})()');
    await this.send('Input.dispatchMouseEvent',{type:'mousePressed',button:'left',clickCount:1,...xy});
    await this.send('Input.dispatchMouseEvent',{type:'mouseReleased',button:'left',clickCount:1,...xy});
  }
  async fill(selector,text) {
    await this.evaluate('(()=>{const e=document.querySelector('+JSON.stringify(selector)+');e.focus();e.select();})()');
    await this.send('Input.insertText',{text});
  }
  async screenshot(path){
    const result=await this.send('Page.captureScreenshot',{format:'png',captureBeyondViewport:false});
    await writeFile(path,Buffer.from(result.data,'base64'));
  }
}

export async function openBrowser({profile,evidence,width=1280,height=900}) {
  await mkdir(profile,{recursive:true});await mkdir(evidence,{recursive:true});
  const child=spawn('/snap/bin/chromium',[
    '--headless=new','--no-sandbox','--disable-dev-shm-usage','--disable-background-networking',
    '--disable-component-update','--disable-default-apps','--disable-extensions','--disable-sync',
    '--no-first-run','--no-default-browser-check','--remote-debugging-address=127.0.0.1',
    '--remote-debugging-port=0','--user-data-dir='+profile,'about:blank',
  ],{stdio:['ignore','pipe','pipe']});
  let stderr='',stdout='';
  child.stdout.on('data',b=>{stdout+=b;});child.stderr.on('data',b=>{stderr+=b;});
  const start=Date.now();let endpoint;
  while(Date.now()-start<20000){
    endpoint=stderr.match(/DevTools listening on (ws:\/\/\S+)/)?.[1];
    if(endpoint)break;
    if(child.exitCode!==null)break;
    await new Promise(r=>setTimeout(r,50));
  }
  if(!endpoint){await writeFile(evidence+'/browser-start.json',JSON.stringify({exitCode:child.exitCode,stderr,stdout},null,2));child.kill();throw new Error('Chromium did not expose the owned debugging endpoint');}
  const port=new URL(endpoint).port;
  const tab=await fetch('http://127.0.0.1:'+port+'/json/new?about:blank',{method:'PUT'}).then(r=>r.json());
  const cdp=await CDP.connect(tab.webSocketDebuggerUrl);
  const errors=[],blocked=[];
  cdp.on('Runtime.exceptionThrown',p=>errors.push(p));
  cdp.on('Fetch.requestPaused',async p=>{
    const url=p.request.url;
    if(url.startsWith('http://127.0.0.1:')||url.startsWith('data:')||url.startsWith('blob:'))await cdp.send('Fetch.continueRequest',{requestId:p.requestId});
    else{blocked.push(url);await cdp.send('Fetch.failRequest',{requestId:p.requestId,errorReason:'BlockedByClient'});}
  });
  await cdp.send('Runtime.enable');await cdp.send('Page.enable');
  await cdp.send('Fetch.enable',{patterns:[{urlPattern:'*'}]});
  await cdp.send('Emulation.setDeviceMetricsOverride',{width,height,deviceScaleFactor:1,mobile:false});
  await cdp.send('Page.addScriptToEvaluateOnNewDocument',{source:`(()=>{
    const proto=CanvasRenderingContext2D.prototype;
    const fill=proto.fillRect,text=proto.fillText;
    proto.fillRect=function(...args){if(args[0]===0&&args[1]===0)this.__radarText=[];return Reflect.apply(fill,this,args);};
    proto.fillText=function(...args){(this.__radarText??=[]).push(args);return Reflect.apply(text,this,args);};
  })();`});
  const version=await cdp.send('Browser.getVersion');
  return {cdp,errors,blocked,version,async close(){
    try{await cdp.send('Browser.close');}catch{}
    cdp.socket.close();
    if(child.exitCode===null){await Promise.race([once(child,'exit'),new Promise(r=>setTimeout(r,2000))]);}
    if(child.exitCode===null)child.kill();
    await writeFile(evidence+'/browser-process.json',JSON.stringify({version,exitCode:child.exitCode,stderr,stdout,blocked},null,2)+'\n');
  }};
}
