// This Vite plugin is loaded only by test tools, never by vite.config.ts.
// It connects the browser UI to the real Rust core's disposable test database.
export const coreTestTransport = {
  name: "test-only-core-transport",
  transformIndexHtml() {
    return [
      {
        tag: "script",
        injectTo: "head-prepend",
        children: `
      window.isTauri = true;
      window.__TAURI_INTERNALS__ = {
        invoke: async (cmd, args = {}) => {
          const response = await fetch('http://127.0.0.1:1421/invoke', {
            method: 'POST', headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ cmd, args })
          });
          const payload = await response.json();
          if (payload.error) throw new Error(payload.error);
          return payload.value;
        }
      };
    `,
      },
    ];
  },
};
