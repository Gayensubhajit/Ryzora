import fs from "fs";

const ARTIFACTS_DIR = "/home/silentbyte/.gemini/antigravity-ide/brain/50ef2876-38e5-4b46-b545-a5640a15a918";

async function run() {
  const tabRes = await fetch("http://127.0.0.1:9222/json/new?http://127.0.0.1:1420/", { method: "PUT" });
  const tab = await tabRes.json();

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

  await send("Page.enable");
  await send("Runtime.enable");

  // Full Desktop 1600x950
  await send("Emulation.setDeviceMetricsOverride", {
    width: 1600,
    height: 950,
    deviceScaleFactor: 1,
    mobile: false,
  });

  await new Promise((r) => setTimeout(r, 1500));

  // Set Dark theme and navigate to Lock Screens
  await send("Runtime.evaluate", {
    expression: `
      (() => {
        document.documentElement.classList.remove('light');
        document.documentElement.classList.add('dark');
        document.documentElement.setAttribute('data-theme', 'dark');

        const buttons = Array.from(document.querySelectorAll('button'));
        const lockTab = buttons.find(b => b.textContent && b.textContent.trim() === 'Lock Screens');
        if (lockTab) lockTab.click();
      })()
    `,
  });

  await new Promise((r) => setTimeout(r, 1000));

  // Capture Full Desktop Dark Theme
  const shot1600 = await send("Page.captureScreenshot", { format: "png" });
  fs.writeFileSync(`${ARTIFACTS_DIR}/ryzora_lockscreens_1600_dark.png`, Buffer.from(shot1600.data, "base64"));
  console.log("Saved ryzora_lockscreens_1600_dark.png");

  // Hover over the second card (Tokyo Glass Quickshell)
  const boxRes = await send("Runtime.evaluate", {
    expression: `
      (() => {
        const cards = document.querySelectorAll('.store-card');
        if (cards.length > 1) {
          const rect = cards[1].getBoundingClientRect();
          return { x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 };
        }
        return null;
      })()
    `,
    returnByValue: true,
  });

  if (boxRes.result.value) {
    const { x, y } = boxRes.result.value;
    await send("Input.dispatchMouseEvent", {
      type: "mouseMoved",
      x,
      y,
    });
    await new Promise((r) => setTimeout(r, 400));
    const shotHover = await send("Page.captureScreenshot", { format: "png" });
    fs.writeFileSync(`${ARTIFACTS_DIR}/ryzora_lockscreens_dark_hover.png`, Buffer.from(shotHover.data, "base64"));
    console.log("Saved ryzora_lockscreens_dark_hover.png");
  }

  // Click on "Filters" button to expand FilterPanel
  await send("Runtime.evaluate", {
    expression: `
      (() => {
        const buttons = Array.from(document.querySelectorAll('button'));
        const filterBtn = buttons.find(b => b.textContent && b.textContent.includes('Filters'));
        if (filterBtn) filterBtn.click();
      })()
    `,
  });
  await new Promise((r) => setTimeout(r, 500));

  const shotPanel = await send("Page.captureScreenshot", { format: "png" });
  fs.writeFileSync(`${ARTIFACTS_DIR}/ryzora_lockscreens_filter_panel.png`, Buffer.from(shotPanel.data, "base64"));
  console.log("Saved ryzora_lockscreens_filter_panel.png");

  await send("Page.close");
  ws.close();
}

run().catch(console.error);
