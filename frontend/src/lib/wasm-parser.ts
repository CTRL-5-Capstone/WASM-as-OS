/**
 * wasm-parser.ts
 * Standalone client-side WASM binary parser.
 * Ported and extended from the original inspect.js LEB128 decoder.
 * No dependencies — runs entirely in the browser.
 */

// ─── LEB128 helpers ────────────────────────────────────────────────────────

function readULEB128(bytes: Uint8Array, offset: number): [value: number, bytesRead: number] {
  let result = 0;
  let shift = 0;
  let pos = offset;
  while (pos < bytes.length) {
    const byte = bytes[pos++];
    result |= (byte & 0x7f) << shift;
    shift += 7;
    if ((byte & 0x80) === 0) break;
    if (shift >= 28) break; // guard against overlong encodings
  }
  return [result, pos - offset];
}

function readString(bytes: Uint8Array, offset: number, length: number): string {
  try {
    return new TextDecoder("utf-8").decode(bytes.slice(offset, offset + length));
  } catch {
    return "<?>";
  }
}

// ─── Section IDs ───────────────────────────────────────────────────────────

export const SECTION_NAMES: Record<number, string> = {
  0:  "Custom",
  1:  "Type",
  2:  "Import",
  3:  "Function",
  4:  "Table",
  5:  "Memory",
  6:  "Global",
  7:  "Export",
  8:  "Start",
  9:  "Element",
  10: "Code",
  11: "Data",
  12: "Data Count",
};

export const IMPORT_KIND_NAMES: Record<number, string> = {
  0: "func",
  1: "table",
  2: "memory",
  3: "global",
};

// ─── Types ─────────────────────────────────────────────────────────────────

export interface WasmSection {
  id: number;
  name: string;
  offset: number;   // byte offset of section content start
  length: number;   // byte length of section content
  raw: Uint8Array;  // raw section bytes
}

export interface WasmImport {
  module: string;
  name:   string;
  kind:   number;
  kindName: string;
}

export interface WasmExport {
  name:  string;
  kind:  number;
  kindName: string;
  index: number;
}

export interface WasmParseResult {
  valid:           boolean;
  version:         number;
  fileSizeBytes:   number;
  sections:        WasmSection[];
  imports:         WasmImport[];
  exports:         WasmExport[];
  functionCount:   number;
  globalCount:     number;
  memoryCount:     number;
  dataSegments:    number;
  customSections:  string[];  // names of custom sections
  strings:         string[];  // printable strings found in data segments
  error?:          string;
}

// ─── Main parser ───────────────────────────────────────────────────────────

export function parseWasm(buffer: ArrayBuffer): WasmParseResult {
  const bytes = new Uint8Array(buffer);
  const empty: WasmParseResult = {
    valid: false, version: 0, fileSizeBytes: bytes.length,
    sections: [], imports: [], exports: [],
    functionCount: 0, globalCount: 0, memoryCount: 0,
    dataSegments: 0, customSections: [], strings: [], error: "",
  };

  if (bytes.length < 8) {
    return { ...empty, error: "File too small to be a WASM module" };
  }

  // Magic number: \0asm
  if (bytes[0] !== 0x00 || bytes[1] !== 0x61 || bytes[2] !== 0x73 || bytes[3] !== 0x6d) {
    return { ...empty, error: "Invalid magic number — not a WASM binary" };
  }

  const version = bytes[4] | (bytes[5] << 8) | (bytes[6] << 16) | (bytes[7] << 24);

  const sections: WasmSection[] = [];
  const imports:  WasmImport[]  = [];
  const exports:  WasmExport[]  = [];
  const customSections: string[] = [];
  const strings: string[] = [];
  let functionCount = 0;
  let globalCount   = 0;
  let memoryCount   = 0;
  let dataSegments  = 0;

  let pos = 8;
  while (pos + 1 < bytes.length) {
    const sectionId = bytes[pos];
    pos += 1;

    const [secLen, lenBytes] = readULEB128(bytes, pos);
    pos += lenBytes;

    const contentStart = pos;
    const contentEnd   = Math.min(pos + secLen, bytes.length);
    const raw          = bytes.slice(contentStart, contentEnd);

    sections.push({
      id:     sectionId,
      name:   SECTION_NAMES[sectionId] ?? `Unknown(${sectionId})`,
      offset: contentStart,
      length: secLen,
      raw,
    });

    // ── Section 0: Custom ──────────────────────────────────────────
    if (sectionId === 0 && raw.length > 0) {
      const [nameLen, nb] = readULEB128(raw, 0);
      const secName = readString(raw, nb, nameLen);
      customSections.push(secName);
    }

    // ── Section 2: Import ─────────────────────────────────────────
    if (sectionId === 2 && raw.length > 0) {
      let p = 0;
      const [count, cb] = readULEB128(raw, p); p += cb;
      for (let i = 0; i < count && p < raw.length; i++) {
        const [mLen, mb] = readULEB128(raw, p); p += mb;
        const mod = readString(raw, p, mLen); p += mLen;
        const [nLen, nb] = readULEB128(raw, p); p += nb;
        const name = readString(raw, p, nLen); p += nLen;
        if (p >= raw.length) break;
        const kind = raw[p]; p += 1;
        // Skip index/type
        const [, idxBytes] = readULEB128(raw, p); p += idxBytes;
        imports.push({ module: mod, name, kind, kindName: IMPORT_KIND_NAMES[kind] ?? "?" });
      }
    }

    // ── Section 3: Function ───────────────────────────────────────
    if (sectionId === 3 && raw.length > 0) {
      const [count, cb] = readULEB128(raw, 0);
      functionCount = count + cb; // cb is small; approximate
      // Better: use the actual count
      functionCount = count;
    }

    // ── Section 5: Memory ────────────────────────────────────────
    if (sectionId === 5 && raw.length > 0) {
      const [count] = readULEB128(raw, 0);
      memoryCount = count;
    }

    // ── Section 6: Global ────────────────────────────────────────
    if (sectionId === 6 && raw.length > 0) {
      const [count] = readULEB128(raw, 0);
      globalCount = count;
    }

    // ── Section 7: Export ────────────────────────────────────────
    if (sectionId === 7 && raw.length > 0) {
      let p = 0;
      const [count, cb] = readULEB128(raw, p); p += cb;
      for (let i = 0; i < count && p < raw.length; i++) {
        const [nLen, nb] = readULEB128(raw, p); p += nb;
        const name = readString(raw, p, nLen); p += nLen;
        if (p >= raw.length) break;
        const kind = raw[p]; p += 1;
        const [index, ib] = readULEB128(raw, p); p += ib;
        exports.push({ name, kind, kindName: IMPORT_KIND_NAMES[kind] ?? "?", index });
      }
    }

    // ── Section 11: Data ─────────────────────────────────────────
    if (sectionId === 11 && raw.length > 0) {
      const [count, cb] = readULEB128(raw, 0);
      dataSegments = count;
      // Extract printable strings
      strings.push(...extractStrings(raw.slice(cb)));
    }

    pos = contentEnd;
  }

  return {
    valid: true,
    version,
    fileSizeBytes: bytes.length,
    sections,
    imports,
    exports,
    functionCount,
    globalCount,
    memoryCount,
    dataSegments,
    customSections,
    strings,
  };
}

// ─── String extraction ─────────────────────────────────────────────────────

function extractStrings(bytes: Uint8Array, minLen = 4): string[] {
  const results: string[] = [];
  let run = "";
  for (let i = 0; i < bytes.length; i++) {
    const c = bytes[i];
    if (c >= 0x20 && c <= 0x7e) {
      run += String.fromCharCode(c);
    } else {
      if (run.length >= minLen) results.push(run);
      run = "";
    }
  }
  if (run.length >= minLen) results.push(run);
  return results;
}

// ─── Security analysis ─────────────────────────────────────────────────────

export type RiskLevel = "critical" | "high" | "medium" | "info";

export interface SecurityFinding {
  id:          string;
  level:       RiskLevel;
  category:    string;
  title:       string;
  description: string;
  evidence?:   string;
}

const RISK_WEIGHTS: Record<RiskLevel, number> = {
  critical: 30,
  high:     18,
  medium:   8,
  info:     2,
};

const SUSPICIOUS_PATTERNS: Array<{
  pattern:     RegExp;
  level:       RiskLevel;
  category:    string;
  title:       string;
  description: string;
}> = [
  {
    pattern:     /\bfd_(read|write|seek|pread|pwrite|allocate|filestat_set|datasync|sync)\b/i,
    level:       "high",
    category:    "File I/O",
    title:       "WASI File Descriptor Operations",
    description: "Module can read or write files via WASI file descriptor API.",
  },
  {
    pattern:     /\bpath_(open|create_directory|remove_directory|rename|link|unlink|symlink|readlink|filestat)\b/i,
    level:       "critical",
    category:    "File System",
    title:       "WASI Filesystem Path Operations",
    description: "Module can open, create, delete, or rename files on the host filesystem.",
  },
  {
    pattern:     /\bsock_(open|connect|recv|send|shutdown|accept|bind|listen|getpeername|getsockname|getaddrinfo)\b/i,
    level:       "critical",
    category:    "Network",
    title:       "Network Socket Access",
    description: "Module can open network connections and transfer data.",
  },
  {
    pattern:     /\b(proc_exit|proc_raise|sched_yield|proc_exec|posix_spawn|fork|exec)\b/i,
    level:       "critical",
    category:    "Process Control",
    title:       "Process Execution Capability",
    description: "Module can spawn or control host processes.",
  },
  {
    pattern:     /\benviron_(get|sizes_get)\b/i,
    level:       "high",
    category:    "Environment",
    title:       "Environment Variable Access",
    description: "Module can read host environment variables (may leak secrets).",
  },
  {
    pattern:     /\b(clock_time_get|clock_res_get)\b/i,
    level:       "medium",
    category:    "Timing",
    title:       "High-Resolution Clock Access",
    description: "Module can measure precise time — potential timing side-channel.",
  },
  {
    pattern:     /\brandom_get\b/i,
    level:       "info",
    category:    "Entropy",
    title:       "Random Number Generation",
    description: "Module requests OS entropy. Generally benign but worth noting.",
  },
  {
    pattern:     /\b(args_get|args_sizes_get)\b/i,
    level:       "info",
    category:    "Arguments",
    title:       "Command-Line Argument Access",
    description: "Module reads host-provided command-line arguments.",
  },
];

const STRING_THREAT_PATTERNS: Array<{
  pattern: RegExp;
  level:   RiskLevel;
  title:   string;
}> = [
  { pattern: /\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}/,            level: "high",   title: "Hardcoded IP Address" },
  { pattern: /https?:\/\/[^\s"']+/,                             level: "high",   title: "Hardcoded URL" },
  { pattern: /(?:\/etc\/passwd|\/etc\/shadow|\/proc\/|\/sys\/)/, level: "critical", title: "Suspicious Filesystem Path" },
  { pattern: /[A-Za-z0-9+/]{40,}={0,2}/,                       level: "medium", title: "Potential Base64-Encoded Payload" },
  { pattern: /(?:password|passwd|secret|apikey|api_key|token)[:=]/i, level: "high", title: "Hardcoded Credential Pattern" },
];

export interface SecurityAnalysis {
  findings:  SecurityFinding[];
  riskScore: number;
  grade:     "A" | "B" | "C" | "D" | "F";
}

export function analyseWasm(parsed: WasmParseResult): SecurityAnalysis {
  const findings: SecurityFinding[] = [];

  if (!parsed.valid) {
    return { findings: [], riskScore: 0, grade: "A" };
  }

  // Check imports against suspicious patterns
  for (const imp of parsed.imports) {
    const fullName = `${imp.module}::${imp.name}`;
    for (const rule of SUSPICIOUS_PATTERNS) {
      if (rule.pattern.test(imp.name) || rule.pattern.test(fullName)) {
        const existing = findings.find(f => f.id === rule.title);
        if (!existing) {
          findings.push({
            id:          rule.title,
            level:       rule.level,
            category:    rule.category,
            title:       rule.title,
            description: rule.description,
            evidence:    fullName,
          });
        }
      }
    }
  }

  // Check data segment strings for suspicious patterns
  for (const str of parsed.strings) {
    for (const rule of STRING_THREAT_PATTERNS) {
      if (rule.pattern.test(str)) {
        const id = `str:${rule.title}`;
        if (!findings.find(f => f.id === id)) {
          findings.push({
            id,
            level:       rule.level,
            category:    "Embedded Data",
            title:       rule.title,
            description: `Suspicious pattern found in data segment strings.`,
            evidence:    str.slice(0, 80),
          });
        }
      }
    }
  }

  // Structural findings
  if (parsed.globalCount > 20) {
    findings.push({
      id:          "many_globals",
      level:       "medium",
      category:    "Complexity",
      title:       "High Global Variable Count",
      description: `Module defines ${parsed.globalCount} globals — may indicate complex or obfuscated state management.`,
    });
  }

  if (parsed.functionCount > 100) {
    findings.push({
      id:          "many_functions",
      level:       "info",
      category:    "Complexity",
      title:       "Large Function Count",
      description: `Module has ${parsed.functionCount} functions — large modules are harder to audit manually.`,
    });
  }

  if (parsed.customSections.some(n => n === "name")) {
    findings.push({
      id:          "debug_names",
      level:       "info",
      category:    "Debug Info",
      title:       "Debug Name Section Present",
      description: "Module contains a WASM 'name' custom section with debug symbols.",
    });
  }

  const riskScore = Math.min(
    100,
    findings.reduce((sum, f) => sum + RISK_WEIGHTS[f.level], 0),
  );

  const grade =
    riskScore <= 15 ? "A"
    : riskScore <= 35 ? "B"
    : riskScore <= 55 ? "C"
    : riskScore <= 75 ? "D"
    : "F";

  // Sort: critical → high → medium → info
  const order: RiskLevel[] = ["critical", "high", "medium", "info"];
  findings.sort((a, b) => order.indexOf(a.level) - order.indexOf(b.level));

  return { findings, riskScore, grade };
}

// ─── Hex dump helper ───────────────────────────────────────────────────────

export function hexDump(bytes: Uint8Array, maxBytes = 256): string {
  const limit = Math.min(bytes.length, maxBytes);
  const lines: string[] = [];
  for (let i = 0; i < limit; i += 16) {
    const row = bytes.slice(i, Math.min(i + 16, limit));
    const hex = Array.from(row).map(b => b.toString(16).padStart(2, "0")).join(" ");
    const ascii = Array.from(row)
      .map(b => (b >= 0x20 && b <= 0x7e) ? String.fromCharCode(b) : ".")
      .join("");
    lines.push(`${i.toString(16).padStart(8, "0")}  ${hex.padEnd(47)}  |${ascii}|`);
  }
  if (bytes.length > maxBytes) lines.push(`... (${bytes.length - maxBytes} more bytes)`);
  return lines.join("\n");
}

// ─── Module diff helper ────────────────────────────────────────────────────

export interface ModuleDiff {
  addedImports:   WasmImport[];
  removedImports: WasmImport[];
  addedExports:   WasmExport[];
  removedExports: WasmExport[];
  newFindings:    SecurityFinding[];
  resolvedFindings: SecurityFinding[];
}

export function diffModules(
  before: WasmParseResult,
  after:  WasmParseResult,
  beforeAnalysis: SecurityAnalysis,
  afterAnalysis:  SecurityAnalysis,
): ModuleDiff {
  const importKey = (i: WasmImport) => `${i.module}::${i.name}::${i.kindName}`;
  const exportKey = (e: WasmExport) => `${e.name}::${e.kindName}`;
  const findingKey = (f: SecurityFinding) => f.id;

  const beforeImportSet = new Set(before.imports.map(importKey));
  const afterImportSet  = new Set(after.imports.map(importKey));

  return {
    addedImports:   after.imports.filter(i => !beforeImportSet.has(importKey(i))),
    removedImports: before.imports.filter(i => !afterImportSet.has(importKey(i))),
    addedExports:   after.exports.filter(e => !new Set(before.exports.map(exportKey)).has(exportKey(e))),
    removedExports: before.exports.filter(e => !new Set(after.exports.map(exportKey)).has(exportKey(e))),
    newFindings:     afterAnalysis.findings.filter(f => !new Set(beforeAnalysis.findings.map(findingKey)).has(findingKey(f))),
    resolvedFindings: beforeAnalysis.findings.filter(f => !new Set(afterAnalysis.findings.map(findingKey)).has(findingKey(f))),
  };
}
// ── Tests (only run by vitest, stripped from production builds) ──────────
if (import.meta.vitest) {
  const { describe, it, expect } = import.meta.vitest;
 
  // Helper: minimal valid WASM (magic + version, no sections)
  function minimalWasm(): ArrayBuffer {
    return new Uint8Array([
      0x00, 0x61, 0x73, 0x6d,
      0x01, 0x00, 0x00, 0x00,
    ]).buffer;
  }
 
  describe('SECTION_NAMES', () => {
    it('maps 0 → Custom', () => {
      expect(SECTION_NAMES[0]).toBe('Custom');
    });
 
    it('maps 1 → Type', () => {
      expect(SECTION_NAMES[1]).toBe('Type');
    });
 
    it('maps 7 → Export', () => {
      expect(SECTION_NAMES[7]).toBe('Export');
    });
 
    it('maps 10 → Code', () => {
      expect(SECTION_NAMES[10]).toBe('Code');
    });
 
    it('has all 13 entries (0–12)', () => {
      for (let i = 0; i <= 12; i++) {
        expect(SECTION_NAMES[i]).toBeDefined();
      }
    });
  });
 
  describe('IMPORT_KIND_NAMES', () => {
    it('maps 0 → func', () => {
      expect(IMPORT_KIND_NAMES[0]).toBe('func');
    });
 
    it('maps 1 → table', () => {
      expect(IMPORT_KIND_NAMES[1]).toBe('table');
    });
 
    it('maps 2 → memory', () => {
      expect(IMPORT_KIND_NAMES[2]).toBe('memory');
    });
 
    it('maps 3 → global', () => {
      expect(IMPORT_KIND_NAMES[3]).toBe('global');
    });
  });
 
  describe('parseWasm', () => {
    it('rejects empty buffer', () => {
      const r = parseWasm(new ArrayBuffer(0));
      expect(r.valid).toBe(false);
    });
 
    it('rejects buffer smaller than 8 bytes', () => {
      const r = parseWasm(new Uint8Array([0x00, 0x61]).buffer);
      expect(r.valid).toBe(false);
      expect(r.error).toContain('too small');
    });
 
    it('rejects invalid magic number', () => {
      const bad = new Uint8Array([
        0xFF, 0xFF, 0xFF, 0xFF,
        0x01, 0x00, 0x00, 0x00,
      ]).buffer;
      const r = parseWasm(bad);
      expect(r.valid).toBe(false);
      expect(r.error).toContain('magic');
    });
 
    it('rejects 8 zero bytes', () => {
      const r = parseWasm(new Uint8Array(8).buffer);
      expect(r.valid).toBe(false);
    });
 
    it('parses minimal WASM as valid', () => {
      const r = parseWasm(minimalWasm());
      expect(r.valid).toBe(true);
      expect(r.version).toBe(1);
    });
 
    it('reports correct file size', () => {
      const r = parseWasm(minimalWasm());
      expect(r.fileSizeBytes).toBe(8);
    });
 
    it('returns empty arrays for minimal module', () => {
      const r = parseWasm(minimalWasm());
      expect(r.sections).toHaveLength(0);
      expect(r.imports).toHaveLength(0);
      expect(r.exports).toHaveLength(0);
      expect(r.functionCount).toBe(0);
    });
 
    it('parses a Type section', () => {
      const wasm = new Uint8Array([
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
        0x01, 0x04, 0x01, 0x60, 0x00, 0x00,
      ]).buffer;
      const r = parseWasm(wasm);
      expect(r.valid).toBe(true);
      expect(r.sections[0].name).toBe('Type');
    });
 
    it('parses Function count', () => {
      const wasm = new Uint8Array([
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
        0x01, 0x04, 0x01, 0x60, 0x00, 0x00,
        0x03, 0x02, 0x01, 0x00,
      ]).buffer;
      const r = parseWasm(wasm);
      expect(r.functionCount).toBe(1);
    });
 
    it('parses Memory section', () => {
      const wasm = new Uint8Array([
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
        0x05, 0x03, 0x01, 0x00, 0x01,
      ]).buffer;
      const r = parseWasm(wasm);
      expect(r.memoryCount).toBe(1);
    });
 
    it('parses Export section', () => {
      const wasm = new Uint8Array([
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
        0x01, 0x04, 0x01, 0x60, 0x00, 0x00,
        0x03, 0x02, 0x01, 0x00,
        0x07, 0x05, 0x01, 0x01, 0x66, 0x00, 0x00,
        0x0a, 0x04, 0x01, 0x02, 0x00, 0x0b,
      ]).buffer;
      const r = parseWasm(wasm);
      expect(r.exports).toHaveLength(1);
      expect(r.exports[0].name).toBe('f');
      expect(r.exports[0].kindName).toBe('func');
    });
 
    it('parses Global section count', () => {
      const wasm = new Uint8Array([
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
        0x06, 0x06, 0x01, 0x7f, 0x01, 0x41, 0x00, 0x0b,
      ]).buffer;
      const r = parseWasm(wasm);
      expect(r.globalCount).toBe(1);
    });
  });
}