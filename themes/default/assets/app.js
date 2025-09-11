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
  const sidebar = document.getElementById('sidebar');
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
    // 搜索时：仅显示含有匹配结果的分组与分类；清空时恢复当前分类筛选
    if(v){
      // 逐分组统计是否有可见卡片
      const hasVisibleByCat = {};
      sections.forEach(sec => {
        const visible = Array.from(sec.querySelectorAll('.card')).some(x => x.style.display !== 'none');
        sec.style.display = visible ? '' : 'none';
        const cat = sec.getAttribute('data-cat') || '';
        if(visible){ hasVisibleByCat[cat] = true; }
      });
      // 侧边栏分类：仅显示仍有结果的分类；若全无结果则隐藏侧边栏
      if(catList){
        const items = Array.from(catList.querySelectorAll('.cat-item'));
        items.forEach(it => {
          const cat = it.getAttribute('data-cat') || '';
          it.style.display = hasVisibleByCat[cat] ? '' : 'none';
        });
        const anyVisibleCat = items.some(it => it.style.display !== 'none');
        if(sidebar){ sidebar.style.display = anyVisibleCat ? '' : 'none'; }
      }
    }else{
      // 恢复当前分类视图
      setActiveCat(currentCat || (catList && catList.querySelector('.cat-item') && catList.querySelector('.cat-item').getAttribute('data-cat')) || '');
      // 恢复侧边栏分类可见性
      if(catList){
        Array.from(catList.querySelectorAll('.cat-item')).forEach(it => { it.style.display = ''; });
      }
      // 如果存在侧边栏（即有分类数据），则恢复显示
      if(sidebar){ sidebar.style.display = (catList && catList.children.length>0) ? '' : 'none'; }
    }
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
  let currentCat = '';
  function setActiveCat(name){
    if(!catList) return;
    const items = Array.from(catList.querySelectorAll('.cat-item'));
    items.forEach(it => it.classList.toggle('active', it.getAttribute('data-cat')===name));
    currentCat = name || '';
    const v = (q && q.value || '').trim();
    if(!v){
      sections.forEach(sec => {
        const cat = sec.getAttribute('data-cat');
        sec.style.display = (!name || name===cat) ? '' : 'none';
      });
    }
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

  // Background image: free providers + controls
  const bgLayer = document.getElementById('bgLayer');
  const bgNextBtn = document.getElementById('bgNext');
  const bgIntervalSel = document.getElementById('bgInterval');
  const bgBlurSel = document.getElementById('bgBlur');

  // Providers without API keys. Rotate through to improve success rate.
  const BG_PROVIDERS = [
    (w,h)=>`https://picsum.photos/${Math.max(1280,w)}/${Math.max(720,h)}?random=${Date.now()}`,
    (w,h)=>`https://source.unsplash.com/random/${Math.max(1280,w)}x${Math.max(720,h)}?wallpapers,landscape&sig=${Math.floor(Math.random()*100000)}`
  ];
  const FALLBACK_BG = 'https://picsum.photos/1280/1264?random=' + Date.now();
  let providerIdx = 0;
  let bgTimer = null;
  // double-buffered bg layers for seamless crossfade
  let bgPrimary = null;   // currently visible layer
  let bgBuffer = null;    // hidden layer to preload next image
  let switching = false;  // prevent concurrent switches
  let queued = false;     // queue one extra switch request during switching
  let preloaded = null;   // { img, url } next preloaded image if available

  function vp(){ return { w: Math.max(800, window.innerWidth||800), h: Math.max(600, window.innerHeight||600) }; }
  function nextUrl(){ const {w,h}=vp(); providerIdx = (providerIdx+1)%BG_PROVIDERS.length; return BG_PROVIDERS[providerIdx](w,h); }
  function analyzeTone(img){
    try {
      const size = 32; const c = document.createElement('canvas'); c.width = size; c.height = size; const ctx = c.getContext('2d', { willReadFrequently: true });
      if(!ctx) return null;
      ctx.drawImage(img, 0, 0, size, size);
      const data = ctx.getImageData(0, 0, size, size).data;
      let sum = 0; const n = size*size; const toLin = (v)=>{ v/=255; return v<=0.04045 ? v/12.92 : Math.pow((v+0.055)/1.055, 2.4); };
      for(let i=0;i<data.length;i+=4){ const r=toLin(data[i]), g=toLin(data[i+1]), b=toLin(data[i+2]); const L = 0.2126*r + 0.7152*g + 0.0722*b; sum += L; }
      const avg = sum / n; return avg > 0.6 ? 'light' : 'dark';
    } catch(e){ return null; }
  }
  function applyTone(tone){ if(!tone) return; document.body.setAttribute('data-img-tone', tone);
    const root = document.documentElement;
    if(tone === 'light') { root.style.setProperty('--bg-overlay', '0.28'); root.style.setProperty('--bg-overlay-light', '0.16'); }
    else { root.style.setProperty('--bg-overlay', '0.50'); root.style.setProperty('--bg-overlay-light', '0.22'); }
  }
  function ensureBgBuffers(){
    if(!bgLayer || bgPrimary) return;
    // use existing as primary
    bgPrimary = bgLayer;
    // create buffer layer after primary
    const buf = document.createElement('div');
    buf.id = 'bgLayer2';
    buf.className = bgLayer.className + ' fade'; // start hidden
    buf.setAttribute('aria-hidden', 'true');
    bgLayer.parentNode.insertBefore(buf, bgLayer.nextSibling);
    bgBuffer = buf;
  }

  function preloadImage(url){
    return new Promise((resolve, reject)=>{
      const img = new Image();
      img.referrerPolicy = 'no-referrer';
      img.crossOrigin = 'anonymous';
      img.onload = ()=> resolve({img, url});
      img.onerror = reject;
      img.src = url;
    });
  }

  async function loadNextWithRetry(maxTries){
    let tries = Math.max(1, maxTries||BG_PROVIDERS.length);
    while(tries-- > 0){
      try {
        const url = nextUrl();
        const res = await preloadImage(url);
        return res; // {img, url}
      } catch(e) {
        // try next provider
      }
    }
    // last resort: local fallback
    try {
      return await preloadImage(FALLBACK_BG);
    } catch(e){
      throw new Error('All providers failed, fallback failed');
    }
  }

  function startProactivePreload(){
    if(preloaded) return; // already have one
    loadNextWithRetry().then(res=>{ preloaded = res; }).catch(()=>{/* ignore */});
  }

  function crossfadeTo(img, url){
    if(!bgPrimary || !bgBuffer) return;
    // set next background on buffer (hidden)
    bgBuffer.style.backgroundImage = `url('${url}')`;
    // analyze tone with the loaded image to adapt overlay
    const tone = analyzeTone(img); applyTone(tone||'dark');
    // force reflow to ensure transition applies
    void bgBuffer.offsetWidth;
    // swap visibility: primary -> fade out, buffer -> fade in
    bgPrimary.classList.add('fade');
    bgBuffer.classList.remove('fade');

    const oldPrimary = bgPrimary;
    const newPrimary = bgBuffer;
    // after transition, swap roles
    const onDone = ()=>{
      newPrimary.removeEventListener('transitionend', onDone);
      // make old primary the new buffer (hidden)
      oldPrimary.classList.add('fade');
      bgPrimary = newPrimary;
      bgBuffer = oldPrimary;
      switching = false;
      if(queued){ queued = false; updateBg(); }
      // proactively preload the next image
      preloaded = null; startProactivePreload();
    };
    newPrimary.addEventListener('transitionend', onDone, { once: true });
    // safety net: if transitionend doesn't fire
    setTimeout(()=>{ if(switching){ onDone(); } }, 700);
  }

  async function updateBg(){
    ensureBgBuffers();
    if(switching){ queued = true; return; }
    switching = true;
    try {
      const res = preloaded || await loadNextWithRetry();
      preloaded = null; // will be replenished after switch
      crossfadeTo(res.img, res.url);
    } catch(e){
      switching = false;
      // hard fallback: show local image on primary immediately
      if(bgPrimary){
        bgPrimary.style.backgroundImage = `url('${FALLBACK_BG}')`;
        bgPrimary.classList.remove('fade');
      }
      startProactivePreload();
    }
  }

  function applyBgInterval(seconds){ if(bgTimer){ clearInterval(bgTimer); bgTimer = null; } localStorage.setItem('dove-bg-interval', String(seconds||0)); if(seconds>0){ bgTimer = setInterval(updateBg, seconds*1000); } }

  function applyBgBlur(px){ const n = Math.max(0, Number(px)||0); document.documentElement.style.setProperty('--bg-blur', `${n}px`); localStorage.setItem('dove-bg-blur', String(n)); }

  // Init interval UI
  (function initBg(){ if(!bgLayer) return; // initial background
    // setup double buffer and default tone
    ensureBgBuffers();
    applyTone('dark');
    // initial load
    updateBg();
    if(bgNextBtn){ bgNextBtn.addEventListener('click', ()=> updateBg()); }
    if(bgIntervalSel){ const saved = parseInt(localStorage.getItem('dove-bg-interval')||'0',10); if(!isNaN(saved)){ bgIntervalSel.value = String(saved); applyBgInterval(saved); } bgIntervalSel.addEventListener('change', ()=>{ const val = parseInt(bgIntervalSel.value||'0',10); applyBgInterval(isNaN(val)?0:val); }); }
    if(bgBlurSel){ const savedBlur = parseInt(localStorage.getItem('dove-bg-blur')||'12',10); const v = isNaN(savedBlur) ? 12 : savedBlur; applyBgBlur(v); bgBlurSel.value = String(v); bgBlurSel.addEventListener('change', ()=>{ const val = parseInt(bgBlurSel.value||'12',10); applyBgBlur(isNaN(val)?12:val); }); }
    // proactively warm up next image
    startProactivePreload();
  })();
})();
