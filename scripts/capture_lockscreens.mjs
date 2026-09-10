import fs from "fs";

const ARTIFACTS_DIR = "/home/silentbyte/.gemini/antigravity-ide/brain/50ef2876-38e5-4b46-b545-a5640a15a918";

async function run() {
  // Create a new tab
  const tabRes = await fetch("http://127.0.0.1:9222/json/new?http://127.0.0.1:1420/", { method: "PUT" });
  const tab = await tabRes.json();
  console.log("Tab created:", tab.id, tab.webSocketDebuggerUrl);

  const ws = new WebSocket(tab.webSocketDebuggerUrl);
  let id = 1;
  const pending = new Map();

  function send(method, params = {}) {
    return new Promise((resolve, reject) => {
      const msgId = id++;
      pending.set(msgId, { resolve, reject });
      ws.send(JSON.stringify({ id: msgId, method, params }));
    });
  }

  ws.onmessage = (event) => {
    const data = JSON.parse(event.data);
    if (data.id && pending.has(data.id)) {
      const { resolve, reject } = pending.get(data.id);
      pending.delete(data.id);
      if (data.error) reject(data.error);
      else resolve(data.result);
    }
  };

  await new Promise((res) => (ws.onopen = res));
  console.log("Connected to CDP WebSocket");

  await send("Page.enable");
  await send("Runtime.enable");

  // Set initial viewport: 1600x1000
  await send("Emulation.setDeviceMetricsOverride", {
    width: 1600,
    height: 1000,
    deviceScaleFactor: 1,
    mobile: false,
  });

  // Wait for React to render
  await new Promise((r) => setTimeout(r, 2000));

  // Dismiss first-run banner if present and click on Lock Screens tab
  const res = await send("Runtime.evaluate", {
    expression: `
      (() => {
        // 1. Dismiss banner if close button exists
        const closeBtn = document.querySelector('button[aria-label="Dismiss banner"]');
        if (closeBtn) closeBtn.click();

        // 2. Find and click the "Lock Screens" tab
        const buttons = Array.from(document.querySelectorAll('button'));
        const lockTab = buttons.find(b => b.textContent && b.textContent.trim() === 'Lock Screens');
        if (lockTab) {
          lockTab.click();
          return "Clicked Lock Screens tab";
        }
        return "Lock Screens tab not found among: " + buttons.map(b => b.textContent?.trim()).filter(Boolean).join(", ");
      })()
    `,
  });
  console.log("Tab action:", res.result.value);

  // Wait for Lock Screens tab content to render
  await new Promise((r) => setTimeout(r, 1200));

  // Capture 1: Lock Screens catalogue Dark theme
  const shot1 = await send("Page.captureScreenshot", { format: "png" });
  fs.writeFileSync(`${ARTIFACTS_DIR}/ryzora_lockscreens_catalogue_dark.png`, Buffer.from(shot1.data, "base64"));
  console.log("Saved ryzora_lockscreens_catalogue_dark.png");

  // Test Filter Subcategory: Click "SDDM"
  await send("Runtime.evaluate", {
    expression: `
      (() => {
        const buttons = Array.from(document.querySelectorAll('button'));
        const sddmBtn = buttons.find(b => b.textContent && b.textContent.includes('SDDM'));
        if (sddmBtn) {
          sddmBtn.click();
          return "Clicked SDDM subfilter";
        }
        return "SDDM button not found";
      })()
    `,
  });
  await new Promise((r) => setTimeout(r, 600));

  const shotSddm = await send("Page.captureScreenshot", { format: "png" });
  fs.writeFileSync(`${ARTIFACTS_DIR}/ryzora_lockscreens_sddm_filter.png`, Buffer.from(shotSddm.data, "base64"));
  console.log("Saved ryzora_lockscreens_sddm_filter.png");

  // Click back to "All" subfilter
  await send("Runtime.evaluate", {
    expression: `
      (() => {
        const buttons = Array.from(document.querySelectorAll('button'));
        // Find the "All" button in the subfilter row (second "All" after main tabs)
        const allBtns = buttons.filter(b => b.textContent && b.textContent.trim() === 'All');
        if (allBtns.length > 1) {
          allBtns[1].click();
          return "Clicked subfilter All";
        } else if (allBtns.length === 1) {
          allBtns[0].click();
          return "Clicked All";
        }
        return "All button not found";
      })()
    `,
  });
  await new Promise((r) => setTimeout(r, 600));

  // Toggle to Light Theme
  await send("Runtime.evaluate", {
    expression: `
      (() => {
        // Theme toggle button in top bar
        const buttons = Array.from(document.querySelectorAll('button'));
        const themeBtn = buttons.find(b => b.title && b.title.toLowerCase().includes('theme')) ||
                         buttons.find(b => b.innerHTML && (b.innerHTML.includes('Sun') || b.innerHTML.includes('Moon')));
        if (themeBtn) {
          themeBtn.click();
          return "Clicked theme toggle";
        }
        // Fallback: set document root attribute
        document.documentElement.classList.remove('dark');
        document.documentElement.classList.add('light');
        document.documentElement.setAttribute('data-theme', 'light');
        return "Switched theme class manually";
      })()
    `,
  });
  await new Promise((r) => setTimeout(r, 800));

  const shotLight = await send("Page.captureScreenshot", { format: "png" });
  fs.writeFileSync(`${ARTIFACTS_DIR}/ryzora_lockscreens_catalogue_light.png`, Buffer.from(shotLight.data, "base64"));
  console.log("Saved ryzora_lockscreens_catalogue_light.png");

  // Switch back to Dark theme and test narrow 2-column viewport (e.g. 580px width)
  await send("Runtime.evaluate", {
    expression: `
      (() => {
        document.documentElement.classList.remove('light');
        document.documentElement.classList.add('dark');
        document.documentElement.setAttribute('data-theme', 'dark');
      })()
    `,
  });

  await send("Emulation.setDeviceMetricsOverride", {
    width: 580,
    height: 900,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await new Promise((r) => setTimeout(r, 600));

  const shotNarrow = await send("Page.captureScreenshot", { format: "png" });
  fs.writeFileSync(`${ARTIFACTS_DIR}/ryzora_lockscreens_narrow_2col.png`, Buffer.from(shotNarrow.data, "base64"));
  console.log("Saved ryzora_lockscreens_narrow_2col.png");

  // Close tab
  await send("Page.close");
  ws.close();
  console.log("All screenshots captured successfully!");
}

run().catch(console.error);
