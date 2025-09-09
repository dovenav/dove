(function(){
  const q = document.getElementById('q');
  const cards = Array.from(document.querySelectorAll('.card'));
  q && q.addEventListener('input', function(){
    const v = q.value.toLowerCase();
    cards.forEach(c => {
      const t = c.textContent.toLowerCase();
      c.style.display = t.includes(v) ? '' : 'none';
    });
  });
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
})();
