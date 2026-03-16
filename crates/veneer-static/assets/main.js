// Veneer Docs - Runtime JavaScript
(function() {
  'use strict';

  // Mobile menu toggle
  const menuBtn = document.querySelector('.menu-btn');
  const sidebar = document.querySelector('.sidebar');

  if (menuBtn && sidebar) {
    menuBtn.addEventListener('click', () => {
      sidebar.classList.toggle('open');
    });
  }

  // Highlight current nav item
  const currentPath = window.location.pathname;
  const navLinks = document.querySelectorAll('.nav-item a');

  navLinks.forEach(link => {
    const href = link.getAttribute('href');
    if (href === currentPath || (currentPath.startsWith(href) && href !== '/')) {
      link.parentElement.classList.add('active');
    }
  });

  // Copy code button for pre blocks
  document.querySelectorAll('main pre').forEach(pre => {
    // Skip if already has a copy button
    if (pre.querySelector('.copy-btn')) return;

    const btn = document.createElement('button');
    btn.className = 'copy-btn';
    btn.textContent = 'Copy';
    btn.setAttribute('type', 'button');

    btn.addEventListener('click', async () => {
      const code = pre.querySelector('code');
      const text = code ? code.textContent : pre.textContent;

      try {
        await navigator.clipboard.writeText(text || '');
        btn.textContent = 'Copied!';
        setTimeout(() => { btn.textContent = 'Copy'; }, 2000);
      } catch (err) {
        btn.textContent = 'Error';
        setTimeout(() => { btn.textContent = 'Copy'; }, 2000);
      }
    });

    pre.appendChild(btn);
  });
})();
