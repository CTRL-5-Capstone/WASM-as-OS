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

// ═══════════════════════════════════════════════════════════════════════════
// ADVANCED FORENSICS — CFG, Decompiler, Entropy, YARA
// ═══════════════════════════════════════════════════════════════════════════

// ─── Control Flow Graph extraction ─────────────────────────────────────────

export interface CFGNode {
  id:          string;      // "func_N" or "import_N"
  label:       string;      // display name
  kind:        "function" | "import" | "export" | "start";
  funcIndex:   number;
  byteSize:    number;      // code section bytes
  callees:     number[];    // function indices this calls
  calledBy:    number[];    // function indices that call this
  isExport:    boolean;
  isStart:     boolean;
  complexity:  number;      // branch instruction count (control flow complexity)
}

export interface CFGEdge {
  source: number;   // func index
  target: number;   // func index
}

export interface ControlFlowGraph {
  nodes:        CFGNode[];
  edges:        CFGEdge[];
  entryPoints:  number[];    // exported or start function indices
  maxDepth:     number;      // max call chain depth
  orphans:      number[];    // functions never called
  suspicious:   string[];    // anomaly descriptions
}

/** WASM bytecode opcodes we care about for CFG */
const OP_CALL     = 0x10;
const OP_CALL_IND = 0x11;
const OP_IF       = 0x04;
const OP_BR       = 0x0c;
const OP_BR_IF    = 0x0d;
const OP_BR_TABLE = 0x0e;
const OP_LOOP     = 0x03;
const OP_BLOCK    = 0x02;
const OP_END      = 0x0b;

export function extractCFG(parsed: WasmParseResult): ControlFlowGraph {
  const numImports = parsed.imports.filter(i => i.kind === 0).length;
  const nodes: CFGNode[] = [];
  const edgeSet = new Set<string>();
  const edges: CFGEdge[] = [];
  const exportedIndices = new Set(
    parsed.exports.filter(e => e.kind === 0).map(e => e.index)
  );

  // Build a name map from exports
  const exportNames = new Map<number, string>();
  for (const exp of parsed.exports) {
    if (exp.kind === 0) exportNames.set(exp.index, exp.name);
  }

  // Imported functions as nodes
  for (let i = 0; i < numImports; i++) {
    const imp = parsed.imports.filter(im => im.kind === 0)[i];
    nodes.push({
      id: `import_${i}`,
      label: imp ? `${imp.module}.${imp.name}` : `import_${i}`,
      kind: "import",
      funcIndex: i,
      byteSize: 0,
      callees: [],
      calledBy: [],
      isExport: false,
      isStart: false,
      complexity: 0,
    });
  }

  // Find the Code section (id=10)
  const codeSection = parsed.sections.find(s => s.id === 10);
  if (codeSection && codeSection.raw.length > 0) {
    const raw = codeSection.raw;
    let p = 0;
    const [funcCount, fb] = readULEB128(raw, p); p += fb;

    for (let fi = 0; fi < funcCount && p < raw.length; fi++) {
      const globalIdx = numImports + fi;
      const [bodyLen, bl] = readULEB128(raw, p); p += bl;
      const bodyStart = p;
      const bodyEnd = Math.min(p + bodyLen, raw.length);

      const callees: number[] = [];
      let complexity = 0;
      let bp = bodyStart;

      // Skip local declarations
      if (bp < bodyEnd) {
        const [localDeclCount, ldb] = readULEB128(raw, bp); bp += ldb;
        for (let ld = 0; ld < localDeclCount && bp < bodyEnd; ld++) {
          const [, countBytes] = readULEB128(raw, bp); bp += countBytes;
          bp += 1; // skip type byte
        }
      }

      // Scan opcodes for calls and branch instructions
      while (bp < bodyEnd) {
        const op = raw[bp]; bp += 1;
        if (op === OP_CALL) {
          const [targetIdx, tb] = readULEB128(raw, bp); bp += tb;
          if (!callees.includes(targetIdx)) callees.push(targetIdx);
        } else if (op === OP_CALL_IND) {
          // call_indirect — skip type index and table index
          const [, tb1] = readULEB128(raw, bp); bp += tb1;
          const [, tb2] = readULEB128(raw, bp); bp += tb2;
          complexity += 2; // indirect calls are highly complex
        } else if (op === OP_IF || op === OP_BR || op === OP_BR_IF || op === OP_LOOP) {
          complexity += 1;
          if (op === OP_BR || op === OP_BR_IF) {
            const [, lb] = readULEB128(raw, bp); bp += lb;
          } else if (op === OP_IF || op === OP_LOOP || op === OP_BLOCK) {
            bp += 1; // skip blocktype
          }
        } else if (op === OP_BR_TABLE) {
          const [vecLen, vb] = readULEB128(raw, bp); bp += vb;
          for (let v = 0; v <= vecLen && bp < bodyEnd; v++) {
            const [, lb] = readULEB128(raw, bp); bp += lb;
          }
          complexity += vecLen + 1;
        } else if (op === OP_BLOCK) {
          bp += 1; // skip blocktype
        }
        // Skip other multi-byte opcodes gracefully
      }

      const name = exportNames.get(globalIdx) ?? `func_${globalIdx}`;
      nodes.push({
        id: `func_${globalIdx}`,
        label: name,
        kind: exportedIndices.has(globalIdx) ? "export" : "function",
        funcIndex: globalIdx,
        byteSize: bodyLen,
        callees,
        calledBy: [],
        isExport: exportedIndices.has(globalIdx),
        isStart: false,
        complexity,
      });

      p = bodyEnd;
    }
  }

  // Build reverse edges (calledBy) and edge list
  for (const node of nodes) {
    for (const target of node.callees) {
      const key = `${node.funcIndex}->${target}`;
      if (!edgeSet.has(key)) {
        edgeSet.add(key);
        edges.push({ source: node.funcIndex, target });
      }
      const targetNode = nodes.find(n => n.funcIndex === target);
      if (targetNode && !targetNode.calledBy.includes(node.funcIndex)) {
        targetNode.calledBy.push(node.funcIndex);
      }
    }
  }

  // Entry points: exports + start section
  const startSection = parsed.sections.find(s => s.id === 8);
  if (startSection && startSection.raw.length > 0) {
    const [startIdx] = readULEB128(startSection.raw, 0);
    const n = nodes.find(nd => nd.funcIndex === startIdx);
    if (n) { n.isStart = true; n.kind = "start"; }
  }
  const entryPoints = nodes.filter(n => n.isExport || n.isStart).map(n => n.funcIndex);

  // Find orphans (defined functions never called and not exported/start)
  const calledSet = new Set(edges.map(e => e.target));
  const orphans = nodes
    .filter(n => n.kind === "function" && !calledSet.has(n.funcIndex) && !n.isExport && !n.isStart)
    .map(n => n.funcIndex);

  // Compute max call depth via BFS from entry points
  let maxDepth = 0;
  const nodeMap = new Map(nodes.map(n => [n.funcIndex, n]));
  for (const ep of entryPoints) {
    const visited = new Set<number>();
    const queue: [number, number][] = [[ep, 0]];
    while (queue.length > 0) {
      const [idx, depth] = queue.shift()!;
      if (visited.has(idx)) continue;
      visited.add(idx);
      maxDepth = Math.max(maxDepth, depth);
      const nd = nodeMap.get(idx);
      if (nd) {
        for (const c of nd.callees) {
          if (!visited.has(c)) queue.push([c, depth + 1]);
        }
      }
    }
  }

  // Detect suspicious patterns
  const suspicious: string[] = [];
  const avgComplexity = nodes.reduce((s, n) => s + n.complexity, 0) / Math.max(1, nodes.length);
  const highComplexity = nodes.filter(n => n.complexity > avgComplexity * 3 && n.complexity > 10);
  if (highComplexity.length > 0) {
    suspicious.push(`${highComplexity.length} function(s) with abnormally high branch complexity (possible obfuscation)`);
  }
  if (orphans.length > nodes.length * 0.4 && orphans.length > 5) {
    suspicious.push(`${orphans.length} orphan functions (${Math.round(orphans.length / nodes.length * 100)}%) — may indicate dead code or anti-analysis padding`);
  }
  if (maxDepth > 20) {
    suspicious.push(`Unusually deep call chain (depth ${maxDepth}) — possible recursion or call-chain obfuscation`);
  }
  const indirectCalls = nodes.filter(n => n.complexity > 5);
  if (indirectCalls.length > 3) {
    suspicious.push(`Multiple functions with heavy indirect calls — dynamic dispatch pattern`);
  }

  return { nodes, edges, entryPoints, maxDepth, orphans, suspicious };
}

// ─── Pseudo-decompiler (Wasm→pseudo-code) ──────────────────────────────────

export interface DecompiledFunction {
  index:    number;
  name:     string;
  params:   string[];
  results:  string[];
  locals:   string[];
  body:     string;       // pseudo-code string
  byteSize: number;
}

const WASM_TYPE_NAMES: Record<number, string> = {
  0x7f: "i32", 0x7e: "i64", 0x7d: "f32", 0x7c: "f64",
  0x70: "funcref", 0x6f: "externref",
};

const WASM_OP_NAMES: Record<number, string> = {
  0x00: "unreachable", 0x01: "nop", 0x02: "block", 0x03: "loop",
  0x04: "if", 0x05: "else", 0x0b: "end", 0x0c: "br", 0x0d: "br_if",
  0x0e: "br_table", 0x0f: "return", 0x10: "call", 0x11: "call_indirect",
  0x1a: "drop", 0x1b: "select",
  0x20: "local.get", 0x21: "local.set", 0x22: "local.tee",
  0x23: "global.get", 0x24: "global.set",
  0x28: "i32.load", 0x29: "i64.load", 0x2a: "f32.load", 0x2b: "f64.load",
  0x36: "i32.store", 0x37: "i64.store", 0x38: "f32.store", 0x39: "f64.store",
  0x3f: "memory.size", 0x40: "memory.grow",
  0x41: "i32.const", 0x42: "i64.const", 0x43: "f32.const", 0x44: "f64.const",
  0x45: "i32.eqz", 0x46: "i32.eq", 0x47: "i32.ne",
  0x48: "i32.lt_s", 0x49: "i32.lt_u", 0x4a: "i32.gt_s", 0x4b: "i32.gt_u",
  0x4c: "i32.le_s", 0x4d: "i32.le_u", 0x4e: "i32.ge_s", 0x4f: "i32.ge_u",
  0x6a: "i32.add", 0x6b: "i32.sub", 0x6c: "i32.mul",
  0x6d: "i32.div_s", 0x6e: "i32.div_u",
  0x6f: "i32.rem_s", 0x70: "i32.rem_u",
  0x71: "i32.and", 0x72: "i32.or", 0x73: "i32.xor",
  0x74: "i32.shl", 0x75: "i32.shr_s", 0x76: "i32.shr_u",
  0x7c: "i64.add", 0x7d: "i64.sub", 0x7e: "i64.mul",
  0xa7: "i32.wrap_i64", 0xac: "i64.extend_i32_s",
};

export function decompileModule(parsed: WasmParseResult): DecompiledFunction[] {
  const result: DecompiledFunction[] = [];
  const numImports = parsed.imports.filter(i => i.kind === 0).length;
  const exportNames = new Map<number, string>();
  for (const exp of parsed.exports) {
    if (exp.kind === 0) exportNames.set(exp.index, exp.name);
  }

  // Parse type section for function signatures
  const typeSection = parsed.sections.find(s => s.id === 1);
  const funcTypes: { params: number[]; results: number[] }[] = [];
  if (typeSection && typeSection.raw.length > 0) {
    const raw = typeSection.raw;
    let p = 0;
    const [count, cb] = readULEB128(raw, p); p += cb;
    for (let i = 0; i < count && p < raw.length; i++) {
      p += 1; // skip 0x60 (func type marker)
      const [paramCount, pb] = readULEB128(raw, p); p += pb;
      const params: number[] = [];
      for (let j = 0; j < paramCount && p < raw.length; j++) { params.push(raw[p]); p += 1; }
      const [resultCount, rb] = readULEB128(raw, p); p += rb;
      const results: number[] = [];
      for (let j = 0; j < resultCount && p < raw.length; j++) { results.push(raw[p]); p += 1; }
      funcTypes.push({ params, results });
    }
  }

  // Parse function section for type indices
  const funcSection = parsed.sections.find(s => s.id === 3);
  const funcTypeIndices: number[] = [];
  if (funcSection && funcSection.raw.length > 0) {
    const raw = funcSection.raw;
    let p = 0;
    const [count, cb] = readULEB128(raw, p); p += cb;
    for (let i = 0; i < count && p < raw.length; i++) {
      const [typeIdx, tb] = readULEB128(raw, p); p += tb;
      funcTypeIndices.push(typeIdx);
    }
  }

  const codeSection = parsed.sections.find(s => s.id === 10);
  if (!codeSection || codeSection.raw.length === 0) return result;

  const raw = codeSection.raw;
  let p = 0;
  const [funcCount, fb] = readULEB128(raw, p); p += fb;

  for (let fi = 0; fi < funcCount && p < raw.length; fi++) {
    const globalIdx = numImports + fi;
    const [bodyLen, bl] = readULEB128(raw, p); p += bl;
    const bodyStart = p;
    const bodyEnd = Math.min(p + bodyLen, raw.length);

    const name = exportNames.get(globalIdx) ?? `func_${globalIdx}`;
    const typeIdx = funcTypeIndices[fi] ?? -1;
    const sig = typeIdx >= 0 && typeIdx < funcTypes.length ? funcTypes[typeIdx] : null;

    const params = sig ? sig.params.map((t, i) => `${WASM_TYPE_NAMES[t] ?? "?"} p${i}`) : [];
    const results = sig ? sig.results.map(t => WASM_TYPE_NAMES[t] ?? "?") : [];

    // Parse locals
    const locals: string[] = [];
    let bp = bodyStart;
    if (bp < bodyEnd) {
      const [localDeclCount, ldb] = readULEB128(raw, bp); bp += ldb;
      for (let ld = 0; ld < localDeclCount && bp < bodyEnd; ld++) {
        const [count, countBytes] = readULEB128(raw, bp); bp += countBytes;
        const typeId = bp < bodyEnd ? raw[bp] : 0; bp += 1;
        for (let li = 0; li < count; li++) {
          locals.push(WASM_TYPE_NAMES[typeId] ?? `0x${typeId.toString(16)}`);
        }
      }
    }

    // Generate pseudo-code from opcodes
    const lines: string[] = [];
    let indent = 1;
    const pad = () => "  ".repeat(indent);
    let opCount = 0;
    const MAX_OPS = 200; // limit to avoid gigantic output

    while (bp < bodyEnd && opCount < MAX_OPS) {
      const op = raw[bp]; bp += 1;
      opCount++;

      if (op === OP_END) {
        indent = Math.max(0, indent - 1);
        lines.push(`${pad()}}`);
        continue;
      }

      const opName = WASM_OP_NAMES[op];
      if (!opName) {
        // Skip unknown opcodes
        continue;
      }

      // Handle structured opcodes
      if (op === OP_BLOCK) {
        bp += 1; // blocktype
        lines.push(`${pad()}block {`);
        indent++;
      } else if (op === OP_LOOP) {
        bp += 1;
        lines.push(`${pad()}loop {`);
        indent++;
      } else if (op === OP_IF) {
        bp += 1;
        lines.push(`${pad()}if (stack.pop()) {`);
        indent++;
      } else if (op === 0x05) { // else
        indent = Math.max(0, indent - 1);
        lines.push(`${pad()}} else {`);
        indent++;
      } else if (op === OP_CALL) {
        const [target, tb] = readULEB128(raw, bp); bp += tb;
        const targetName = exportNames.get(target) ??
          (target < numImports ? (parsed.imports.filter(i => i.kind === 0)[target]?.name ?? `import_${target}`) : `func_${target}`);
        lines.push(`${pad()}call ${targetName}  // func[${target}]`);
      } else if (op === OP_CALL_IND) {
        const [typeI, tb1] = readULEB128(raw, bp); bp += tb1;
        const [, tb2] = readULEB128(raw, bp); bp += tb2;
        lines.push(`${pad()}call_indirect type[${typeI}]  // dynamic dispatch`);
      } else if (op === OP_BR || op === OP_BR_IF) {
        const [depth, db] = readULEB128(raw, bp); bp += db;
        lines.push(`${pad()}${opName} ${depth}`);
      } else if (op === OP_BR_TABLE) {
        const [vecLen, vb] = readULEB128(raw, bp); bp += vb;
        const targets: number[] = [];
        for (let v = 0; v <= vecLen && bp < bodyEnd; v++) {
          const [t, tb] = readULEB128(raw, bp); bp += tb;
          targets.push(t);
        }
        lines.push(`${pad()}br_table [${targets.join(", ")}]`);
      } else if (op === 0x41) { // i32.const
        const [val, vb] = readULEB128(raw, bp); bp += vb;
        lines.push(`${pad()}push ${val}  // i32.const`);
      } else if (op === 0x42) { // i64.const
        const [val, vb] = readULEB128(raw, bp); bp += vb;
        lines.push(`${pad()}push ${val}L  // i64.const`);
      } else if (op >= 0x20 && op <= 0x24) {
        const [idx, ib] = readULEB128(raw, bp); bp += ib;
        lines.push(`${pad()}${opName} ${idx}`);
      } else if (op >= 0x28 && op <= 0x3e) {
        // memory instructions: alignment + offset
        const [, ab] = readULEB128(raw, bp); bp += ab;
        const [offset, ob] = readULEB128(raw, bp); bp += ob;
        lines.push(`${pad()}${opName} offset=${offset}`);
      } else {
        lines.push(`${pad()}${opName}`);
      }
    }

    if (opCount >= MAX_OPS && bp < bodyEnd) {
      lines.push(`${pad()}// ... ${bodyEnd - bp} more bytes truncated`);
    }

    const returnType = results.length > 0 ? `: ${results.join(", ")}` : "";
    const header = `fn ${name}(${params.join(", ")})${returnType}`;
    const localDecls = locals.length > 0 ? `  // locals: ${locals.join(", ")}\n` : "";
    const body = `${header} {\n${localDecls}${lines.join("\n")}\n}`;

    result.push({
      index: globalIdx,
      name,
      params,
      results,
      locals,
      body,
      byteSize: bodyLen,
    });

    p = bodyEnd;
  }

  return result;
}

// ─── Entropy computation ───────────────────────────────────────────────────

export interface EntropyBlock {
  offset:  number;
  size:    number;
  entropy: number;  // 0.0 – 8.0
}

/**
 * Compute Shannon entropy over fixed-size blocks of the binary.
 * High entropy (>7.0) suggests compressed/encrypted content.
 */
export function computeEntropy(bytes: Uint8Array, blockSize = 256): EntropyBlock[] {
  const blocks: EntropyBlock[] = [];
  for (let offset = 0; offset < bytes.length; offset += blockSize) {
    const end = Math.min(offset + blockSize, bytes.length);
    const chunk = bytes.slice(offset, end);
    const len = chunk.length;
    if (len === 0) continue;

    // Frequency table
    const freq = new Uint32Array(256);
    for (let i = 0; i < len; i++) freq[chunk[i]]++;

    // Shannon entropy
    let entropy = 0;
    for (let i = 0; i < 256; i++) {
      if (freq[i] === 0) continue;
      const p = freq[i] / len;
      entropy -= p * Math.log2(p);
    }

    blocks.push({ offset, size: len, entropy });
  }
  return blocks;
}

// ─── YARA rule parser & matcher ────────────────────────────────────────────

export interface YaraRule {
  name:        string;
  tags:        string[];
  meta:        Record<string, string>;
  strings:     YaraString[];
  condition:   string;
  raw:         string;
}

export interface YaraString {
  id:      string;        // $identifier
  type:    "text" | "hex" | "regex";
  value:   string;
  nocase?: boolean;
  wide?:   boolean;
}

export interface YaraMatch {
  rule:     string;
  tags:     string[];
  meta:     Record<string, string>;
  matches:  { stringId: string; offset: number; length: number; matched: string }[];
}

/**
 * Minimal YARA rule parser — handles the most common patterns:
 *   rule name : tag1 tag2 { meta: ... strings: ... condition: ... }
 * Supports text strings, hex strings { AB CD ?? EF }, and basic conditions.
 */
export function parseYaraRules(source: string): YaraRule[] {
  const rules: YaraRule[] = [];
  // Match rule blocks
  const ruleRegex = /rule\s+(\w+)\s*(?::\s*([\w\s]+?))?\s*\{([\s\S]*?)\}/g;
  let match;

  while ((match = ruleRegex.exec(source)) !== null) {
    const name = match[1];
    const tags = match[2] ? match[2].trim().split(/\s+/) : [];
    const body = match[3];

    // Parse meta section
    const meta: Record<string, string> = {};
    const metaMatch = body.match(/meta\s*:\s*([\s\S]*?)(?=strings\s*:|condition\s*:|$)/);
    if (metaMatch) {
      const metaLines = metaMatch[1].split("\n");
      for (const line of metaLines) {
        const kv = line.match(/^\s*(\w+)\s*=\s*"?([^"\n]+)"?\s*$/);
        if (kv) meta[kv[1]] = kv[2].trim();
      }
    }

    // Parse strings section
    const strings: YaraString[] = [];
    const stringsMatch = body.match(/strings\s*:\s*([\s\S]*?)(?=condition\s*:|$)/);
    if (stringsMatch) {
      const strLines = stringsMatch[1].split("\n");
      for (const line of strLines) {
        const textMatch = line.match(/^\s*(\$\w+)\s*=\s*"([^"]+)"(\s+(?:nocase|wide|ascii))*\s*$/);
        if (textMatch) {
          strings.push({
            id: textMatch[1],
            type: "text",
            value: textMatch[2],
            nocase: /nocase/.test(textMatch[3] || ""),
            wide: /wide/.test(textMatch[3] || ""),
          });
          continue;
        }
        const hexMatch = line.match(/^\s*(\$\w+)\s*=\s*\{([^}]+)\}\s*$/);
        if (hexMatch) {
          strings.push({
            id: hexMatch[1],
            type: "hex",
            value: hexMatch[2].trim(),
          });
          continue;
        }
        const regexMatch = line.match(/^\s*(\$\w+)\s*=\s*\/(.+)\/([is]*)\s*$/);
        if (regexMatch) {
          strings.push({
            id: regexMatch[1],
            type: "regex",
            value: regexMatch[2],
          });
        }
      }
    }

    // Parse condition
    const condMatch = body.match(/condition\s*:\s*([\s\S]*?)$/);
    const condition = condMatch ? condMatch[1].trim() : "any of them";

    rules.push({ name, tags, meta, strings, condition, raw: match[0] });
  }

  return rules;
}

/**
 * Match parsed YARA rules against a binary buffer.
 * Supports text string matching, hex patterns (with ?? wildcards), and basic conditions.
 */
export function matchYaraRules(rules: YaraRule[], bytes: Uint8Array): YaraMatch[] {
  const results: YaraMatch[] = [];

  for (const rule of rules) {
    const allMatches: YaraMatch["matches"] = [];

    for (const str of rule.strings) {
      if (str.type === "text") {
        const needle = str.nocase ? str.value.toLowerCase() : str.value;
        const haystack = new TextDecoder("utf-8", { fatal: false }).decode(bytes);
        const searchIn = str.nocase ? haystack.toLowerCase() : haystack;
        let idx = 0;
        while ((idx = searchIn.indexOf(needle, idx)) !== -1) {
          allMatches.push({
            stringId: str.id,
            offset: idx,
            length: needle.length,
            matched: haystack.slice(idx, idx + needle.length),
          });
          idx += 1;
        }
      } else if (str.type === "hex") {
        // Parse hex pattern with ?? wildcards
        const hexTokens = str.value.split(/\s+/).filter(t => t.length > 0);
        const pattern: (number | null)[] = hexTokens.map(t =>
          t === "??" ? null : parseInt(t, 16)
        );
        if (pattern.length === 0) continue;

        for (let offset = 0; offset <= bytes.length - pattern.length; offset++) {
          let found = true;
          for (let j = 0; j < pattern.length; j++) {
            if (pattern[j] !== null && bytes[offset + j] !== pattern[j]) {
              found = false;
              break;
            }
          }
          if (found) {
            allMatches.push({
              stringId: str.id,
              offset,
              length: pattern.length,
              matched: Array.from(bytes.slice(offset, offset + pattern.length))
                .map(b => b.toString(16).padStart(2, "0"))
                .join(" "),
            });
          }
        }
      } else if (str.type === "regex") {
        try {
          const text = new TextDecoder("utf-8", { fatal: false }).decode(bytes);
          const re = new RegExp(str.value, "g");
          let m;
          while ((m = re.exec(text)) !== null) {
            allMatches.push({
              stringId: str.id,
              offset: m.index,
              length: m[0].length,
              matched: m[0].slice(0, 80),
            });
          }
        } catch { /* invalid regex — skip */ }
      }
    }

    // Evaluate condition (simplified)
    const cond = rule.condition.trim();
    let ruleMatched = false;
    if (cond === "any of them") {
      ruleMatched = allMatches.length > 0;
    } else if (cond === "all of them") {
      const matchedIds = new Set(allMatches.map(m => m.stringId));
      ruleMatched = rule.strings.every(s => matchedIds.has(s.id));
    } else if (cond.match(/^\d+ of them$/)) {
      const n = parseInt(cond);
      const matchedIds = new Set(allMatches.map(m => m.stringId));
      ruleMatched = matchedIds.size >= n;
    } else {
      // Default: any match counts
      ruleMatched = allMatches.length > 0;
    }

    if (ruleMatched) {
      results.push({
        rule: rule.name,
        tags: rule.tags,
        meta: rule.meta,
        matches: allMatches,
      });
    }
  }

  return results;
}


// ─── In-source tests (vitest) ────────────────────────────────────────────────
// Stripped from production via `define: { 'import.meta.vitest': 'undefined' }`.
// Run with: npm test
if (import.meta.vitest) {
  const { describe, it, expect } = import.meta.vitest;
 
  // Real fixture: byte-for-byte copy of WasmOSTest/A.wasm — a minimal module
  // that imports `my_namespace.imported_func` and exports `exported_func`.
  // Embedded inline so tests are self-contained (no fs in jsdom).
  const A_WASM = new Uint8Array([
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
    0x01, 0x08, 0x02, 0x60, 0x01, 0x7f, 0x00, 0x60, 0x00, 0x00,
    0x02, 0x1e, 0x01, 0x0c, 0x6d, 0x79, 0x5f, 0x6e, 0x61, 0x6d, 0x65, 0x73,
    0x70, 0x61, 0x63, 0x65, 0x0d, 0x69, 0x6d, 0x70, 0x6f, 0x72, 0x74, 0x65,
    0x64, 0x5f, 0x66, 0x75, 0x6e, 0x63, 0x00, 0x00,
    0x03, 0x02, 0x01, 0x01,
    0x07, 0x11, 0x01, 0x0d, 0x65, 0x78, 0x70, 0x6f, 0x72, 0x74, 0x65, 0x64,
    0x5f, 0x66, 0x75, 0x6e, 0x63, 0x00, 0x01,
    0x0a, 0x08, 0x01, 0x06, 0x00, 0x41, 0x2a, 0x10, 0x00, 0x0b,
  ]);
 
  const makeHeaderOnly = () => new Uint8Array([
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
  ]);
 
  describe('parseWasm() — rejection paths', () => {
    it('rejects an empty buffer as too small', () => {
      const r = parseWasm(new ArrayBuffer(0));
      expect(r.valid).toBe(false);
      expect(r.error).toMatch(/too small/i);
      expect(r.fileSizeBytes).toBe(0);
    });
 
    it('rejects a buffer shorter than 8 bytes', () => {
      const r = parseWasm(new Uint8Array([0x00, 0x61, 0x73, 0x6d, 0x01]).buffer);
      expect(r.valid).toBe(false);
      expect(r.error).toMatch(/too small/i);
    });
 
    it('rejects a buffer with a wrong magic number', () => {
      const bad = new Uint8Array([0xde, 0xad, 0xbe, 0xef, 0x01, 0x00, 0x00, 0x00]);
      const r = parseWasm(bad.buffer);
      expect(r.valid).toBe(false);
      expect(r.error).toMatch(/magic/i);
    });
 
    it('reports fileSizeBytes even when invalid', () => {
      const bad = new Uint8Array(20);
      bad.set([0xde, 0xad, 0xbe, 0xef]);
      const r = parseWasm(bad.buffer);
      expect(r.valid).toBe(false);
      expect(r.fileSizeBytes).toBe(20);
    });
  });
 
  describe('parseWasm() — header-only module', () => {
    it('accepts the magic number and reads the version', () => {
      const r = parseWasm(makeHeaderOnly().buffer);
      expect(r.valid).toBe(true);
      expect(r.version).toBe(1);
      expect(r.sections).toEqual([]);
      expect(r.imports).toEqual([]);
      expect(r.exports).toEqual([]);
    });
  });
 
  describe('parseWasm() — real fixture (A.wasm)', () => {
    const result = parseWasm(A_WASM.buffer);
 
    it('marks the module valid and reports file size', () => {
      expect(result.valid).toBe(true);
      expect(result.fileSizeBytes).toBe(83);
      expect(result.version).toBe(1);
    });
 
    it('discovers all five real sections (type, import, function, export, code)', () => {
      const ids = result.sections.map((s) => s.id).sort((a, b) => a - b);
      expect(ids).toEqual([1, 2, 3, 7, 10]);
    });
 
    it('names sections from SECTION_NAMES', () => {
      const names = result.sections.map((s) => s.name);
      expect(names).toContain('Type');
      expect(names).toContain('Import');
      expect(names).toContain('Function');
      expect(names).toContain('Export');
      expect(names).toContain('Code');
    });
 
    it('parses the single import as my_namespace.imported_func / func', () => {
      expect(result.imports).toHaveLength(1);
      expect(result.imports[0]).toMatchObject({
        module: 'my_namespace',
        name: 'imported_func',
        kind: 0,
        kindName: 'func',
      });
    });
 
    it('parses the single export as exported_func / func', () => {
      expect(result.exports).toHaveLength(1);
      expect(result.exports[0]).toMatchObject({
        name: 'exported_func',
        kind: 0,
        kindName: 'func',
      });
    });
 
    it('reports functionCount = 1 from the function section', () => {
      expect(result.functionCount).toBe(1);
    });
 
    it('reports zero counts for sections that are absent', () => {
      expect(result.memoryCount).toBe(0);
      expect(result.globalCount).toBe(0);
      expect(result.dataSegments).toBe(0);
    });
  });
 
  describe('SECTION_NAMES / IMPORT_KIND_NAMES exports', () => {
    it('maps section IDs to canonical names', () => {
      expect(SECTION_NAMES[0]).toBe('Custom');
      expect(SECTION_NAMES[10]).toBe('Code');
      expect(SECTION_NAMES[12]).toBe('Data Count');
    });
 
    it('maps import kinds 0..3', () => {
      expect(IMPORT_KIND_NAMES[0]).toBe('func');
      expect(IMPORT_KIND_NAMES[1]).toBe('table');
      expect(IMPORT_KIND_NAMES[2]).toBe('memory');
      expect(IMPORT_KIND_NAMES[3]).toBe('global');
    });
  });
 
  describe('analyseWasm() — security analysis', () => {
    it('returns grade A and zero score for an invalid module (no analysis)', () => {
      const invalid = parseWasm(new ArrayBuffer(0));
      const r = analyseWasm(invalid);
      expect(r.grade).toBe('A');
      expect(r.riskScore).toBe(0);
      expect(r.findings).toEqual([]);
    });
 
    it('returns a clean grade A on the small A.wasm fixture', () => {
      const r = analyseWasm(parseWasm(A_WASM.buffer));
      expect(r.grade).toBe('A');
      expect(r.riskScore).toBeLessThanOrEqual(15);
    });
 
    it('flags a debug "name" custom section as info-level', () => {
      const parsed = parseWasm(A_WASM.buffer);
      parsed.customSections = ['name'];
      const r = analyseWasm(parsed);
      expect(r.findings.some((f) => f.id === 'debug_names')).toBe(true);
    });
 
    it('flags high global counts as a medium-severity complexity finding', () => {
      const parsed = parseWasm(A_WASM.buffer);
      parsed.globalCount = 50;
      const r = analyseWasm(parsed);
      const finding = r.findings.find((f) => f.id === 'many_globals');
      expect(finding).toBeDefined();
      expect(finding!.level).toBe('medium');
    });
 
    it('caps the risk score at 100', () => {
      const parsed = parseWasm(A_WASM.buffer);
      parsed.globalCount = 9999;
      parsed.functionCount = 9999;
      parsed.customSections = ['name'];
      const r = analyseWasm(parsed);
      expect(r.riskScore).toBeLessThanOrEqual(100);
    });
 
    it('sorts findings critical → high → medium → info', () => {
      const parsed = parseWasm(A_WASM.buffer);
      parsed.globalCount = 50;
      parsed.functionCount = 200;
      parsed.customSections = ['name'];
      const r = analyseWasm(parsed);
      const order = ['critical', 'high', 'medium', 'info'];
      for (let i = 1; i < r.findings.length; i++) {
        expect(order.indexOf(r.findings[i - 1].level))
          .toBeLessThanOrEqual(order.indexOf(r.findings[i].level));
      }
    });
  });
 
  describe('hexDump()', () => {
    it('emits 16 bytes per row in lowercase hex', () => {
      const bytes = new Uint8Array(16).fill(0xab);
      const out = hexDump(bytes);
      expect(out).toContain('ab ab ab ab ab ab ab ab ab ab ab ab ab ab ab ab');
    });
 
    it('honours maxBytes (default 256) and appends a truncation marker', () => {
      const bytes = new Uint8Array(1024);
      const out = hexDump(bytes);
      expect(out.split('\n')).toHaveLength(17);
      expect(out).toMatch(/\.\.\. \(768 more bytes\)/);
    });
 
    it('respects an explicit maxBytes (with truncation marker)', () => {
      const bytes = new Uint8Array(1024);
      const out = hexDump(bytes, 32);
      expect(out.split('\n')).toHaveLength(3);
      expect(out).toMatch(/\.\.\. \(992 more bytes\)/);
    });
 
    it('omits the truncation marker when bytes.length <= maxBytes', () => {
      const bytes = new Uint8Array(16);
      const out = hexDump(bytes);
      expect(out.split('\n')).toHaveLength(1);
      expect(out).not.toMatch(/more bytes/);
    });
 
    it('renders printable bytes as ASCII and others as dots', () => {
      const bytes = new Uint8Array([0x41, 0x42, 0x43, 0x00, 0x01]);
      const out = hexDump(bytes);
      expect(out).toContain('ABC');
      expect(out).toContain('..');
    });
  });
}
