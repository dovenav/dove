(function(){
  const q = document.getElementById('q');
  const cards = Array.from(document.querySelectorAll('.card'));
  const engineBtn = document.getElementById('engineBtn');
  const engineMenu = document.getElementById('engineMenu');
  const doSearch = document.getElementById('doSearch');
  const clockEl = document.getElementById('clock');
  const clockDateEl = document.getElementById('clock-date');
  // Categories
  const catList = document.getElementById('cats');
  const sections = Array.from(document.querySelectorAll('section.group'));

  function filter() {
    const v = (q && q.value || '').toLowerCase().trim();
    cards.forEach(c => {
      const name = (c.getAttribute('data-name')||'').toLowerCase();
      const host = (c.getAttribute('data-host')||'').toLowerCase();
      const desc = (c.getAttribute('data-desc')||'').toLowerCase();
      const t = `${name} ${host} ${desc}`;
      c.style.display = v ? (t.includes(v) ? '' : 'none') : '';
    });
  }
  q && q.addEventListener('input', filter);

  function engines(){
    if(!engineMenu) return [];
    return Array.from(engineMenu.querySelectorAll('li')).map(li=>({name: li.dataset.name, tpl: li.dataset.template, el: li}));
  }
  function selectedEngineName(){ return localStorage.getItem('dove-engine'); }
  function setSelectedEngine(name){ if(!engineMenu) return; const list = engines(); const e = list.find(x=>x.name===name) || list[0]; if(!e) return; localStorage.setItem('dove-engine', e.name); const label = document.getElementById('engineLabel'); if(label) label.textContent = e.name; list.forEach(x=>x.el.setAttribute('aria-selected', x===e? 'true':'false')); }
  function currentEngineUrl(qs){ const list = engines(); let name = selectedEngineName(); let e = list.find(x=>x.name===name) || list[0]; if(!e) return null; return (e.tpl||'').replace('{q}', encodeURIComponent(qs)); }
  function toggleEngineMenu(force){ if(!engineMenu || !engineBtn) return; const open = force!==undefined? force : engineMenu.hasAttribute('hidden'); if(open){ engineMenu.removeAttribute('hidden'); engineBtn.setAttribute('aria-expanded','true'); } else { engineMenu.setAttribute('hidden',''); engineBtn.setAttribute('aria-expanded','false'); }}
  function nextEngine(delta){ const list = engines(); if(list.length===0) return; const name = selectedEngineName(); let idx = Math.max(0, list.findIndex(x=>x.name===name)); idx = (idx + delta + list.length) % list.length; setSelectedEngine(list[idx].name); }
  // init engine
  (function initEngine(){ const list = engines(); if(list.length===0) return; const saved = selectedEngineName(); if(saved){ setSelectedEngine(saved);} else { setSelectedEngine(list[0].name);} list.forEach(x=>{ x.el.addEventListener('click', ()=>{ setSelectedEngine(x.name); toggleEngineMenu(false); }); x.el.addEventListener('keydown', (ev)=>{ if(ev.key==='Enter' || ev.key===' '){ ev.preventDefault(); setSelectedEngine(x.name); toggleEngineMenu(false);} }); }); })();
  engineBtn && engineBtn.addEventListener('click', ()=> toggleEngineMenu());
  document.addEventListener('click', (ev)=>{ if(engineMenu && !engineMenu.hasAttribute('hidden')){ if(!engineMenu.contains(ev.target) && ev.target!==engineBtn){ toggleEngineMenu(false);} } });

  function externalSearch(){
    const v = (q && q.value || '').trim();
    if(!v) return;
    const url = currentEngineUrl(v);
    if(url) window.open(url, '_blank', 'noopener');
  }
  doSearch && doSearch.addEventListener('click', externalSearch);

  q && q.addEventListener('keydown', function(ev){
    if(ev.key === 'Enter'){
      const v = (q && q.value || '').trim();
      if(!v){ return; }
      const visible = cards.filter(c => c.style.display !== 'none');
      if(ev.shiftKey){ externalSearch(); return; }
      if(visible.length > 0){ const href = visible[0].getAttribute('href'); if(href) window.open(href, '_blank', 'noopener'); }
      else { externalSearch(); }
    }
  });

  // 主题切换保留
  const btn = document.getElementById('toggleTheme');
  function setTheme(t){
    document.body.classList.remove('theme-auto','theme-light','theme-dark');
    document.body.classList.add('theme-'+t);
    localStorage.setItem('dove-theme', t);
  }
  const saved = localStorage.getItem('dove-theme');
  if(saved){ setTheme(saved); }
  btn && btn.addEventListener('click', function(){
    const cur = (localStorage.getItem('dove-theme')||'auto');
    const nxt = cur==='auto' ? 'light' : (cur==='light'?'dark':'auto');
    setTheme(nxt);
  });

  // 快捷键：/ 或 Ctrl/Cmd+K 聚焦搜索；Alt+Shift+E 切换引擎；Alt+上下切换选项
  document.addEventListener('keydown', function(ev){
    const isMac = navigator.platform.toUpperCase().indexOf('MAC')>=0;
    if((ev.key==='/' && document.activeElement!==q) || ((isMac? ev.metaKey:ev.ctrlKey) && ev.key.toLowerCase()==='k')){
      ev.preventDefault(); q && q.focus(); return;
    }
    if(ev.altKey && ev.shiftKey && ev.key.toLowerCase()==='e'){ ev.preventDefault(); nextEngine(1); return; }
    if(ev.altKey && (ev.key==='ArrowUp' || ev.key==='ArrowDown')){ ev.preventDefault(); nextEngine(ev.key==='ArrowDown'?1:-1); return; }
    if((isMac? ev.metaKey:ev.ctrlKey) && ev.key==='/'){ ev.preventDefault(); externalSearch(); return; }
  });

  // Clock
  function pad(n){ return n<10? '0'+n : ''+n; }
  function cnWeekday(d){ return ['日','一','二','三','四','五','六'][d.getDay()]; }
  function fmtDateCN(d){
    const y = d.getFullYear();
    const m = (d.getMonth()+1).toString().padStart(2,'0');
    const day = d.getDate().toString().padStart(2,'0');
    return `${y}年${m}月${day}日`;
  }
  function lunarString(d){
    try {
      const fmt = new Intl.DateTimeFormat('zh-CN-u-ca-chinese', { month: 'long', day: 'numeric'});
      const s = fmt.format(d);
      // s 一般类似 “八月二十五” 或 “闰二月初三”，我们前面加“农历 ”
      return `农历${s}`;
    } catch(e) { return ''; }
  }
  function updateClock(){
    if(!clockEl) return;
    const now = new Date();
    const h = pad(now.getHours());
    const m = pad(now.getMinutes());
    const sec = now.getSeconds();
    clockEl.innerHTML = `${h}<span class="clock-colon${sec%2? ' off':''}">:</span>${m}`;
    if(clockDateEl){
      const dateStr = fmtDateCN(now);
      const weekStr = `周${cnWeekday(now)}`;
      const lunar = lunarString(now);
      clockDateEl.textContent = lunar ? `${dateStr}   ${weekStr}   ${lunar}` : `${dateStr}   ${weekStr}`;
    }
  }
  updateClock();
  setInterval(updateClock, 1000); // 每秒更新以实现冒号闪烁

  // Category switching
  function setActiveCat(name){
    if(!catList) return;
    const items = Array.from(catList.querySelectorAll('.cat-item'));
    items.forEach(it => it.classList.toggle('active', it.getAttribute('data-cat')===name));
    sections.forEach(sec => {
      const cat = sec.getAttribute('data-cat');
      sec.style.display = (!name || name===cat) ? '' : 'none';
    });
    localStorage.setItem('dove-cat', name||'');
  }
  function initCats(){
    if(!catList) return;
    const saved = localStorage.getItem('dove-cat');
    let target = saved;
    const items = Array.from(catList.querySelectorAll('.cat-item'));
    if(!target || !items.some(it=>it.getAttribute('data-cat')===target)){
      target = items.length? items[0].getAttribute('data-cat') : '';
    }
    items.forEach(it => {
      it.addEventListener('click', ()=> setActiveCat(it.getAttribute('data-cat')));
      it.addEventListener('keydown', (ev)=>{ if(ev.key==='Enter' || ev.key===' '){ ev.preventDefault(); setActiveCat(it.getAttribute('data-cat')); }});
    });
    setActiveCat(target);
  }
  initCats();
})();
