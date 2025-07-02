(function() {
  const TAG = '[ModelSelector][Page]';
  console.log(`${TAG} installing fetch override`);

  const origFetch = window.fetch;
  window.fetch = async function(input, init = {}) {

    const urlString = typeof input === 'string' ? input : input.url;
    const urlObj = new URL(urlString, window.location.origin);
    const pathname = urlObj.pathname;
    console.log(`${TAG} fetch →`, pathname);

    const method = (init.method || 'GET').toUpperCase();
    if (method === 'OPTIONS') {
      console.log(`${TAG} OPTIONS request → bypassing completely`);
      return origFetch(input, init);
    }

    // Only intercept conversation fetches
    if (pathname === '/backend-api/conversation') {
      console.log(`${TAG} matched → proxy via content script`);

      const { port1, port2 } = new MessageChannel();

      // ✅ Remove non-cloneable properties like 'signal'
      const safeInit = { ...init };
      delete safeInit.signal;

      // Forward the fetch details to the content script
      window.postMessage({
        type: 'ARCHGW_FETCH',
        url: urlString,
        init: safeInit
      }, '*', [port2]);

      // Return a stream response that the content script will fulfill
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

    // Otherwise, pass through to the original fetch
    return origFetch(input, init);
  };

  console.log(`${TAG} fetch override installed`);
})();
