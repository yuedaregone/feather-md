/* ========================================
   FeatherMD - Frontend Application Logic
   ======================================== */

(function () {
  'use strict';

  // ---- State ----
  let currentFilePath = '';
  let rawMarkdown = '';
  let currentTheme = 'github-light';
  let zoomLevel = 2; // 0-6, default 16px
  const ZOOM_SIZES = [12, 14, 16, 18, 20, 22, 24];
  const THEMES = ['github-light', 'github-dark', 'one-dark', 'nord'];
  let mermaidLoaded = false;
  let mermaidInitializing = false;

  // ---- Markdown Rendering ----

  function initMarked() {
    const renderer = new marked.Renderer();

    // Custom code block rendering
    renderer.code = function (code, language) {
      // Handle mermaid diagrams
      if (language === 'mermaid') {
        return '<div class="mermaid">' + escapeHtml(code) + '</div>';
      }

      // Syntax highlighting with highlight.js
      let highlighted;
      if (language && hljs.getLanguage(language)) {
        try {
          highlighted = hljs.highlight(code, { language: language }).value;
        } catch (e) {
          highlighted = escapeHtml(code);
        }
      } else {
        highlighted = escapeHtml(code);
      }

      const langLabel = language ? `<span class="code-lang">${escapeHtml(language)}</span>` : '';
      return `<div class="code-block-wrapper">${langLabel}<pre><code class="hljs language-${escapeHtml(language || '')}">${highlighted}</code></pre><button class="copy-btn" onclick="window.__feather_copy_code(this)">复制</button></div>`;
    };

    // Custom image rendering - resolve relative paths via custom protocol
    renderer.image = function (href, title, text) {
      let src = href;
      // Convert relative paths to feather://app/local/ URLs
      // Rust backend serves local files from the markdown file's directory
      if (currentFilePath && !href.startsWith('http') && !href.startsWith('data:') && !href.startsWith('feather://')) {
        src = 'feather://app/local/' + href.replace(/\\/g, '/');
      }
      const titleAttr = title ? ` title="${escapeHtml(title)}"` : '';
      return `<img src="${src}" alt="${escapeHtml(text)}"${titleAttr} loading="lazy">`;
    };

    marked.setOptions({
      renderer: renderer,
      gfm: true,
      breaks: false,
    });
  }

  function renderMarkdown(content) {
    rawMarkdown = content;
    const html = marked.parse(content);
    document.getElementById('content').innerHTML = html;

    // Render mermaid diagrams
    renderMermaidDiagrams();
  }

  // ---- Mermaid Rendering ----

  async function loadMermaid() {
    if (mermaidLoaded || mermaidInitializing) return;
    mermaidInitializing = true;

    return new Promise((resolve) => {
      const script = document.createElement('script');
      script.src = 'https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.min.js';
      script.onload = function () {
        mermaid.initialize({
          startOnLoad: false,
          theme: currentTheme.includes('dark') ? 'dark' : 'default',
          securityLevel: 'loose',
        });
        mermaidLoaded = true;
        mermaidInitializing = false;
        resolve();
      };
      script.onerror = function () {
        mermaidInitializing = false;
        resolve();
      };
      document.head.appendChild(script);
    });
  }

  async function renderMermaidDiagrams() {
    const mermaidDivs = document.querySelectorAll('.mermaid');
    if (mermaidDivs.length === 0) return;

    // Load mermaid on demand
    if (!mermaidLoaded) {
      await loadMermaid();
    }

    if (!mermaidLoaded) {
      // Failed to load, show raw code
      mermaidDivs.forEach(function (el) {
        el.innerHTML = '<pre><code>' + el.textContent + '</code></pre><p style="color:var(--quote-color);font-size:0.85em;">⚠ Mermaid 加载失败，需要网络连接</p>';
        el.classList.remove('mermaid');
      });
      return;
    }

    try {
      // mermaid v10+ API
      await mermaid.run({ nodes: mermaidDivs });
    } catch (e) {
      console.warn('Mermaid render error:', e);
    }
  }

  // ---- Theme Management ----

  function setTheme(themeName) {
    if (!THEMES.includes(themeName)) return;
    currentTheme = themeName;

    const link = document.getElementById('theme-link');
    link.href = 'themes/' + themeName + '.css';

    // Notify Rust to save theme
    if (window.__feather_ipc) {
      window.__feather_ipc.postMessage(JSON.stringify({ type: 'theme-changed', theme: themeName }));
    }
  }

  function cycleTheme() {
    const idx = THEMES.indexOf(currentTheme);
    const next = THEMES[(idx + 1) % THEMES.length];
    setTheme(next);
  }

  // ---- Zoom ----

  function setZoom(level) {
    zoomLevel = Math.max(0, Math.min(6, level));
    document.body.style.fontSize = ZOOM_SIZES[zoomLevel] + 'px';
    sendToRust({ type: 'zoom-changed', level: zoomLevel });
  }

  function zoomIn() { setZoom(zoomLevel + 1); }
  function zoomOut() { setZoom(zoomLevel - 1); }
  function zoomReset() { setZoom(2); }

  // ---- Context Menu ----

  function showContextMenu(e) {
    e.preventDefault();
    const menu = document.getElementById('context-menu');

    // Make menu visible but hidden to measure its dimensions
    menu.style.visibility = 'hidden';
    menu.classList.remove('hidden');

    const menuWidth = menu.offsetWidth;
    const menuHeight = menu.offsetHeight;
    const windowWidth = window.innerWidth;
    const windowHeight = window.innerHeight;

    let x = e.clientX;
    let y = e.clientY;

    // Adjust position if it overflows
    if ((x + menuWidth) > windowWidth) {
      x = windowWidth - menuWidth - 5;
    }
    if ((y + menuHeight) > windowHeight) {
      y = windowHeight - menuHeight - 5;
    }

    menu.style.left = x + 'px';
    menu.style.top = y + 'px';
    menu.style.visibility = 'visible';
  }

  function hideContextMenu() {
    const menu = document.getElementById('context-menu');
    menu.classList.add('hidden');
    menu.style.visibility = 'hidden';
  }

  function handleMenuAction(action, data) {
    switch (action) {
      case 'copy':
        document.execCommand('copy');
        break;
      case 'copy-md':
        copyToClipboard(rawMarkdown);
        break;
      case 'theme':
        setTheme(data);
        break;
      case 'zoom-in':
        zoomIn();
        break;
      case 'zoom-out':
        zoomOut();
        break;
      case 'zoom-reset':
        zoomReset();
        break;
      case 'open-folder':
        sendToRust({ type: 'open-folder', path: currentFilePath });
        break;
      case 'open-editor':
        sendToRust({ type: 'open-editor', path: currentFilePath });
        break;
    }
    hideContextMenu();
  }

  // ---- Keyboard Shortcuts ----

  function handleKeyDown(e) {
    // Ctrl+O: Open file
    if (e.ctrlKey && e.key === 'o') {
      e.preventDefault();
      sendToRust({ type: 'open-file' });
    }
    // Ctrl+T: Cycle theme
    if (e.ctrlKey && e.key === 't') {
      e.preventDefault();
      cycleTheme();
    }
    // Ctrl+P: Print
    if (e.ctrlKey && e.key === 'p') {
      e.preventDefault();
      window.print();
    }
    // Ctrl+Plus: Zoom in
    if (e.ctrlKey && (e.key === '+' || e.key === '=')) {
      e.preventDefault();
      zoomIn();
    }
    // Ctrl+Minus: Zoom out
    if (e.ctrlKey && e.key === '-') {
      e.preventDefault();
      zoomOut();
    }
    // Ctrl+0: Reset zoom
    if (e.ctrlKey && e.key === '0') {
      e.preventDefault();
      zoomReset();
    }
    // F5: Refresh
    if (e.key === 'F5') {
      e.preventDefault();
      if (currentFilePath) {
        sendToRust({ type: 'reload' });
      }
    }
    // Ctrl+Shift+C: Copy current code block
    if (e.ctrlKey && e.shiftKey && e.key === 'C') {
      e.preventDefault();
      // Find nearest code block
      const sel = window.getSelection();
      if (sel.rangeCount) {
        let node = sel.anchorNode;
        while (node) {
          if (node.tagName === 'PRE' || (node.querySelector && node.querySelector('pre'))) {
            const code = node.textContent || node.querySelector('pre').textContent;
            copyToClipboard(code);
            break;
          }
          node = node.parentNode;
        }
      }
    }
    // Escape: Exit
    if (e.key === 'Escape') {
      sendToRust({ type: 'exit' });
    }
  }

  // ---- Error Page ----

  function showError(title, message) {
    document.getElementById('content').style.display = 'none';
    document.getElementById('error-title').textContent = title;
    document.getElementById('error-message').textContent = message;
    document.getElementById('error-page').classList.remove('hidden');
  }

  function hideError() {
    document.getElementById('content').style.display = '';
    document.getElementById('error-page').classList.add('hidden');
  }

  // ---- IPC with Rust ----

  function sendToRust(msg) {
    if (window.__feather_ipc) {
      window.__feather_ipc.postMessage(JSON.stringify(msg));
    }
  }

  // Expose for Rust → JS communication
  window.__feather_render = function (content, filePath) {
    hideError();
    currentFilePath = filePath || '';
    renderMarkdown(content);
    document.title = (filePath ? filePath.split(/[\\/]/).pop() : 'Untitled') + ' - FeatherMD';
  };

  window.__feather_set_theme = function (theme) {
    setTheme(theme);
  };

  window.__feather_set_zoom = setZoom;

  window.__feather_refresh = function (content) {
    renderMarkdown(content);
  };

  window.__feather_error = function (title, message) {
    showError(title, message);
  };

  window.__feather_open_file = function () {
    sendToRust({ type: 'open-file' });
  };

  window.__feather_copy_code = function (btn) {
    const code = btn.previousElementSibling.querySelector('code');
    if (code) {
      copyToClipboard(code.textContent);
      btn.textContent = '已复制!';
      setTimeout(function () { btn.textContent = '复制'; }, 1500);
    }
  };

  // ---- Utilities ----

  function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
  }

  function copyToClipboard(text) {
    navigator.clipboard.writeText(text).catch(function () {
      // Fallback
      const ta = document.createElement('textarea');
      ta.value = text;
      ta.style.position = 'fixed';
      ta.style.left = '-9999px';
      document.body.appendChild(ta);
      ta.select();
      document.execCommand('copy');
      document.body.removeChild(ta);
    });
  }

  // ---- Event Listeners ----

  document.addEventListener('contextmenu', showContextMenu);
  document.addEventListener('click', function (e) {
    // Handle context menu clicks
    if (e.target.closest('.menu-item')) {
      const item = e.target.closest('.menu-item');
      const action = item.dataset.action;
      const data = item.dataset.theme;
      if (action) handleMenuAction(action, data);
      return;
    }
    hideContextMenu();
  });
  document.addEventListener('keydown', handleKeyDown);

  // Close menu on scroll
  document.addEventListener('scroll', hideContextMenu);

  // ---- Init ----
  initMarked();

  // Show welcome if no file loaded
  document.getElementById('content').innerHTML = '<div style="text-align:center;padding:4rem 1rem;color:var(--quote-color);"><h2 style="border:none;padding:0;color:var(--text-color);">FeatherMD</h2><p>极致轻量的 Markdown 查看器</p><p style="margin-top:2rem;font-size:0.9em;">双击 .md 文件打开，或将文件拖入此窗口</p></div>';

  // Let the backend know the frontend is ready
  sendToRust({ type: 'frontend-ready' });

})();
