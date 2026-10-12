import { resolveLocalAssetSrc } from "../providers/customMediaProvider.ts";

export interface ParsedFastfetchOutput {
  hasKittyImage: boolean;
  kittyImageUrl?: string;
  kittyCols?: number;
  kittyRows?: number;
  kittyTopPadding?: number;
  kittyLeftPadding?: number;
  textRows?: number;
  textHtml: string;
}

export function stripJsoncComments(input: string): string {
  let out = "";
  let inString = false;
  let escape = false;
  let i = 0;
  while (i < input.length) {
    const c = input[i];
    if (inString) {
      out += c;
      if (escape) {
        escape = false;
      } else if (c === "\\") {
        escape = true;
      } else if (c === '"') {
        inString = false;
      }
      i++;
    } else if (c === '"') {
      inString = true;
      out += c;
      i++;
    } else if (c === "/" && input[i + 1] === "/") {
      i += 2;
      while (i < input.length && input[i] !== "\n") i++;
    } else if (c === "/" && input[i + 1] === "*") {
      i += 2;
      while (i + 1 < input.length && !(input[i] === "*" && input[i + 1] === "/")) i++;
      if (i + 1 < input.length) i += 2;
    } else {
      out += c;
      i++;
    }
  }
  return out.replace(/,(\s*[\]}])/g, "$1");
}

export function parseFastfetchAnsi(raw: string): ParsedFastfetchOutput {
  if (!raw) return { hasKittyImage: false, textRows: 0, textHtml: "" };

  const kittyRegex = /\x1b_G([^;]*);([^\x1b\x07]+)(?:\x1b\\|\x07)/;
  const match = raw.match(kittyRegex);

  if (!match) {
    const textHtml = ansiToHtml(raw);
    const textRows = textHtml ? textHtml.split("\n").length : 0;
    return {
      hasKittyImage: false,
      textRows,
      textHtml,
    };
  }

  const controlsStr = match[1] || "";
  const payloadB64 = match[2] || "";
  const controls: Record<string, string> = {};
  for (const part of controlsStr.split(",")) {
    const [k, v] = part.split("=");
    if (k && v) controls[k] = v;
  }

  let imageUrl = "";
  try {
    const decoded = atob(payloadB64);
    if (controls.t === "f" || controls.t === "t") {
      imageUrl = resolveLocalAssetSrc(decoded);
    } else {
      imageUrl = `data:image/png;base64,${payloadB64}`;
    }
  } catch {
    imageUrl = "";
  }

  const preImage = raw.slice(0, match.index ?? 0);
  const leftPadMatch = preImage.match(/\x1b\[(\d+)C$/);
  const leftCols = leftPadMatch ? parseInt(leftPadMatch[1], 10) : 0;
  const topPadding = Math.max(0, preImage.split("\n").length - 1);

  const postImage = raw.slice((match.index ?? 0) + match[0].length);
  const upMatch = postImage.match(/^[\r\n]*\x1b\[(\d+)A/);
  const textPart = upMatch ? postImage.slice((upMatch.index ?? 0) + upMatch[0].length) : postImage;

  const cols = controls.c ? parseInt(controls.c, 10) : 36;
  const rows = controls.r ? parseInt(controls.r, 10) : 18;
  const textHtml = ansiToHtml(textPart);
  const textRows = textHtml ? textHtml.split("\n").length : 0;

  return {
    hasKittyImage: Boolean(imageUrl),
    kittyImageUrl: imageUrl,
    kittyCols: cols,
    kittyRows: rows,
    kittyTopPadding: topPadding,
    kittyLeftPadding: leftCols,
    textRows,
    textHtml,
  };
}

export function get256Color(n: number): string {
  const stdColors = [
    "#1e1e1e", "#ef4444", "#22c55e", "#eab308",
    "#3b82f6", "#a855f7", "#06b6d4", "#e2e8f0"
  ];
  const brightColors = [
    "#64748b", "#f87171", "#4ade80", "#fde047",
    "#60a5fa", "#c084fc", "#38bdf8", "#ffffff"
  ];

  if (n < 0 || n > 255) return "#f1f5f9";
  if (n < 8) return stdColors[n];
  if (n < 16) return brightColors[n - 8];
  if (n >= 16 && n <= 231) {
    const idx = n - 16;
    const r = Math.floor(idx / 36);
    const g = Math.floor((idx % 36) / 6);
    const b = idx % 6;
    const toVal = (x: number) => (x === 0 ? 0 : 55 + x * 40);
    return `rgb(${toVal(r)}, ${toVal(g)}, ${toVal(b)})`;
  }
  const gray = 8 + (n - 232) * 10;
  return `rgb(${gray}, ${gray}, ${gray})`;
}

interface Cell {
  ch: string;
  fg: string | null;
  bg: string | null;
  bold: boolean;
  dim: boolean;
  italic: boolean;
  underline: boolean;
}

function emptyCell(): Cell {
  return {
    ch: " ",
    fg: null,
    bg: null,
    bold: false,
    dim: false,
    italic: false,
    underline: false,
  };
}

function sameCellStyle(a: Cell, b: Cell): boolean {
  return (
    a.fg === b.fg &&
    a.bg === b.bg &&
    a.bold === b.bold &&
    a.dim === b.dim &&
    a.italic === b.italic &&
    a.underline === b.underline
  );
}

function isUnstyledSpace(c: Cell): boolean {
  return (
    c.ch === " " &&
    c.fg === null &&
    c.bg === null &&
    !c.bold &&
    !c.dim &&
    !c.italic &&
    !c.underline
  );
}

function normalizeFastfetchGlyphs(input: string): string {
  return input
    .replace(/\uf3fe/g, "█")
    .replace(/\uf192/g, "\uf111")
    .replace(/\uf3ff/g, "\uf303")
    .replace(/\uf878/g, "\uf108")
    .replace(/\ufb19/g, "\u{f035b}");
}

export function ansiToHtml(raw: string): string {
  if (!raw) return "";

  const cleaned = normalizeFastfetchGlyphs(raw)
    .replace(/\x1b_G[^\x1b\x07]*(?:\x1b\\|\x07)/g, "")
    .replace(/\x1b\[\?[0-9;:]*[a-zA-Z]/g, "")
    .replace(/\x1b\[[0-3]?J/g, "");

  const stdColors = [
    "#1e1e1e", "#ef4444", "#22c55e", "#eab308",
    "#3b82f6", "#a855f7", "#06b6d4", "#e2e8f0"
  ];
  const brightColors = [
    "#64748b", "#f87171", "#4ade80", "#fde047",
    "#60a5fa", "#c084fc", "#38bdf8", "#ffffff"
  ];

  const escapeHtml = (str: string) =>
    str.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

  const grid: Cell[][] = [[]];
  let row = 0;
  let col = 0;

  let currentFg: string | null = null;
  let currentBg: string | null = null;
  let isBold = false;
  let isDim = false;
  let isItalic = false;
  let isUnderline = false;
  let isInverse = false;

  const ensureRow = (r: number) => {
    while (grid.length <= r) {
      grid.push([]);
    }
  };

  const tokens = cleaned.split(/(\x1b\[[0-9;:]*[a-zA-Z])/);
  for (const token of tokens) {
    if (!token) continue;
    if (token.startsWith("\x1b[")) {
      const type = token.slice(-1);
      const argsStr = token.slice(2, -1).replace(/:+/g, ";");
      const args = argsStr ? argsStr.split(";").map((s) => (s === "" ? 0 : Number(s))) : [0];

      if (type === "m") {
        let i = 0;
        while (i < args.length) {
          const c = args[i];
          if (c === 0 || Number.isNaN(c)) {
            currentFg = null;
            currentBg = null;
            isBold = false;
            isDim = false;
            isItalic = false;
            isUnderline = false;
            isInverse = false;
          } else if (c === 1) {
            isBold = true;
          } else if (c === 2) {
            isDim = true;
          } else if (c === 3) {
            isItalic = true;
          } else if (c === 4) {
            isUnderline = true;
          } else if (c === 7) {
            isInverse = true;
          } else if (c === 22) {
            isBold = false;
            isDim = false;
          } else if (c === 23) {
            isItalic = false;
          } else if (c === 24) {
            isUnderline = false;
          } else if (c === 27) {
            isInverse = false;
          } else if (c >= 30 && c <= 37) {
            currentFg = stdColors[c - 30];
          } else if (c === 39) {
            currentFg = null;
          } else if (c >= 40 && c <= 47) {
            currentBg = stdColors[c - 40];
          } else if (c === 49) {
            currentBg = null;
          } else if (c >= 90 && c <= 97) {
            currentFg = brightColors[c - 90];
          } else if (c >= 100 && c <= 107) {
            currentBg = brightColors[c - 100];
          } else if (c === 38 && i + 2 < args.length && args[i + 1] === 5) {
            currentFg = get256Color(args[i + 2]);
            i += 2;
          } else if (c === 48 && i + 2 < args.length && args[i + 1] === 5) {
            currentBg = get256Color(args[i + 2]);
            i += 2;
          } else if (c === 38 && i + 4 < args.length && args[i + 1] === 2) {
            currentFg = `rgb(${args[i + 2]}, ${args[i + 3]}, ${args[i + 4]})`;
            i += 4;
          } else if (c === 48 && i + 4 < args.length && args[i + 1] === 2) {
            currentBg = `rgb(${args[i + 2]}, ${args[i + 3]}, ${args[i + 4]})`;
            i += 4;
          }
          i++;
        }
      } else if (type === "A") {
        row = Math.max(0, row - (args[0] || 1));
      } else if (type === "B") {
        row += args[0] || 1;
        ensureRow(row);
      } else if (type === "C") {
        col += args[0] || 1;
      } else if (type === "D") {
        col = Math.max(0, col - (args[0] || 1));
      } else if (type === "E") {
        row += args[0] || 1;
        col = 0;
        ensureRow(row);
      } else if (type === "F") {
        row = Math.max(0, row - (args[0] || 1));
        col = 0;
      } else if (type === "G") {
        col = Math.max(0, (args[0] || 1) - 1);
      } else if (type === "H" || type === "f") {
        row = Math.max(0, (args[0] || 1) - 1);
        col = Math.max(0, (args[1] || 1) - 1);
        ensureRow(row);
      } else if (type === "K") {
        ensureRow(row);
        const mode = args[0] || 0;
        if (mode === 0) {
          grid[row].length = Math.min(grid[row].length, col);
        } else if (mode === 1) {
          for (let k = 0; k <= col && k < grid[row].length; k++) {
            grid[row][k] = emptyCell();
          }
        } else if (mode === 2) {
          grid[row] = [];
        }
      }
    } else {
      for (const ch of token) {
        if (ch === "\n") {
          row++;
          col = 0;
          ensureRow(row);
        } else if (ch === "\r") {
          col = 0;
        } else {
          ensureRow(row);
          const lineCells = grid[row];
          while (lineCells.length < col) {
            lineCells.push(emptyCell());
          }
          const effectiveFg = isInverse ? (currentBg || "#050608") : currentFg;
          const effectiveBg = isInverse ? (currentFg || "#f1f5f9") : currentBg;
          lineCells[col] = {
            ch,
            fg: effectiveFg,
            bg: effectiveBg,
            bold: isBold,
            dim: isDim,
            italic: isItalic,
            underline: isUnderline,
          };
          col++;
        }
      }
    }
  }

  // Trim trailing completely unstyled empty lines
  while (grid.length > 0 && grid[grid.length - 1].every(isUnstyledSpace)) {
    grid.pop();
  }

  const processedLines: string[] = [];
  for (const lineCells of grid) {
    let end = lineCells.length;
    while (end > 0 && isUnstyledSpace(lineCells[end - 1])) {
      end--;
    }
    if (end === 0) {
      processedLines.push("");
      continue;
    }

    let lineOut = "";
    let runStart = 0;
    while (runStart < end) {
      const first = lineCells[runStart];
      let runEnd = runStart + 1;
      let runText = first.ch;
      while (runEnd < end && sameCellStyle(first, lineCells[runEnd])) {
        runText += lineCells[runEnd].ch;
        runEnd++;
      }

      const styles: string[] = [];
      if (first.fg) styles.push(`color: ${first.fg}`);
      if (first.bg) styles.push(`background-color: ${first.bg}`);
      if (first.bold) styles.push("font-weight: 700");
      if (first.dim) styles.push("opacity: 0.75");
      if (first.italic) styles.push("font-style: italic");
      if (first.underline) styles.push("text-decoration: underline");

      if (styles.length === 0) {
        lineOut += escapeHtml(runText);
      } else {
        lineOut += `<span style="${styles.join("; ")}">${escapeHtml(runText)}</span>`;
      }
      runStart = runEnd;
    }
    processedLines.push(lineOut);
  }

  return processedLines.join("\n");
}

