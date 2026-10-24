/* Shared PerchFlow UI dialogs; no browser-native prompt/confirm/alert. */
'use strict';
window.PerchDialog=(()=>{
  let active=null;
  const tt=(zh,en)=>typeof getLang==='function'&&getLang()==='en'?en:zh;
  function tag(name,cls,value){
    const e=document.createElement(name);
    if(cls)e.className=cls;
    if(value!==undefined)e.textContent=String(value);
    return e;
  }
  function build(title,description){
    if(active)active.cancel();
    const previous=document.activeElement;
    const overlay=tag('div','pf-dialog-overlay');
    const panel=tag('div','pf-dialog-panel');
    panel.setAttribute('role','dialog');
    panel.setAttribute('aria-modal','true');
    const header=tag('div','pf-dialog-head');
    const titleNode=tag('h2','pf-dialog-title',title);
    titleNode.id='pf-dialog-title';
    panel.setAttribute('aria-labelledby',titleNode.id);
    const x=tag('button','pf-dialog-close','×');
    x.type='button';x.setAttribute('aria-label',tt('关闭','Close'));
    header.append(titleNode,x);panel.append(header);
    if(description)panel.append(tag('p','pf-dialog-help',description));
    const content=tag('div','pf-dialog-content');
    const error=tag('p','pf-dialog-error');
    error.hidden=true;error.setAttribute('role','alert');
    const footer=tag('div','pf-dialog-footer');
    panel.append(content,error,footer);overlay.append(panel);
    document.body.append(overlay);
    let complete;
    const promise=new Promise(resolve=>{complete=resolve;});
    let busy=false;
    const close=(value)=>{
      if(active!==instance||busy)return;
      active=null;
      document.removeEventListener('keydown',keyDown,true);
      overlay.remove();
      if(previous?.isConnected&&typeof previous.focus==='function')previous.focus({preventScroll:true});
      complete(value);
    };
    const keyDown=e=>{
      if(e.key==='Escape'){e.preventDefault();close(null);}
      if(e.key==='Tab'){
        const focus=Array.from(panel.querySelectorAll('button:not(:disabled),input:not(:disabled),[tabindex="0"]')).filter(el=>el.offsetParent!==null);
        if(focus.length){
          const first=focus[0],last=focus[focus.length-1];
          if(e.shiftKey&&document.activeElement===first){e.preventDefault();last.focus();}
          else if(!e.shiftKey&&document.activeElement===last){e.preventDefault();first.focus();}
        }
      }
    };
    const instance={panel,content,error,footer,promise,close,cancel:()=>close(null),setBusy(value){busy=value;x.disabled=value;}};
    active=instance;
    x.addEventListener('click',instance.cancel);
    overlay.addEventListener('click',e=>{if(e.target===overlay)instance.cancel();});
    document.addEventListener('keydown',keyDown,true);
    x.focus();
    return instance;
  }
  function button(text,primary=false,danger=false){
    const b=tag('button','pf-dialog-button'+(primary?' pf-dialog-primary':'')+(danger?' pf-dialog-danger':''),text);
    b.type='button';return b;
  }
  function showError(dialog,e){
    dialog.error.textContent=e instanceof Error?e.message:String(e);
    dialog.error.hidden=false;
  }
  async function confirm(text,options={}){
    const d=build(options.title||tt('确认操作','Confirm action'),text);
    const no=button(tt('取消','Cancel'));
    const yes=button(options.ok||tt('确定','Confirm'),true,!!options.danger);
    d.footer.append(no,yes);
    no.onclick=()=>d.close(false);
    yes.onclick=()=>d.close(true);
    const result=await d.promise;
    return result===true;
  }
  async function notice(text,options={}){
    const d=build(options.title||tt('提示','Notice'),text);
    const ok=button(tt('知道了','OK'),true);
    ok.onclick=()=>d.close(true);d.footer.append(ok);
    return d.promise;
  }
  function describeCron(value){
    const parts=value.trim().split(/\s+/);
    if(![6,7].includes(parts.length))return tt('需输入包含秒的 6 或 7 段 Cron','Enter 6 or 7 Cron fields (including seconds)');
    if(parts.slice(0,3).every(x=>/^\d+$/.test(x))&&parts[3]==='*'&&parts[4]==='*'&&parts[5]==='*'&&(parts.length===6||parts[6]==='*')){
      const [s,m,h]=parts.map(Number);
      if(s<60&&m<60&&h<24)return tt('每天 ','Every day at ')+[h,m,s].map(v=>String(v).padStart(2,'0')).join(':')+
        tt('（电脑本地时间）',' (computer local time)');
    }
    if(parts.length===7&&parts.slice(0,5).every(x=>/^\d+$/.test(x))&&parts[5]==='*'&&/^\d{4}$/.test(parts[6])){
      const [sec,min,hour,day,month,year]=[+parts[0],+parts[1],+parts[2],+parts[3],+parts[4],+parts[6]];
      if(sec<60&&min<60&&hour<24&&day>0&&day<=31&&month>0&&month<=12){
        return String(year)+'-'+String(month).padStart(2,'0')+'-'+String(day).padStart(2,'0')+' '+
          [hour,min,sec].map(x=>String(x).padStart(2,'0')).join(':')+tt('（电脑本地时间）',' (computer local time)');
      }
    }
    return tt('按 Cron 表达式执行（电脑本地时间）','Runs using Cron (computer local time)');
  }
  async function cron({title,initial='',editing=false,onSave}){
    const d=build(title||tt('添加定时计划','Add schedule'),
      tt('支持包含秒数的 6 或 7 段 Cron 表达式','Cron supports 6 or 7 fields including seconds'));
    const label=tag('label','pf-dialog-field');
    label.append(tag('span','',tt('Cron 表达式','Cron expression')));
    const input=tag('input','pf-dialog-input');
    input.value=initial||'0 0 0 * * *';
    input.spellcheck=false;input.autocomplete='off';
    label.append(input);d.content.append(label);
    const preview=tag('p','pf-dialog-preview');
    const update=()=>{preview.textContent=describeCron(input.value);};
    input.addEventListener('input',update);update();d.content.append(preview);
    d.content.append(tag('p','pf-dialog-hint',editing
      ?tt('修改后保留原来的启用或停用状态','Editing preserves current enabled/disabled status')
      :tt('首次创建后默认启用，立即加入调度','New schedules are enabled immediately')));
    const no=button(tt('取消','Cancel')),yes=button(tt('保存计划','Save schedule'),true);
    d.footer.append(no,yes);no.onclick=d.cancel;
    const submit=async()=>{
      const schedule=input.value.trim();
      if(![6,7].includes(schedule.split(/\s+/).length)||!schedule){
        showError(d,tt('请输入有效的 6 或 7 段 Cron 表达式','Enter a valid 6/7-field Cron expression'));return;
      }
      yes.disabled=true;no.disabled=true;d.setBusy(true);d.error.hidden=true;
      try{await onSave(schedule);d.setBusy(false);d.close(true);}
      catch(e){if(d.panel.isConnected)showError(d,e);}
      finally{d.setBusy(false);yes.disabled=false;no.disabled=false;}
    };
    yes.onclick=submit;input.addEventListener('keydown',e=>{if(e.key==='Enter'){e.preventDefault();void submit();}});
    input.focus();input.select();
    return d.promise;
  }
  async function chooseScript(tasks){
    const d=build(tt('选择要设置定时的脚本','Choose a script to schedule'),
      tt('选择脚本后配置定时计划，不需要输入序号','Choose a script to configure its schedule'));
    const search=tag('input','pf-dialog-input');
    search.placeholder=tt('搜索脚本名称或 Python 文件名','Search script or Python filename');
    search.setAttribute('aria-label',search.placeholder);
    d.content.append(search);
    const list=tag('div','pf-dialog-choices');
    d.content.append(list);
    let selected=null;
    const render=()=>{
      list.replaceChildren();
      const q=search.value.trim().toLowerCase();
      const matches=tasks.filter(t=>[t.name,t.project,...(t.args||[])].join(' ').toLowerCase().includes(q));
      if(selected&&!matches.includes(selected)){selected=null;next.disabled=true;}
      if(!matches.length){list.append(tag('p','pf-dialog-hint',tt('没有匹配的脚本','No matching scripts')));return;}
      for(const item of matches){
        const btn=button('');
        btn.className='pf-dialog-choice'+(selected===item?' is-selected':'');
        const name=tag('strong','',item.display_name||item.name);
        const path=String((item.args||[]).find(a=>String(a).toLowerCase().endsWith('.py'))||'').replace(/\\/g,'/').split('/').pop();
        const subtitle=tag('small','',String(item.project||'')+(path?' · '+path:''));
        btn.append(name,subtitle);
        btn.setAttribute('aria-pressed',String(selected===item));
        btn.onclick=()=>{selected=item;next.disabled=false;render();};
        list.append(btn);
      }
    };
    const no=button(tt('取消','Cancel')),next=button(tt('下一步：配置计划','Next: schedule'),true);
    next.disabled=true;d.footer.append(no,next);no.onclick=d.cancel;next.onclick=()=>d.close(selected);
    search.addEventListener('input',render);render();search.focus();
    return d.promise;
  }
  return {confirm,notice,cron,chooseScript};
})();
