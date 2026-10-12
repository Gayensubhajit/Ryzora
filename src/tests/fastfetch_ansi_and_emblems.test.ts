import test from "node:test";
import assert from "node:assert/strict";
import { ansiToHtml, get256Color, parseFastfetchAnsi, stripJsoncComments } from "../services/fastfetchAnsi.ts";

test("Fastfetch ANSI Parser: Standard & Bright SGR foreground colors", () => {
  // Red (31), Green (32), Blue (34), Bright Yellow (93), Bright Cyan (96)
  const input = "\x1b[31mRed\x1b[0m \x1b[32mGreen\x1b[0m \x1b[34mBlue\x1b[0m \x1b[93mBrightYellow\x1b[0m \x1b[96mBrightCyan\x1b[0m";
  const html = ansiToHtml(input);

  assert.ok(html.includes("color: #ef4444"), "Must map 31 to red");
  assert.ok(html.includes("color: #22c55e"), "Must map 32 to green");
  assert.ok(html.includes("color: #3b82f6"), "Must map 34 to blue");
  assert.ok(html.includes("color: #fde047"), "Must map 93 to bright yellow");
  assert.ok(html.includes("color: #38bdf8"), "Must map 96 to bright cyan");
});

test("Fastfetch ANSI Parser: 256-color palette (38;5;n & 48;5;n)", () => {
  // 38;5;196 = Bright Red in 6x6x6 cube
  const color196 = get256Color(196);
  assert.ok(color196.startsWith("rgb("), "256-color cube index must produce rgb()");

  const inputFg = "\x1b[38;5;196m256-Color\x1b[0m";
  const htmlFg = ansiToHtml(inputFg);
  assert.ok(htmlFg.includes(`color: ${color196}`), "Must parse 38;5;n foreground");

  const inputBg = "\x1b[48;5;235mDarkBg\x1b[0m";
  const htmlBg = ansiToHtml(inputBg);
  assert.ok(htmlBg.includes("background-color: rgb("), "Must parse 48;5;n background");
});

test("Fastfetch ANSI Parser: 24-bit Truecolor RGB (38;2;r;g;b & 48;2;r;g;b)", () => {
  const input = "\x1b[38;2;226;52;42mGarudaRed\x1b[48;2;20;25;35mDarkPanel\x1b[0m";
  const html = ansiToHtml(input);

  assert.ok(html.includes("color: rgb(226, 52, 42)"), "Must parse 24-bit RGB foreground");
  assert.ok(html.includes("background-color: rgb(20, 25, 35)"), "Must parse 24-bit RGB background");
});

test("Fastfetch ANSI Parser: Text styles (bold, dim, italic, underline)", () => {
  const input = "\x1b[1mBold\x1b[22m \x1b[2mDim\x1b[22m \x1b[3mItalic\x1b[23m \x1b[4mUnderline\x1b[24m";
  const html = ansiToHtml(input);

  assert.ok(html.includes("font-weight: 700"), "Must parse bold");
  assert.ok(html.includes("opacity: 0.75"), "Must parse dim");
  assert.ok(html.includes("font-style: italic"), "Must parse italic");
  assert.ok(html.includes("text-decoration: underline"), "Must parse underline");
});

test("Fastfetch ANSI Parser: Cursor positioning & indentation (\x1b[40C)", () => {
  const input = "\x1b[40C\x1b[34mIndentedText\x1b[0m";
  const html = ansiToHtml(input);

  assert.ok(html.startsWith(" ".repeat(40)), "Must expand \x1b[40C to 40 monospace spaces");
  assert.ok(html.includes("IndentedText"), "Must include text content");
});

test("Fastfetch ANSI Parser: SGR color persistence across lines & bottom palette blocks", () => {
  const input = "\x1b[32mLine1\nLine2StillGreen\x1b[0m\n\x1b[31m\uf3fe\x1b[32m\uf3fe\x1b[34m\uf192\x1b[0m";
  const html = ansiToHtml(input);
  const lines = html.split("\n");

  assert.equal(lines.length, 3, "Must render 3 lines");
  assert.ok(lines[0].includes("color: #22c55e") && lines[0].includes("Line1"), "First line must be green");
  assert.ok(lines[1].includes("color: #22c55e") && lines[1].includes("Line2StillGreen"), "Second line must inherit green until reset");
  assert.ok(lines[2].includes("█") && lines[2].includes("\uf111"), "Bottom palette must normalize PUA blocks to solid glyphs");
});

test("Fastfetch ANSI Parser: 2D cursor movement (A, D, G) and surrogate pairs", () => {
  const input = "LogoLeft\nLogoLine2\x1b[1A\r\x1b[12C\u{f035b}\x1b[15GAligned";
  const html = ansiToHtml(input);
  const lines = html.split("\n");

  assert.equal(lines.length, 2, "Cursor up must keep output on 2 lines");
  assert.ok(lines[0].startsWith("LogoLeft    \u{f035b} Aligned"), "Must place surrogate-pair icon at col 12 and Aligned at col 14 (15G) on row 0");
  assert.equal(lines[1], "LogoLine2", "Must preserve row 1");
});

test("Fastfetch JSONC Parser: stripJsoncComments preserves URLs inside strings", () => {
  const jsonc = `{
    // Line comment
    "$schema": "https://github.com/fastfetch-cli/fastfetch/raw/dev/doc/json_schema.json",
    /* Block comment */
    "logo": {
      "width": 34,
      "height": 15,
    }
  }`;
  const parsed = JSON.parse(stripJsoncComments(jsonc));
  assert.equal(parsed.$schema, "https://github.com/fastfetch-cli/fastfetch/raw/dev/doc/json_schema.json");
  assert.equal(parsed.logo.width, 34);
  assert.equal(parsed.logo.height, 15);
});

test("Fastfetch Kitty Protocol: parseFastfetchAnsi extracts image and coordinates", () => {
  const payloadB64 = Buffer.from("/home/silentbyte/.config/fastfetch/presets/fastfetch-aayushchalise/images/arch.png").toString("base64");
  const raw = `\x1b[m\n\n\x1b_Ga=T,f=100,t=f,c=36,r=18;${payloadB64}\x1b\\\x1b[20A\x1b[40C\x1b[m\x1b[90m  \x1b[31m\x1b[m\n\x1b[40C\x1b[m\x1b[1m\x1b[31msilentbyte\x1b[m@\x1b[1m\x1b[31mslayer\x1b[m`;

  const parsed = parseFastfetchAnsi(raw);

  assert.equal(parsed.hasKittyImage, true, "Must detect kitty image");
  assert.equal(parsed.kittyCols, 36, "Must parse 36 columns");
  assert.equal(parsed.kittyRows, 18, "Must parse 18 rows");
  assert.equal(parsed.textRows, 2, "Must count 2 text rows");
  assert.ok(parsed.kittyImageUrl?.includes("arch.png"), "Must extract correct image URL");
  assert.ok(parsed.textHtml.includes("silentbyte"), "Must preserve textual output");
  assert.ok(parsed.textHtml.includes(" ".repeat(40)), "Must retain column 40 indentation");
});

test("Fastfetch Real Presets: aayushchalise, preset-2, and preset-3 render colors, bottom blocks, and kitty dimensions", async () => {
  const { execFileSync } = await import("node:child_process");
  const fs = await import("node:fs");
  const path = await import("node:path");
  const os = await import("node:os");

  const runPreset = (presetId: string, overrideLogoSource?: string) => {
    const repoCfg = path.resolve(`repositories/community/packages/${presetId}/files/config.jsonc`);
    const rawJsonc = fs.readFileSync(repoCfg, "utf8");
    const parsedCfg = JSON.parse(stripJsoncComments(rawJsonc));

    parsedCfg.logo.type = "kitty-direct";
    if (!parsedCfg.logo.width && parsedCfg.logo.height) {
      parsedCfg.logo.width = parsedCfg.logo.height * 2;
    }
    if (overrideLogoSource) {
      parsedCfg.logo.source = overrideLogoSource;
    } else if (typeof parsedCfg.logo.source === "string" && parsedCfg.logo.source.startsWith("~/")) {
      const home = os.homedir();
      const expanded = path.join(home, parsedCfg.logo.source.slice(2));
      if (fs.existsSync(expanded)) {
        parsedCfg.logo.source = expanded;
      } else {
        const alt = path.join(path.dirname(repoCfg), "images", path.basename(expanded));
        if (fs.existsSync(alt)) parsedCfg.logo.source = alt;
      }
    }

    const tmpCfg = path.join(os.tmpdir(), `ryzora_test_${presetId}.jsonc`);
    fs.writeFileSync(tmpCfg, JSON.stringify(parsedCfg, null, 2));
    try {
      const out = execFileSync("fastfetch", ["-c", tmpCfg, "--pipe", "false"], { encoding: "utf8" });
      return parseFastfetchAnsi(out);
    } finally {
      fs.rmSync(tmpCfg, { force: true });
    }
  };

  // 1. fastfetch-aayushchalise (native Kitty image arch.png, 36x18)
  const pAayush = runPreset("fastfetch-aayushchalise");
  assert.equal(pAayush.hasKittyImage, true, "fastfetch-aayushchalise must produce Kitty image");
  assert.equal(pAayush.kittyCols, 36, "fastfetch-aayushchalise must have 36 cols");
  assert.equal(pAayush.kittyRows, 18, "fastfetch-aayushchalise must have 18 rows");
  assert.ok((pAayush.textRows ?? 0) >= 18, "fastfetch-aayushchalise must have full telemetry rows");
  assert.ok(pAayush.textHtml.includes("\uf111"), "fastfetch-aayushchalise must render solid bottom color circles");
  assert.ok(pAayush.textHtml.includes("color: #ef4444"), "fastfetch-aayushchalise must include ANSI red");

  // 2. fastfetch-preset-2 (untouched + with Katana emblem)
  const p2 = runPreset("fastfetch-preset-2");
  assert.ok((p2.textRows ?? 0) >= 18, "fastfetch-preset-2 must have full rows");
  assert.ok(p2.textHtml.includes("\uf111"), "fastfetch-preset-2 must render solid bottom color circles");
  assert.ok(p2.textHtml.includes("color: #3b82f6"), "fastfetch-preset-2 must include ANSI blue keys");

  const homeKatana = path.join(os.homedir(), "Pictures/ryodecors/katana.png");
  const fallbackArch = path.resolve("repositories/community/packages/fastfetch-aayushchalise/files/images/arch.png");
  const emblemPath = fs.existsSync(homeKatana) ? homeKatana : fallbackArch;
  const p2Emblem = runPreset("fastfetch-preset-2", emblemPath);
  assert.equal(p2Emblem.hasKittyImage, true, "fastfetch-preset-2 with emblem must produce Kitty image");
  assert.equal(p2Emblem.kittyCols, 34, "fastfetch-preset-2 must respect declared 34 cols");
  assert.equal(p2Emblem.kittyRows, 15, "fastfetch-preset-2 must respect declared 15 rows");

  // 3. fastfetch-preset-3 (untouched)
  const p3 = runPreset("fastfetch-preset-3");
  assert.ok((p3.textRows ?? 0) >= 18, "fastfetch-preset-3 must have full rows");
  assert.ok(p3.textHtml.includes("█"), "fastfetch-preset-3 must normalize bottom \uf3fe to solid blocks █");
  assert.ok(p3.textHtml.includes("color: #a855f7"), "fastfetch-preset-3 must include ANSI magenta");
});




test("Fastfetch ANSI Parser: SGR Inverse (7 / 27) and CUP positioning (H)", () => {
  const inputInv = "\x1b[31m\x1b[7mInverse\x1b[27mNormal\x1b[0m";
  const htmlInv = ansiToHtml(inputInv);
  assert.ok(htmlInv.includes("background-color: #ef4444"), "SGR 7 must invert fg to bg");

  const inputCup = "Line1\nLine2\x1b[1;1HRow0Col0";
  const htmlCup = ansiToHtml(inputCup);
  assert.ok(htmlCup.startsWith("Row0Col0"), "CUP 1;1H must position at start of row 0");
});
