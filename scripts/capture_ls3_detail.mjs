import fs from "fs";

const ARTIFACTS_DIR = "/home/silentbyte/.gemini/antigravity-ide/brain/50ef2876-38e5-4b46-b545-a5640a15a918";

async function run() {
  const tabRes = await fetch("http://127.0.0.1:9222/json/new?http://127.0.0.1:1420/", { method: "PUT" });
  const tab = await tabRes.json();

  const ws = new WebSocket(tab.webSocketDebuggerUrl);
  let id = 1;
  const send = (method, params = {}) => new Promise((resolve, reject) => {
    const msgId = id++;
    const handler = (event) => {
      const data = JSON.parse(event.data);
      if (data.id === msgId) {
        ws.removeEventListener("message", handler);
        if (data.error) reject(data.error); else resolve(data.result);
      }
    };
    ws.addEventListener("message", handler);
    ws.send(JSON.stringify({ id: msgId, method, params }));
  });

  await new Promise(r => ws.onopen = r);
  await send("Page.enable");
  await send("Runtime.enable");

  // 1440x950 desktop viewport
  await send("Emulation.setDeviceMetricsOverride", {
    width: 1440,
    height: 950,
    deviceScaleFactor: 1,
    mobile: false,
  });

  await new Promise(r => setTimeout(r, 1200));

  // Set Dark theme and navigate to Lock Screens
  await send("Runtime.evaluate", { expression: `
    localStorage.setItem("ryzora_appearance_mode", "dark");
    document.documentElement.classList.add("dark");
    document.documentElement.setAttribute("data-theme", "dark");
    const lockTab = Array.from(document.querySelectorAll("button")).find(b => b.textContent && b.textContent.trim() === "Lock Screens");
    if (lockTab) lockTab.click();
  `});
  await new Promise(r => setTimeout(r, 800));

  // 1. Click on "Aurora Glass Hyprlock" card
  await send("Runtime.evaluate", { expression: `
    const cards = Array.from(document.querySelectorAll(".store-card"));
    const auroraCard = cards.find(c => c.textContent && c.textContent.includes("Aurora Glass"));
    if (auroraCard) auroraCard.click();
  `});
  await new Promise(r => setTimeout(r, 800));

  // Capture: Aurora Glass Hyprlock Detail View (Dark Theme)
  const shot1 = await send("Page.captureScreenshot", { format: "png" });
  fs.writeFileSync(`${ARTIFACTS_DIR}/ryzora_lockscreen_detail_hyprlock_dark.png`, Buffer.from(shot1.data, "base64"));
  console.log("Saved ryzora_lockscreen_detail_hyprlock_dark.png");

  // 2. Click on "Files & Code" tab
  await send("Runtime.evaluate", { expression: `
    const tabBtns = Array.from(document.querySelectorAll("button"));
    const filesTab = tabBtns.find(b => b.textContent && b.textContent.includes("Files"));
    if (filesTab) filesTab.click();
  `});
  await new Promise(r => setTimeout(r, 500));

  const shotFiles = await send("Page.captureScreenshot", { format: "png" });
  fs.writeFileSync(`${ARTIFACTS_DIR}/ryzora_lockscreen_detail_tabs_files.png`, Buffer.from(shotFiles.data, "base64"));
  console.log("Saved ryzora_lockscreen_detail_tabs_files.png");

  // 3. Click "Dry Run Preview" button
  await send("Runtime.evaluate", { expression: `
    const btns = Array.from(document.querySelectorAll("button"));
    const dryRunBtn = btns.find(b => b.textContent && b.textContent.includes("Dry Run"));
    if (dryRunBtn) dryRunBtn.click();
  `});
  await new Promise(r => setTimeout(r, 700));

  const shotDryRun = await send("Page.captureScreenshot", { format: "png" });
  fs.writeFileSync(`${ARTIFACTS_DIR}/ryzora_lockscreen_detail_dryrun.png`, Buffer.from(shotDryRun.data, "base64"));
  console.log("Saved ryzora_lockscreen_detail_dryrun.png");

  // Dismiss dry run dialog and click Back to Lock Screens
  await send("Runtime.evaluate", { expression: `
    // Close dry run dialog
    const cancelBtn = Array.from(document.querySelectorAll("button")).find(b => b.textContent && b.textContent.trim() === "Cancel");
    if (cancelBtn) cancelBtn.click();

    // Click Back to Lock Screens
    setTimeout(() => {
      const backBtn = Array.from(document.querySelectorAll("button")).find(b => b.textContent && b.textContent.includes("Back to Lock Screens"));
      if (backBtn) backBtn.click();
    }, 200);
  `});
  await new Promise(r => setTimeout(r, 800));

  // 4. Click on "Sugar Candy SDDM" card
  await send("Runtime.evaluate", { expression: `
    const cards = Array.from(document.querySelectorAll(".store-card"));
    const sddmCard = cards.find(c => c.textContent && c.textContent.includes("Sugar Candy"));
    if (sddmCard) sddmCard.click();
  `});
  await new Promise(r => setTimeout(r, 800));

  const shotSddm = await send("Page.captureScreenshot", { format: "png" });
  fs.writeFileSync(`${ARTIFACTS_DIR}/ryzora_lockscreen_detail_sddm_dark.png`, Buffer.from(shotSddm.data, "base64"));
  console.log("Saved ryzora_lockscreen_detail_sddm_dark.png");

  // 5. Toggle to Light Theme on Sugar Candy SDDM
  await send("Runtime.evaluate", { expression: `
    document.documentElement.classList.remove("dark");
    document.documentElement.classList.add("light");
    document.documentElement.setAttribute("data-theme", "light");
    localStorage.setItem("ryzora_appearance_mode", "light");
  `});
  await new Promise(r => setTimeout(r, 600));

  const shotLight = await send("Page.captureScreenshot", { format: "png" });
  fs.writeFileSync(`${ARTIFACTS_DIR}/ryzora_lockscreen_detail_light.png`, Buffer.from(shotLight.data, "base64"));
  console.log("Saved ryzora_lockscreen_detail_light.png");

  await send("Page.close");
  ws.close();
  console.log("All LS-3 detail screenshots completed successfully!");
}

run().catch(console.error);
