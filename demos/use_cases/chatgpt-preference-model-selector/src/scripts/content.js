(() => {
  const TAG = '[ModelSelector]';
  // Content script to intercept fetch requests and modify them based on user preferences
  async function streamToPort(response, port) {
    const reader = response.body?.getReader();
    if (!reader) {
      port.postMessage({ done: true });
      return;
    }
    while (true) {
      const { done, value } = await reader.read();
      if (done) {
        port.postMessage({ done: true });
        break;
      }
      port.postMessage({ chunk: value.buffer }, [value.buffer]);
    }
  }

  // Extract messages from the DOM, falling back to requestMessages if DOM is empty
  function getMessagesFromDom(requestMessages = null) {
    const bubbles = [...document.querySelectorAll('[data-message-author-role]')];

    const domMessages = bubbles
      .map(b => {
        const role = b.getAttribute('data-message-author-role');
        const content =
          role === 'assistant'
            ? (b.querySelector('.markdown')?.innerText ?? b.innerText ?? '').trim()
            : (b.innerText ?? '').trim();
        return content ? { role, content } : null;
      })
      .filter(Boolean);

    // Fallback: If DOM is empty but we have requestMessages, use those
    if (domMessages.length === 0 && requestMessages?.length > 0) {
      return requestMessages
        .map(msg => {
          const role = msg.author?.role;
          const parts = msg.content?.parts ?? [];
          const textPart = parts.find(p => typeof p === 'string');
          return role && textPart ? { role, content: textPart.trim() } : null;
        })
        .filter(Boolean);
    }

    return domMessages;
  }

  // Insert a route label for the last user message in the chat
  function insertRouteLabelForLastUserMessage(routeName) {
    chrome.storage.sync.get(['preferences'], ({ preferences }) => {
    if (!Array.isArray(preferences)) return;

    const match = preferences.find(p => p.name === routeName);
    if (!match || !match.usage) {
      console.warn('[RouteLabel] No usage found for route:', routeName);
      return;
    }

    const bubbles = [...document.querySelectorAll('[data-message-author-role="user"]')];
    const lastBubble = bubbles[bubbles.length - 1];
    if (!lastBubble) return;

    if (lastBubble.querySelector('.arch-route-label')) {
      console.log('[RouteLabel] Label already exists, skipping');
      return;
    }

    const label = document.createElement('span');
    label.textContent = `RouteGPT preference >> ${match.usage}`;
    label.className = 'arch-route-label';
    label.style.fontWeight = '100';
    label.style.fontSize = '0.85rem';
    label.style.marginTop = '4px';

    lastBubble.appendChild(label);
    console.log('[RouteLabel] Inserted label:', label.textContent);
  });
  }


  // Prepare the system prompt for the proxy request
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

  function getModelIdForRoute(routeName) {
    return new Promise(resolve => {
      chrome.storage.sync.get(['preferences'], ({ preferences }) => {
        const match = (preferences || []).find(p => p.name === routeName);
        if (match) resolve(match.model);
        else resolve(null);
      });
    });
  }

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

        const { routingEnabled, preferences } = await new Promise(resolve => {
          chrome.storage.sync.get(['routingEnabled', 'preferences'], resolve);
        });

        if (!routingEnabled) {
          console.log(`${TAG} Routing disabled — forwarding original request`);
          await streamToPort(await fetch(url, init), port);
          return;
        }

        const scrapedMessages = getMessagesFromDom(originalBody.messages);
        const routes = (preferences || []).map(p => ({
          name: p.name,
          description: p.usage
        }));
        const prompt = prepareProxyRequest(scrapedMessages, routes);

        let selectedRoute = null;
        try {
          const res = await fetch('http://localhost:11434/api/generate', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
              model: 'hf.co/katanemo/Arch-Router-1.5B.gguf:Q4_K_M',
              prompt: prompt,
              temperature: 0.01,
              top_p: 0.95,
              top_k: 20,
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
                  const safe = data.response.replace(/'/g, '"');
                  parsed = JSON.parse(safe);
                }
              }
              selectedRoute = parsed.route || null;
              if (!selectedRoute) console.warn(`${TAG} Route missing in parsed response`);
            } catch (e) {
              console.warn(`${TAG} Failed to parse or extract route from response`, e);
            }
          } else {
            console.warn(`${TAG} Ollama router failed:`, res.status);
          }
        } catch (err) {
          console.error(`${TAG} Ollama request error`, err);
        }

        let targetModel = null;
        if (selectedRoute) {
          targetModel = await getModelIdForRoute(selectedRoute);
          console.log(`${TAG} Resolved model for route "${selectedRoute}" →`, targetModel);
          insertRouteLabelForLastUserMessage(selectedRoute);
        }

        const modifiedBody = { ...originalBody };
        if (targetModel) {
          modifiedBody.model = targetModel;
          console.log(`${TAG} Overriding request with model: ${targetModel}`);
        } else {
          console.warn(`${TAG} No route/model override applied`);
        }

        await streamToPort(await fetch(url, {
          method: init.method,
          headers: init.headers,
          credentials: init.credentials,
          body: JSON.stringify(modifiedBody)
        }), port);
      } catch (err) {
        console.error(`${TAG} Proxy fetch error`, err);
        port.postMessage({ done: true });
      }
    })();
  });

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
    const btn = ev.target.closest('button[data-testid="model-switcher-dropdown-button"]');
    if (!btn) return;

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
