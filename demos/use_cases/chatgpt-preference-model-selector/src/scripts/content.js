// content.js

(() => {
  const TAG = '[ModelSelector]';

  /**─────────────────────── 0️⃣ Broadcast initial settings ───────────────────────**/
  chrome.storage.sync.get(['preferences','defaultModel'], settings => {
    window.postMessage({ type: 'PBMS_SETTINGS', settings }, '*');
  });
  chrome.storage.onChanged.addListener(() => {
    chrome.storage.sync.get(['preferences','defaultModel'], settings => {
      window.postMessage({ type: 'PBMS_SETTINGS', settings }, '*');
    });
  });

  /**─────────────────────── 1️⃣ Inject page-context fetch override ───────────────────────**/
  (function injectPageFetchOverride() {
    const injectorTag = '[ModelSelector][Injector]';
    const s = document.createElement('script');
    s.src = chrome.runtime.getURL('pageFetchOverride.js');
    s.onload = () => {
      console.log(`${injectorTag} loaded pageFetchOverride.js`);
      s.remove();
    };
    (document.head || document.documentElement).appendChild(s);
  })();

  /**─────────────────────── 2️⃣ Handle proxied fetch from the page ───────────────────────**/
  window.addEventListener('message', ev => {
    console.log('[ModelSelector] page→content message', ev.data, ev.ports);

    if (ev.source !== window || ev.data?.type !== 'ARCHGW_FETCH') return;

    const { url, init, originalRequestUrl } = ev.data;
    const port = ev.ports[0];

    (async () => {
      try {
        console.log('[ModelSelector] Fetching model recommendation from local proxy...');
        //const res = await fetch(url, init);
        //const json = await res.json();

        //console.log('[ModelSelector] Proxy responded with:', json);

        const targetModel = 'o4-mini-high';
        if (!targetModel) {
          console.warn('[ModelSelector] No model returned from proxy, using default fetch');
          port.postMessage({ done: true });
          return;
        }

        // ✅ Extract the original fetch request body from init.body
        let originalBody = {};
        try {
          originalBody = JSON.parse(init.body);
        } catch {
          console.warn('[ModelSelector] Could not parse original fetch body');
        }

        // ✅ Patch the model in the request
        originalBody.model = targetModel;

        console.log(`[ModelSelector] Updating model in request → ${targetModel}`);

        // ✅ Resume the request to the real backend
        const upstreamRes = await fetch('/backend-api/conversation', {
          method: init.method,
          headers: init.headers,
          credentials: init.credentials,
          body: JSON.stringify(originalBody)
        });

        // ✅ Stream the upstream response back to the page
        const reader = upstreamRes.body.getReader();

        while (true) {
          const { done, value } = await reader.read();
          if (done) {
            port.postMessage({ done: true });
            break;
          } else {
            port.postMessage({ chunk: value.buffer }, [value.buffer]);
          }
        }
      } catch (err) {
        console.error('[ModelSelector] proxy fetch error', err);
        port.postMessage({ done: true });
      }
    })();
  });


  /**─────────────────────── 3️⃣ DOM patch for model selector label ───────────────────────**/
  let desiredModel = null;
  function patchDom() {
    if (desiredModel == null) return;
    const btn = document.querySelector('[data-testid="model-switcher-dropdown-button"]');
    if (!btn) return;
    const span = btn.querySelector('span.text-token-text-tertiary') || btn.querySelector('span');
    const wantLabel = `Model selector, current model is ${desiredModel}`;
    if (span && span.textContent !== desiredModel) span.textContent = desiredModel;
    if (btn.getAttribute('aria-label') !== wantLabel) btn.setAttribute('aria-label', wantLabel);
  }
  const observer = new MutationObserver(patchDom);
  observer.observe(document.body || document.documentElement, {
    subtree: true, childList: true, characterData: true, attributes: true
  });
  chrome.storage.sync.get(['defaultModel'], ({ defaultModel }) => {
    if (defaultModel) { desiredModel = defaultModel; patchDom(); }
  });
  chrome.runtime.onMessage.addListener(msg => {
    if (msg.action === 'applyModelSelection' && msg.model) {
      desiredModel = msg.model;
      patchDom();
    }
  });

  /**─────────────────────── 4️⃣ Modal / dropdown interception ───────────────────────**/
  function showModal() {
    if (document.getElementById('pbms-overlay')) return;
    const overlay = document.createElement('div');
    overlay.id = 'pbms-overlay';
    Object.assign(overlay.style, {
      position:'fixed', top:0, left:0,
      width:'100vw', height:'100vh',
      background:'rgba(0,0,0,0.4)',
      display:'flex', alignItems:'center', justifyContent:'center',
      zIndex:2147483647
    });
    const iframe = document.createElement('iframe');
    iframe.src = chrome.runtime.getURL('index.html');
    Object.assign(iframe.style,{
      width:'500px', height:'600px',
      border:0, borderRadius:'8px',
      boxShadow:'0 4px 16px rgba(0,0,0,0.2)',
      background:'white', zIndex:2147483648
    });
    overlay.addEventListener('click', e => e.target===overlay && overlay.remove());
    overlay.appendChild(iframe);
    document.body.appendChild(overlay);
  }
  function interceptDropdown(ev) {
    if (!ev.target.closest('button[aria-haspopup="menu"]')) return;
    ev.preventDefault(); ev.stopPropagation();
    showModal();
  }
  document.addEventListener('pointerdown', interceptDropdown, true);
  document.addEventListener('mousedown', interceptDropdown, true);
  window.addEventListener('message', ev => {
    if (ev.data?.action === 'CLOSE_PBMS_MODAL') {
      document.getElementById('pbms-overlay')?.remove();
    }
  });

  console.log(`${TAG} content script initialized`);
})();
