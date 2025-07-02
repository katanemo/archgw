// pageFetchOverride.js
(function() {
  const TAG = '[ModelSelector][Page]';
  console.log(`${TAG} installing fetch override`);

  window.archgwSettings = window.archgwSettings || { preferences: [], defaultModel: null };

  window.addEventListener('message', ev => {
    if (ev.source === window && ev.data?.type === 'PBMS_SETTINGS') {
      window.archgwSettings = ev.data.settings;
      console.log(`${TAG} got updated settings`, window.archgwSettings);
    }
  });

  // New function: scrape current messages from the DOM
  function get_messages() {
    const bubbles = [...document.querySelectorAll('[data-message-author-role]')];

    const messages = bubbles
      .map(b => {
        const role = b.getAttribute('data-message-author-role'); // "user" | "assistant"
        const content =
          role === 'assistant'
            ? (b.querySelector('.markdown')?.innerText ?? b.innerText ?? '').trim()
            : (b.innerText ?? '').trim();
        return content ? { role, content } : null;
      })
      .filter(Boolean);

    return { messages };
  }

  const origFetch = window.fetch;
  window.fetch = async function(input, init = {}) {
    const urlString = typeof input === 'string' ? input : input.url;
    console.log(`${TAG} fetch →`, urlString);

    let pathname;
    try {
      pathname = new URL(urlString).pathname;
    } catch {
      pathname = urlString;
    }

    if (pathname === '/backend-api/conversation') {
      console.log(`${TAG} matched conversation → proxy via content script`);

      let body = {};
      try { body = JSON.parse(init.body); } catch {}

      const currentMessages = get_messages();
      console.log(`${TAG} scraped messages →`, currentMessages);

      // Patch metadata with current preferences
      body.metadata = {
        archgw_preference_config: window.archgwSettings.preferences
          .map(p => `- name: ${p.name}\n  model: ${p.model}\n  usage: ${p.usage}`)
          .join('\n'),

        // Add current messages dynamically
        archgw_current_messages: JSON.stringify(currentMessages)
      };

      init.body = JSON.stringify(body);

      const safeInit = {
        method: init.method,
        headers: init.headers,
        body: init.body,
        credentials: init.credentials
      };

      const { port1, port2 } = new MessageChannel();
      window.postMessage({
        type: 'ARCHGW_FETCH',
        url: 'http://localhost:12000/v1/chat/completions',
        init: safeInit
      }, '*', [port2]);

      return new Response(new ReadableStream({
        start(controller) {
          port1.onmessage = ({ data }) => {
            if (data.done) {
              controller.close();
              port1.close();
            } else {
              controller.enqueue(new Uint8Array(data.chunk));
            }
          };
        },
        cancel() {
          port1.close();
        }
      }), {
        headers: { 'Content-Type': 'text/event-stream' }
      });
    }

    return origFetch(input, init);
  };

  console.log(`${TAG} fetch override installed`);
})();
