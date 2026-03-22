import type { HardwareInfo } from "../components/hardware";

// ── Quant bytes-per-parameter table ──────────────────────────────────────────

const QUANT_BPP: Record<string, number> = {
  IQ1: 0.19,
  Q1: 0.19,
  IQ2: 0.31,
  Q2: 0.31,
  IQ3: 0.38,
  Q3: 0.44,
  Q4: 0.56,
  IQ4: 0.56,
  Q5: 0.69,
  Q6: 0.81,
  Q8: 1.06,
  F16: 2.0,
  F32: 4.0,
};

// ── Parsers ───────────────────────────────────────────────────────────────────

/** Extract billions of parameters from a GGUF filename.
 *  e.g. "mistral-7b-..."  → 7,  "llama-3-70B-..." → 70
 */
function parseParamsBillion(filename: string): number | null {
  // Match patterns like -7b-, -70B-, -32B-Instruct, -7b.gguf
  const match = filename.match(/[-._](\d+(?:\.\d+)?)[bB](?=[-._]|$)/i);
  return match ? parseFloat(match[1]) : null;
}

/** Extract bytes-per-parameter from the quantization tag in a GGUF filename.
 *  e.g. "Q4_K_M" → 0.56,  "IQ2_XS" → 0.31,  "F16" → 2.0
 */
function parseBpp(filename: string): number | null {
  const upper = filename.toUpperCase();
  // Match quant tags like Q4_K_M, IQ2_XS, Q8_0, F16 — can't use \b because
  // underscore is a word character, so \bQ4\b won't match Q4_K_M.
  const match = upper.match(
    /(?<![A-Z0-9])(IQ[1-4]|Q[1-8]|F16|F32)(?=[_.\-]|$)/,
  );
  if (!match) return null;
  const key = match[1];
  return QUANT_BPP[key] ?? null;
}

// ── Memory estimation ─────────────────────────────────────────────────────────

const OVERHEAD = 1.15; // 15% for KV cache and runtime buffers

/** Estimate memory required to run a GGUF model in GB.
 *  Returns null if the filename doesn't contain enough info.
 */
export function estimateMemoryGb(filename: string): number | null {
  const params = parseParamsBillion(filename);
  const bpp = parseBpp(filename);
  if (params === null || bpp === null) return null;
  return params * bpp * OVERHEAD;
}

// ── Available memory ──────────────────────────────────────────────────────────

/** Returns usable memory in GB for inference.
 *  Prefers VRAM if available, falls back to 75% of RAM.
 */
export function availableMemoryGb(hw: HardwareInfo): number {
  if (hw.gpu?.vram_gb != null && hw.gpu.vram_gb > 0) {
    return hw.gpu.vram_gb;
  }
  return hw.memory.total_gb * 0.75;
}

// ── Compatibility check ───────────────────────────────────────────────────────

export interface CompatResult {
  compatible: boolean;
  requiredGb: number | null;
  availableGb: number;
}

export function checkCompat(filename: string, hw: HardwareInfo): CompatResult {
  const requiredGb = estimateMemoryGb(filename);
  const availableGb = availableMemoryGb(hw);
  return {
    compatible: requiredGb === null || requiredGb <= availableGb,
    requiredGb,
    availableGb,
  };
}
