// background.js
chrome.runtime.onMessage.addListener((msg, sender) => {
  if (msg.type === 'ARCHGW_STREAM') {
    const { url, init, port } = msg;

    fetch(url, {
      method: init.method || 'POST',
      headers: init.headers,
      body: init.body,
      credentials: init.credentials ?? 'same-origin',
      // ...copy other init options as needed
    }).then(res => {
      const reader = res.body.getReader();
      function read() {
        reader.read().then(({ done, value }) => {
          if (done) {
            port.postMessage({ done: true });
            port.close();
          } else {
            // Send each chunk (Uint8Array) to the content script
            port.postMessage({ chunk: value.buffer }, [value.buffer]);
            read();
          }
        });
      }
      read();
    }).catch(err => {
      // In case of error, signal done
      port.postMessage({ done: true });
      port.close();
    });

    // Indicate we’re handling this asynchronously
    return true;
  }
});
