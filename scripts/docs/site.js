// A small local search: no analytics, external service, or background requests.
const field = document.querySelector('#search');
const status = document.querySelector('#search-status');
let current;
field.addEventListener('input', () => {
  if (current) current.classList.remove('match');
  const query = field.value.trim().toLocaleLowerCase();
  const matches = query ? [...document.querySelectorAll('article h2,article h3,article p,article li,article td')].filter(node => node.textContent.toLocaleLowerCase().includes(query)) : [];
  current = matches[0];
  status.textContent = query ? `${matches.length} 件見つかりました。Enterで最初の項目へ移動します。` : '';
});
field.addEventListener('keydown', event => {
  if (event.key === 'Enter' && current) {
    current.classList.add('match');
    current.scrollIntoView({block: 'center'});
  }
});
if ('IntersectionObserver' in window) {
  const observer = new IntersectionObserver(entries => {
    for (const entry of entries) {
      if (!entry.isIntersecting) continue;
      for (const item of document.querySelectorAll('nav a')) item.removeAttribute('aria-current');
      const item = [...document.querySelectorAll('nav a')].find(item => item.hash === '#' + encodeURIComponent(entry.target.id) || decodeURIComponent(item.hash) === '#' + entry.target.id);
      if (item) item.setAttribute('aria-current', 'true');
    }
  }, {rootMargin:'-90px 0px -65% 0px'});
  document.querySelectorAll('article h2').forEach(heading => observer.observe(heading));
}
