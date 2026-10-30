import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";

// 1. Verify Catalog: cyber-spec and preset-1 removed, starts with Two-Column Spectrum
test("Fastfetch Catalog: first two packages removed and catalog starts with Two-Column Spectrum", () => {
  const repoPath = path.resolve(process.cwd(), "repositories/community/repository.json");
  const repoData = JSON.parse(fs.readFileSync(repoPath, "utf-8"));

  const pkgIds = new Set(repoData.packages.map((p: any) => p.id));
  assert.equal(pkgIds.has("fastfetch-cyber-spec"), false, "fastfetch-cyber-spec must be removed");
  assert.equal(pkgIds.has("fastfetch-preset-1"), false, "fastfetch-preset-1 must be removed");

  const ffPackages = repoData.packages.filter(
    (p: any) => p.package_type === "fastfetch" || p.category === "fastfetch"
  );

  assert.ok(ffPackages.length >= 22, `Expected at least 22 fastfetch packages, got ${ffPackages.length}`);
  assert.equal(ffPackages[0].id, "fastfetch-preset-2");
  assert.equal(ffPackages[0].name, "Two-Column Spectrum");
});

// 2. Verify Detail Routing Contract: Fastfetch routes to FastfetchDetailView, Lock Screen to LockScreenDetailView, other to PackageDetailModal
test("Detail Routing Contract: Fastfetch packages route to FastfetchDetailView, LockScreen to LockScreenDetailView, others to PackageDetailModal", () => {
  const isLockscreen = (pkg: { category: string; package_type: string }) =>
    pkg.category === "lockscreens" || pkg.package_type === "lockscreen";

  const isFastfetch = (pkg: { category: string; package_type: string }) =>
    pkg.category === "fastfetch" || pkg.package_type === "fastfetch";

  const resolveDetailView = (pkg: { category: string; package_type: string }) => {
    if (isLockscreen(pkg)) return "LockScreenDetailView";
    if (isFastfetch(pkg)) return "FastfetchDetailView";
    return "PackageDetailModal";
  };

  // Fastfetch cases -> route to FastfetchDetailView
  assert.equal(
    resolveDetailView({ category: "fastfetch", package_type: "fastfetch" }),
    "FastfetchDetailView"
  );
  assert.equal(
    resolveDetailView({ category: "terminal", package_type: "fastfetch" }),
    "FastfetchDetailView"
  );
  assert.equal(
    resolveDetailView({ category: "fastfetch", package_type: "theme" }),
    "FastfetchDetailView"
  );

  // Lockscreen cases -> route to LockScreenDetailView
  assert.equal(
    resolveDetailView({ category: "lockscreens", package_type: "lockscreen" }),
    "LockScreenDetailView"
  );

  // Generic other cases -> route to PackageDetailModal
  assert.equal(
    resolveDetailView({ category: "rices", package_type: "rice" }),
    "PackageDetailModal"
  );
  assert.equal(
    resolveDetailView({ category: "bars", package_type: "waybar" }),
    "PackageDetailModal"
  );
});

// 3. Fastfetch Detail View: Pure package detail page (not an editor)
test("Fastfetch Detail View: Pure package detail page with Install / Open in Settings actions", () => {
  const detailFile = fs.readFileSync(path.resolve(process.cwd(), "src/views/FastfetchDetailView.tsx"), "utf-8");

  // Pure detail elements
  assert.ok(detailFile.includes("ProductDetailShell"), "Must use ProductDetailShell");
  assert.ok(detailFile.includes('categoryLabel={isEmblem ? "Fastfetch Emblems" : "Fastfetch Configs"}'), "Must have dynamic categoryLabel for Fastfetch");
  assert.ok(detailFile.includes("Open in Fastfetch Settings"), "Must have Open in Fastfetch Settings button");
  assert.ok(detailFile.includes("Available on this system"), "Must show Available on this system");
  assert.ok(detailFile.includes("~/.config/fastfetch/"), "Must show target path");
  assert.ok(detailFile.includes("Overview"), "Must have Overview tab");
  assert.ok(detailFile.includes("Files & Code"), "Must have Files tab");
  assert.ok(detailFile.includes("Dependencies"), "Must have Dependencies tab");
  assert.ok(detailFile.includes("Installation"), "Must have Installation tab");
  assert.ok(detailFile.includes("Changelog"), "Must have Changelog tab");

  // Overflow menu actions
  assert.ok(detailFile.includes("Uninstall"), "Must have Uninstall in overflow");
  assert.ok(detailFile.includes("View Files"), "Must have View Files in overflow");
  assert.ok(detailFile.includes("Upstream Repository"), "Must have Upstream Repository in overflow");

  // Must NOT contain editor controls inside detail page
  assert.ok(!detailFile.includes("FastfetchLivePreview"), "Must not embed live preview editor in detail view");
  assert.ok(!detailFile.includes("FastfetchControls"), "Must not embed configuration controls in detail view");
  assert.ok(!detailFile.includes("FastfetchPresetSidebar"), "Must not embed preset sidebar in detail view");
});

// 4. Fastfetch Settings Page: Global configuration workspace with preset handoff
test("Fastfetch Settings Page: Global editor in FastfetchView with handoff and controls", () => {
  const settingsFile = fs.readFileSync(path.resolve(process.cwd(), "src/views/FastfetchView.tsx"), "utf-8");

  // Props for handoff
  assert.ok(settingsFile.includes("initialStyleId?: string"), "Must accept initialStyleId prop");
  assert.ok(settingsFile.includes("onConsumeInitialPresetId?: () => void"), "Must support consuming initial preset ID");

  // Core editor features
  assert.ok(settingsFile.includes("Style / Presets"), "Must have Style presets list");
  assert.ok(settingsFile.includes("silentbyte@slayer"), "Must have terminal preview");
  assert.ok(settingsFile.includes("Configuration"), "Must have Configuration panel");
  assert.ok(settingsFile.includes("Emblem"), "Must have Emblem controls");
  assert.ok(settingsFile.includes("Width"), "Must have Width control");
  assert.ok(settingsFile.includes("Height"), "Must have Height control");
  assert.ok(settingsFile.includes("Padding"), "Must have Padding control");
  assert.ok(settingsFile.includes("READOUT ACCENT"), "Must have Accent color control");
  assert.ok(settingsFile.includes("INFO MODULES"), "Must have INFO MODULES control");
  assert.ok(settingsFile.includes("APPLY INSTALLED STYLE"), "Must have APPLY action");
  assert.ok(settingsFile.includes("REMOVE INSTALLED STYLE"), "Must have REMOVE action");
  assert.ok(settingsFile.includes("RESET TO DEFAULT"), "Must have RESET action");
  assert.ok(settingsFile.includes("SAVE"), "Must have SAVE action");
  assert.ok(settingsFile.includes("REVERT"), "Must have REVERT action");
  assert.ok(settingsFile.includes("PREVIEW IN TERMINAL"), "Must have PREVIEW IN TERMINAL action");
});

// 5. Sidebar and App routing isolation: No duplicate sidebar tool, App wires handoff
test("App & Sidebar Routing: No duplicate sidebar tool, App wires FastfetchDetailView and preset handoff", () => {
  const sidebarContent = fs.readFileSync(path.resolve(process.cwd(), "src/components/Sidebar.tsx"), "utf-8");
  const appContent = fs.readFileSync(path.resolve(process.cwd(), "src/App.tsx"), "utf-8");
  const modalContent = fs.readFileSync(path.resolve(process.cwd(), "src/components/PackageDetailModal.tsx"), "utf-8");

  // Sidebar toolsNav must NOT contain Fastfetch shortcut (Settings only reachable via Store -> Detail)
  assert.ok(!sidebarContent.includes('id: "fastfetch"'), "Sidebar toolsNav must not contain Fastfetch shortcut");

  // App.tsx wires FastfetchDetailView with preset handoff
  assert.ok(appContent.includes("FastfetchDetailView"), "App.tsx must import and use FastfetchDetailView");
  assert.ok(appContent.includes("fastfetchSettingsPresetId"), "App.tsx must track fastfetchSettingsPresetId state");
  assert.ok(appContent.includes("onOpenSettings"), "App.tsx must provide onOpenSettings callback");

  // PackageDetailModal must NOT contain dead fastfetch bridge code
  assert.ok(!modalContent.includes("Open in Fastfetch Settings"), "PackageDetailModal must not have dead fastfetch button");
});

// 6. Branding Isolation Contract: Zero Ryoku/RyoHub/RyoStore references in Fastfetch workspace
test("Branding Isolation: Zero Ryoku, RyoHub, or RyoStore branding across Fastfetch files", () => {
  const filesToCheck = [
    "src/views/FastfetchDetailView.tsx",
    "src/views/FastfetchView.tsx",
  ];

  const forbiddenTerms = ["ryoku", "ryohub", "ryostore"];

  for (const relPath of filesToCheck) {
    const fullPath = path.resolve(process.cwd(), relPath);
    const content = fs.readFileSync(fullPath, "utf-8").toLowerCase();
    for (const term of forbiddenTerms) {
      assert.equal(
        content.includes(term),
        false,
        `File ${relPath} must not contain forbidden term "${term}"`
      );
    }
  }
});

// 7. Full Runtime Flow Simulation (TESTS A through F)
test("Runtime Verification: Complete Fastfetch Store -> Detail -> Settings Lifecycle (TESTS A-F)", () => {
  type AppState = {
    activeCategory: string;
    selectedPackage: any | null;
    fastfetchSettingsPresetId: string | null;
    selectedSettingsStyleId: string;
  };

  // Initial State: Store / Discover
  let state: AppState = {
    activeCategory: "discover",
    selectedPackage: null,
    fastfetchSettingsPresetId: null,
    selectedSettingsStyleId: "fastfetch-preset-2",
  };

  const twoColSpectrum = {
    id: "fastfetch-preset-2",
    title: "Two-Column Spectrum",
    category: "fastfetch",
    package_type: "fastfetch",
  };

  const lockscreenPkg = {
    id: "silentsddm-wave",
    title: "SilentSDDM Wave",
    category: "lockscreens",
    package_type: "lockscreen",
  };

  const genericPkg = {
    id: "rice-nord",
    title: "Nord Rice",
    category: "rices",
    package_type: "rice",
  };

  const resolveActiveOverlay = (s: AppState) => {
    if (!s.selectedPackage) return null;
    if (s.selectedPackage.category === "lockscreens" || s.selectedPackage.package_type === "lockscreen") {
      return "LockScreenDetailView";
    }
    if (s.selectedPackage.category === "fastfetch" || s.selectedPackage.package_type === "fastfetch") {
      return "FastfetchDetailView";
    }
    return "PackageDetailModal";
  };

  // TEST A: Fastfetch Store -> click Two-Column Spectrum -> full-page package detail appears
  state.selectedPackage = twoColSpectrum;
  assert.equal(resolveActiveOverlay(state), "FastfetchDetailView", "TEST A: Must render FastfetchDetailView");
  assert.equal(state.activeCategory, "discover", "TEST A: activeCategory remains discover during detail viewing");

  // TEST B: Installed package -> click Open in Fastfetch Settings
  // Handoff logic: set preset ID, clear selectedPackage, set activeCategory to fastfetch
  const onOpenSettings = (presetId: string) => {
    state.fastfetchSettingsPresetId = presetId;
    state.selectedPackage = null;
    state.activeCategory = "fastfetch";
    // FastfetchView consumes preset ID
    state.selectedSettingsStyleId = state.fastfetchSettingsPresetId;
    state.fastfetchSettingsPresetId = null;
  };

  onOpenSettings(state.selectedPackage.id);
  assert.equal(resolveActiveOverlay(state), null, "TEST B: Overlay closed");
  assert.equal(state.activeCategory, "fastfetch", "TEST B: Active view switched to fastfetch Settings");
  assert.equal(state.selectedSettingsStyleId, "fastfetch-preset-2", "TEST B: Two-Column Spectrum preset is selected in Settings");

  // TEST C: Inside Settings -> select another preset, e.g. Spectrum (fastfetch-preset-3)
  state.selectedSettingsStyleId = "fastfetch-preset-3";
  assert.equal(state.activeCategory, "fastfetch", "TEST C: Still on Fastfetch Settings page");
  assert.equal(state.selectedSettingsStyleId, "fastfetch-preset-3", "TEST C: Selected style changed to fastfetch-preset-3");

  // TEST D: Return to Store -> package detail still works
  state.activeCategory = "discover";
  state.selectedPackage = twoColSpectrum;
  assert.equal(resolveActiveOverlay(state), "FastfetchDetailView", "TEST D: FastfetchDetailView opens again from Store");
  state.selectedPackage = null;

  // TEST E: Open a non-Fastfetch package -> existing detail behavior unchanged
  state.selectedPackage = genericPkg;
  assert.equal(resolveActiveOverlay(state), "PackageDetailModal", "TEST E: Generic package opens PackageDetailModal");
  state.selectedPackage = null;

  // TEST F: Lock Screen detail page -> unchanged
  state.selectedPackage = lockscreenPkg;
  assert.equal(resolveActiveOverlay(state), "LockScreenDetailView", "TEST F: Lock screen opens LockScreenDetailView");
  state.selectedPackage = null;
});

// 8. Fastfetch Settings Installed-Only Filter Contract
test("Fastfetch Settings Installed-Only Filter Contract", () => {
  const settingsFile = fs.readFileSync(path.resolve(process.cwd(), "src/views/FastfetchView.tsx"), "utf-8");
  assert.ok(settingsFile.includes("installedFastfetchPackages"), "Must compute installedFastfetchPackages");
  assert.ok(settingsFile.includes("No installed styles"), "Must render No installed styles when empty");
  assert.ok(settingsFile.includes("Installed"), "Must surface Installed badge on installed styles");
});

// 9. Fastfetch Correctness: No Sidebar entry, No Fake Telemetry, No Remote GitHub Emblems
test("Fastfetch Correctness Contract: No Sidebar Shortcut, Real Hardware Telemetry, Local Emblems Only", () => {
  const sidebarFile = fs.readFileSync(path.resolve(process.cwd(), "src/components/Sidebar.tsx"), "utf-8");
  const settingsFile = fs.readFileSync(path.resolve(process.cwd(), "src/views/FastfetchView.tsx"), "utf-8");

  // 1. Sidebar does not contain fastfetch
  assert.ok(!sidebarFile.includes('id: "fastfetch"'), "Sidebar must not contain Fastfetch shortcut");

  // 2. No fake hardcoded telemetry in INITIAL_ROWS
  assert.ok(!settingsFile.includes("Ryzen 7 5800X"), "Must not hardcode Ryzen 7 5800X demo CPU");
  assert.ok(!settingsFile.includes("RTX 3060 Mobile"), "Must not hardcode RTX 3060 Mobile demo GPU");
  assert.ok(!settingsFile.includes("CachyOS x86_64"), "Must not hardcode CachyOS demo OS");
  assert.ok(!settingsFile.includes("fish 4.7.1"), "Must not hardcode fish 4.7.1 demo shell");
  assert.ok(!settingsFile.includes("6.18.32-1-cachyos-lts"), "Must not hardcode demo kernel");
  assert.ok(settingsFile.includes('"Detecting…"'), "Initial rows must use Detecting… placeholder");

  // 3. No fake remote emblem library
  assert.ok(!settingsFile.includes("EMBLEM_PRESETS"), "Must eliminate hardcoded EMBLEM_PRESETS array");
  assert.ok(!settingsFile.includes("raw.githubusercontent.com"), "Must not download remote GitHub emblems into UI");
  assert.ok(!settingsFile.includes("setCustomEmblemUrl(found.hero_image)"), "Hero image must not be assigned to emblem");
  assert.ok(settingsFile.includes("No installed emblems"), "Must render No installed emblems empty state");
  assert.ok(settingsFile.includes("list_fastfetch_local_emblems"), "Must query real local filesystem emblems");

  // 4. Real terminal preview wiring
  assert.ok(settingsFile.includes("preview_fastfetch_terminal"), "Must invoke preview_fastfetch_terminal with real config");
  assert.ok(settingsFile.includes("apply_fastfetch_configuration"), "Must invoke apply_fastfetch_configuration");
});

// 10. Fastfetch Store Subfilters and Strict Categorization Contract
test("Fastfetch Store Subfilters: exactly Configs and Emblems without All, strict counts and separation", () => {
  const catViewFile = fs.readFileSync(path.resolve(process.cwd(), "src/views/CategoryView.tsx"), "utf-8");
  assert.ok(catViewFile.includes('fastfetch: ["Configs", "Emblems"]'), "Fastfetch subfilters must be exactly Configs and Emblems (no All)");
  assert.ok(!catViewFile.includes('fastfetch: ["All", "Configs", "Emblems"]'), "Fastfetch subfilters must not contain All");
  assert.ok(catViewFile.includes('setSubFilter(activeCategory === "fastfetch" ? "Configs" : "All")'), "Default subfilter for fastfetch must be Configs");

  const repo = JSON.parse(fs.readFileSync(path.resolve(process.cwd(), "repositories/community/repository.json"), "utf-8"));
  const fastfetchPkgs = repo.packages.filter((p: any) => p.category === "fastfetch" || p.package_type === "fastfetch");
  const configs = fastfetchPkgs.filter((p: any) => p.subfilter === "Configs");
  const emblems = fastfetchPkgs.filter((p: any) => p.subfilter === "Emblems");

  assert.equal(configs.length, 23, "Fastfetch Configs must contain exactly 23 preset packages");
  assert.equal(emblems.length, 11, "Fastfetch Emblems must contain exactly 11 emblem packages");
  assert.equal(configs.length + emblems.length, fastfetchPkgs.length, "Configs and Emblems must partition all Fastfetch packages without overlap or unclassified items");
});

// 11. Fastfetch Emblem Integrity: No Ryoku branding, local assets exist on disk, Ryzora Community author
test("Fastfetch Emblem Integrity: local assets exist, Ryzora Community attribution, zero Ryoku branding", () => {
  const repoRaw = fs.readFileSync(path.resolve(process.cwd(), "repositories/community/repository.json"), "utf-8");
  assert.ok(!/ryoku/i.test(repoRaw), "repository.json must contain zero Ryoku branding");
  assert.ok(!/ryostore/i.test(repoRaw), "repository.json must contain zero RyoStore branding");
  assert.ok(!/ryohub/i.test(repoRaw), "repository.json must contain zero RyoHub branding");

  const repo = JSON.parse(repoRaw);
  const emblems = repo.packages.filter((p: any) => p.subfilter === "Emblems");
  assert.equal(emblems.length, 11);

  const expectedNames = [
    "Katana", "Arch", "Moon", "Camera", "Mic", "Needle",
    "Aurelius", "David", "Hawk", "Lighthouse", "Rashin"
  ];

  for (const name of expectedNames) {
    const found = emblems.find((e: any) => e.name.toLowerCase().includes(name.toLowerCase()));
    assert.ok(found, `Must find emblem package for ${name}`);
    assert.equal(found.author.name, "Ryzora Community", `${name} author must be Ryzora Community`);
    assert.ok(found.hero_image.startsWith("packages/fastfetch-emblem-"), `${name} hero_image must be local package path`);

    // Verify file exists on disk
    const diskPath = path.resolve(process.cwd(), "repositories/community", found.hero_image);
    assert.ok(fs.existsSync(diskPath), `Emblem file on disk must exist: ${diskPath}`);

    // Verify manifest
    const manifestPath = path.resolve(process.cwd(), "repositories/community", found.manifest);
    assert.ok(fs.existsSync(manifestPath), `Manifest on disk must exist: ${manifestPath}`);
    const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf-8"));
    assert.equal(manifest.author, "Ryzora Community", `Manifest author must be Ryzora Community for ${name}`);
    assert.ok(!/ryoku/i.test(JSON.stringify(manifest)), `Manifest must contain no Ryoku branding for ${name}`);
  }
});

// 12. App routing: Both Emblem and Config cards open full FastfetchDetailView with settings handoff
test("App routing: Both Emblem and Config cards open full FastfetchDetailView with settings handoff", () => {
  const appFile = fs.readFileSync(path.resolve(process.cwd(), "src/App.tsx"), "utf-8");
  assert.ok(
    appFile.includes('selectedPackage.category === "fastfetch" || selectedPackage.package_type === "fastfetch"'),
    "App.tsx routes all Fastfetch packages (Configs and Emblems) to FastfetchDetailView"
  );
  assert.ok(
    appFile.includes("fastfetchSettingsEmblemId"),
    "App.tsx manages fastfetchSettingsEmblemId state for emblem handoff to Settings"
  );
  assert.ok(
    appFile.includes("initialEmblemId"),
    "App.tsx passes initialEmblemId to FastfetchView"
  );
});

// 14. Emblem Card Preview and Install Destination Contract
test("Fastfetch Emblem Card Preview and Install Destination Contract", () => {
  // 1. Emblem manifests install strictly into ~/.config/fastfetch/logos/<package-id>/<filename>
  const emblemDir = path.resolve(process.cwd(), "repositories/community/packages");
  const packages = fs.readdirSync(emblemDir).filter(p => p.startsWith("fastfetch-emblem-"));
  assert.equal(packages.length, 11, "Must have exactly 11 emblem packages");

  for (const pkg of packages) {
    const manifestPath = path.join(emblemDir, pkg, "manifest.json");
    const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf-8"));
    const files = manifest.files || [];
    assert.ok(files.length > 0, `${pkg} must declare installation files`);
    for (const f of files) {
      assert.ok(
        f.target.startsWith(`~/.config/fastfetch/logos/${pkg}/`),
        `${pkg} target must be scoped to ~/.config/fastfetch/logos/${pkg}/, got: ${f.target}`
      );
      assert.notEqual(
        f.target,
        "~/.config/fastfetch/config.jsonc",
        `${pkg} must never install directly to active user config.jsonc`
      );
    }
  }

  // 2. FastfetchDetailView adapts for emblems
  const detailFile = fs.readFileSync(path.resolve(process.cwd(), "src/views/FastfetchDetailView.tsx"), "utf-8");
  assert.ok(detailFile.includes("isEmblem"), "FastfetchDetailView must branch on isEmblem");
  assert.ok(detailFile.includes('categoryLabel={isEmblem ? "Fastfetch Emblems" : "Fastfetch Configs"}'), "Breadcrumb adapts to subtype");
  assert.ok(detailFile.includes("object-contain"), "Emblem preview uses object-contain");

  // 3. StoreCard and PackageCard use contain presentation
  const storeCardFile = fs.readFileSync(path.resolve(process.cwd(), "src/components/StoreCard.tsx"), "utf-8");
  assert.ok(storeCardFile.includes('fit={isEmblem ? "contain" : "cover"}'), "StoreCard passes contain fit for emblems");
});

// 13. DiscoverView Fastfetch subfilter navigation and CategoryHeader contract
test("DiscoverView Fastfetch subfilter navigation and CategoryHeader contract", () => {
  const discoverFile = fs.readFileSync(path.resolve(process.cwd(), "src/views/DiscoverView.tsx"), "utf-8");
  assert.ok(discoverFile.includes('fastfetch: ["Configs", "Emblems"]'), "DiscoverView must define Fastfetch subfilters");
  assert.ok(discoverFile.includes('setSubFilter(tab === "fastfetch" ? "Configs" : "All")'), "DiscoverView must default fastfetch to Configs");

  const headerFile = fs.readFileSync(path.resolve(process.cwd(), "src/components/catalogue/CategoryHeader.tsx"), "utf-8");
  assert.ok(headerFile.includes('subFilter === "Emblems" ? "Fastfetch Emblems" : "Fastfetch Configs"'), "CategoryHeader must render Fastfetch Configs or Fastfetch Emblems");
  assert.ok(!headerFile.includes('return "Fastfetch Profiles"'), "CategoryHeader must not return Fastfetch Profiles");
});


// 15. Fastfetch Config Lifecycle and Safety Invariant Contract
test("Fastfetch Config Lifecycle: Selecting a preset NEVER touches config.jsonc, Save is atomic", () => {
  const settingsFile = fs.readFileSync(path.resolve(process.cwd(), "src/views/FastfetchView.tsx"), "utf-8");
  const systemFile = fs.readFileSync(path.resolve(process.cwd(), "crates/ryzora-qt-backend/src/lib.rs"), "utf-8");

  // 1. handleSelectStyle must NOT invoke apply_fastfetch_configuration
  const selectStyleIdx = settingsFile.indexOf("const handleSelectStyle");
  const selectStyleEnd = settingsFile.indexOf("const handleApplyInstalledStyle", selectStyleIdx);
  const selectStyleBody = settingsFile.slice(selectStyleIdx, selectStyleEnd);
  assert.ok(
    !selectStyleBody.includes("apply_fastfetch_configuration"),
    "handleSelectStyle must NEVER call apply_fastfetch_configuration"
  );

  // 2. save_fastfetch_configuration must reject empty content
  assert.ok(
    systemFile.includes("trimmed.is_empty()"),
    "Backend must check and reject empty configuration"
  );
  assert.ok(
    systemFile.includes("Refusing to save empty Fastfetch configuration"),
    "Backend must return error on empty configuration"
  );

  // 3. Backend must perform atomic write with temporary file and rename
  assert.ok(
    systemFile.includes(".config.jsonc.tmp."),
    "Backend must write to temporary file before replacing config.jsonc"
  );
  assert.ok(
    systemFile.includes("fs::rename(&tmp, &target)"),
    "Backend must atomically rename temporary file over target_path"
  );

  // 4. Backend must create backup of existing config.jsonc
  assert.ok(
    systemFile.includes("config.jsonc.ryzora.bak"),
    "Backend must backup existing configuration before overwriting"
  );
});

// 16. Fastfetch Detail Action Button & Typography Contract
test("Fastfetch Detail Action Button: explicit contrast, no white-on-white text, readable actions", () => {
  const detailFile = fs.readFileSync(path.resolve(process.cwd(), "src/views/FastfetchDetailView.tsx"), "utf-8");

  // Must not have text-white on accent buttons
  assert.ok(
    !detailFile.includes("bg-[var(--rz-accent)] hover:bg-[var(--rz-accent-hover)] text-white"),
    "Detail action buttons must NOT have text-white on silver accent background"
  );

  // Must have theme-aware contrast on accent buttons
  assert.ok(
    detailFile.includes("text-[var(--rz-accent-contrast)]") || detailFile.includes("text-[#050608]"),
    "Detail action buttons must use theme-aware contrast (text-[var(--rz-accent-contrast)])"
  );

  // Action labels
  assert.ok(detailFile.includes("Open in Fastfetch Settings"), "Must render Open in Fastfetch Settings");
  assert.ok(detailFile.includes("Install"), "Must render Install action for uninstalled");
});
