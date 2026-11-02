// Kept out of index.html so the Content-Security-Policy (see build-web.sh)
// can forbid inline scripts.

// The bundle's unused networking plugin assigns to an undeclared
// register_plugin, which strict mode rejects with an error.
var register_plugin;

// Waits for the plugin scripts after the bundle to register themselves before
// the game loads; this file runs first because of the declaration above.
document.addEventListener("DOMContentLoaded", async function () {
  // Download the game ourselves to show progress, then hand it to
  // macroquad's loader. build-web.sh writes in the file's real size, since
  // a compressing web host's Content-Length is the compressed one.
  const total = Number("__WASM_BYTES__") || 0;
  const overlay = document.getElementById("loading");
  const bar = overlay.querySelector(".bar");
  const fill = overlay.querySelector(".fill");
  const status = overlay.querySelector(".status");
  let source = "carmen.wasm";
  try {
    const response = await fetch(source);
    const reader = response.body.getReader();
    const chunks = [];
    let got = 0;
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      chunks.push(value);
      got += value.length;
      if (total) {
        const percent = Math.min(100, Math.round((got * 100) / total));
        fill.style.width = percent + "%";
        bar.setAttribute("aria-valuenow", percent);
        status.textContent = "Loading… " + percent + "%";
      } else {
        status.textContent = "Loading… " + (got / 1048576).toFixed(1) + " MB";
      }
    }
    source = URL.createObjectURL(new Blob(chunks, { type: "application/wasm" }));
  } catch (e) {
    // No streaming (very old browser): let the loader fetch it plainly.
  }
  fill.style.width = "100%";
  bar.setAttribute("aria-valuenow", 100);
  status.textContent = "Starting…";
  load(source);
  // The loader sets wasm_exports once the game is running.
  const started = setInterval(function () {
    if (typeof wasm_exports !== "undefined" && wasm_exports) {
      clearInterval(started);
      overlay.remove();
      document.getElementById("glcanvas").focus();
    }
  }, 50);
});
