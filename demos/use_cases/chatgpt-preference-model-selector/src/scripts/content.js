(() => {
  const TAG = '[ModelSelector]';

  /**─────────────────────── 🔧 Utility: Scrape Current Messages ───────────────────────**/
  function getMessagesFromDom() {
    const bubbles = [...document.querySelectorAll('[data-message-author-role]')];

    return bubbles
      .map(b => {
        const role = b.getAttribute('data-message-author-role');
        const content =
          role === 'assistant'
            ? (b.querySelector('.markdown')?.innerText ?? b.innerText ?? '').trim()
            : (b.innerText ?? '').trim();
        return content ? { role, content } : null;
      })
      .filter(Boolean);
  }

  /**─────────────────────── 🔧 Utility: Prepare Request to Proxy ───────────────────────**/
  function prepareProxyRequest(messages, routes, maxTokenLength = 2048) {
    const SYSTEM_PROMPT_TEMPLATE = `
    You are a helpful assistant designed to find the best suited route.
    You are provided with route description within <routes></routes> XML tags:
    <routes>
    {routes}
    </routes>

    <conversation>
    {conversation}
    </conversation>

    Your task is to decide which route is best suit with user intent on the conversation in <conversation></conversation> XML tags.  Follow the instruction:
    1. If the latest intent from user is irrelevant or user intent is full filled, response with other route {"route": "other"}.
    2. You must analyze the route descriptions and find the best match route for user latest intent.
    3. You only response the name of the route that best matches the user's request, use the exact name in the <routes></routes>.

    Based on your analysis, provide your response in the following JSON formats if you decide to match any route:
    {"route": "route_name"}
    `;
    const TOKEN_DIVISOR = 4;

    const filteredMessages = messages.filter(
      m => m.role !== 'system' && m.role !== 'tool' && m.content?.trim()
    );

    let tokenCount = SYSTEM_PROMPT_TEMPLATE.length / TOKEN_DIVISOR;
    const selected = [];

    for (let i = filteredMessages.length - 1; i >= 0; i--) {
      const msg = filteredMessages[i];
      tokenCount += msg.content.length / TOKEN_DIVISOR;

      if (tokenCount > maxTokenLength) {
        if (msg.role === 'user') selected.push(msg);
        break;
      }

      selected.push(msg);
    }

    if (selected.length === 0 && filteredMessages.length > 0) {
      selected.push(filteredMessages[filteredMessages.length - 1]);
    }

    const selectedOrdered = selected.reverse();

    const systemPrompt = SYSTEM_PROMPT_TEMPLATE
      .replace('{routes}', JSON.stringify(routes, null, 2))
      .replace('{conversation}', JSON.stringify(selectedOrdered, null, 2));

    return systemPrompt;
  }

  /**─────────────────────── 🔧 Get routes from storage ───────────────────────**/
  function getRoutesFromStorage() {
    return new Promise(resolve => {
      chrome.storage.sync.get(['preferences'], ({ preferences }) => {
        if (!preferences || !Array.isArray(preferences)) {
          console.warn('[ModelSelector] No preferences found in storage');
          return resolve([]);
        }

        const routes = preferences.map(p => ({
          name: p.name,
          description: p.usage
        }));

        resolve(routes);
      });
    });
  }


  /**─────────────────────── 🔧 Get model ID by route name ───────────────────────**/
  function getModelIdForRoute(routeName) {
    return new Promise(resolve => {
      chrome.storage.sync.get(['preferences'], ({ preferences }) => {
        const match = (preferences || []).find(p => p.name === routeName);
        if (match) resolve(match.model);
        else resolve(null);
      });
    });
  }

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

  /**─────────────────────── 2️⃣ Intercept fetch and reroute via Ollama ───────────────────────**/
  window.addEventListener('message', ev => {
    if (ev.source !== window || ev.data?.type !== 'ARCHGW_FETCH') return;

    const { url, init } = ev.data;
    const port = ev.ports[0];

    (async () => {
      try {
        console.log(`${TAG} Intercepted fetch from page:`, url);

        let originalBody = {};
        try {
          originalBody = JSON.parse(init.body);
        } catch {
          console.warn(`${TAG} Could not parse original fetch body`);
        }

        const scrapedMessages = getMessagesFromDom();
        const routes = await getRoutesFromStorage();
        const prompt = prepareProxyRequest(scrapedMessages, routes);

        // 🔁 Call Ollama router
        let selectedRoute = null;
        try {
          const res = await fetch('http://localhost:11434/api/generate', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
              model: 'hf.co/katanemo/Arch-Router-1.5B.gguf:Q4_K_M',
              prompt: prompt,
              temperature: 0.1,
              stream: false
            })
          });

        if (res.ok) {
            const data = await res.json();
            console.log(`${TAG} Ollama router response:`, data.response);
            try {
            let parsed = data.response;
            if (typeof data.response === 'string') {
              try {
                parsed = JSON.parse(data.response);
              } catch (jsonErr) {
                // Try to recover from single quotes
                const safe = data.response.replace(/'/g, '"');
                parsed = JSON.parse(safe);
              }
            }
            selectedRoute = parsed.route || null;
            if (!selectedRoute) console.warn(`${TAG} Route missing in parsed response`);
          } catch (e) {
            console.warn(`${TAG} Failed to parse or extract route from response`, e);
          }
          }
          else {
            console.warn(`${TAG} Ollama router failed:`, res.status);
          }
        } catch (err) {
          console.error(`${TAG} Ollama request error`, err);
        }

        let targetModel = null;
        if (selectedRoute) {
          targetModel = await getModelIdForRoute(selectedRoute);
          console.log(`${TAG} Resolved model for route "${selectedRoute}" →`, targetModel);
        }

        // 🧠 Replace model if we found one
        const modifiedBody = { ...originalBody };
        if (targetModel) {
          modifiedBody.model = targetModel;
          console.log(`${TAG} Overriding request with model: ${targetModel}`);
        } else {
          console.warn(`${TAG} No route/model override applied`);
        }

        const upstreamRes = await fetch(url, {
          method: init.method,
          headers: init.headers,
          credentials: init.credentials,
          body: JSON.stringify(modifiedBody)
        });

        const reader = upstreamRes.body.getReader();
        while (true) {
          const { done, value } = await reader.read();
          if (done) {
            port.postMessage({ done: true });
            break;
          }
          port.postMessage({ chunk: value.buffer }, [value.buffer]);
        }
      } catch (err) {
        console.error(`${TAG} Proxy fetch error`, err);
        port.postMessage({ done: true });
      }
    })();
  });

  /**─────────────────────── 3️⃣ DOM patch for model selector label ───────────────────────**/
  let desiredModel = null;
  function patchDom() {
    if (!desiredModel) return;

    const btn = document.querySelector('[data-testid="model-switcher-dropdown-button"]');
    if (!btn) return;

    const span = btn.querySelector('div > span');
    const wantLabel = `Model selector, current model is ${desiredModel}`;

    if (span && span.textContent !== desiredModel) span.textContent = desiredModel;
    if (btn.getAttribute('aria-label') !== wantLabel) {
      btn.setAttribute('aria-label', wantLabel);
    }
  }

  const observer = new MutationObserver(patchDom);
  observer.observe(document.body || document.documentElement, {
    subtree: true, childList: true, characterData: true, attributes: true
  });

  chrome.storage.sync.get(['defaultModel'], ({ defaultModel }) => {
    if (defaultModel) {
      desiredModel = defaultModel;
      patchDom();
    }
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
      position: 'fixed', top: 0, left: 0,
      width: '100vw', height: '100vh',
      background: 'rgba(0,0,0,0.4)',
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      zIndex: 2147483647
    });
    const iframe = document.createElement('iframe');
    iframe.src = chrome.runtime.getURL('index.html');
    Object.assign(iframe.style, {
      width: '500px', height: '600px',
      border: 0, borderRadius: '8px',
      boxShadow: '0 4px 16px rgba(0,0,0,0.2)',
      background: 'white', zIndex: 2147483648
    });
    overlay.addEventListener('click', e => e.target === overlay && overlay.remove());
    overlay.appendChild(iframe);
    document.body.appendChild(overlay);
  }

  function interceptDropdown(ev) {
    if (!ev.target.closest('button[aria-haspopup="menu"]')) return;
    ev.preventDefault();
    ev.stopPropagation();
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
