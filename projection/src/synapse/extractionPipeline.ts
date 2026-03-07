/**
 * Extraction Pipeline — Orchestrates the full AI extraction flow.
 *
 * Flow:
 *   1. User pastes lab text
 *   2. buildExtractionPrompt() constructs the Gemini prompt
 *   3. Call the Hono API proxy → Gemini
 *   4. Parse the raw JSON response
 *   5. Zod validates the structure (schemas.ts)
 *   6. On success → return ExtractionResult
 *   7. On failure → optionally retry with buildRefinementPrompt()
 */

import { validateExtraction, type ExtractionResult } from './schemas';
import { buildExtractionPrompt, buildRefinementPrompt } from './promptBuilder';

// ── Configuration ────────────────────────────────────────────────────
const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:8787';
const MAX_RETRIES = 2;

// ── Types ────────────────────────────────────────────────────────────
export interface ExtractionSuccess {
  status: 'success';
  data: ExtractionResult;
  retries: number;
}

export interface ExtractionError {
  status: 'error';
  message: string;
  errors?: string[];
  rawOutput?: string;
}

export type ExtractionResponse = ExtractionSuccess | ExtractionError;

// ── Main extraction function ─────────────────────────────────────────
export async function extractCircuit(labText: string): Promise<ExtractionResponse> {
  if (!labText.trim()) {
    return { status: 'error', message: 'Lab text is empty.' };
  }

  let lastRawOutput = '';
  let lastErrors: string[] = [];

  for (let attempt = 0; attempt <= MAX_RETRIES; attempt++) {
    // Build prompt (first attempt or refinement)
    const prompt =
      attempt === 0
        ? buildExtractionPrompt(labText)
        : buildRefinementPrompt(labText, lastRawOutput, lastErrors);

    // Call the API proxy
    let rawJson: string;
    try {
      rawJson = await callGeminiProxy(prompt);
    } catch (err) {
      return {
        status: 'error',
        message: `API call failed: ${err instanceof Error ? err.message : String(err)}`,
      };
    }

    lastRawOutput = rawJson;

    // Smart JSON extraction (handles ```json ... ``` blocks)
    const cleanedJson = extractJson(rawJson);
    
    // Parse the cleaned JSON
    let parsed: unknown;
    try {
      parsed = JSON.parse(cleanedJson);
    } catch {
      const snippet = rawJson.length > 200 ? rawJson.slice(0, 200) + '...' : rawJson;
      lastErrors = [`Invalid JSON. Failed to parse: "${snippet}"`];
      
      if (attempt === MAX_RETRIES) {
        return {
          status: 'error',
          message: 'Gemini returned invalid JSON structure.',
          errors: lastErrors,
          rawOutput: rawJson,
        };
      }
      continue;
    }

    // Validate with Zod
    const validation = validateExtraction(parsed);

    if (validation.success) {
      return {
        status: 'success',
        data: validation.data,
        retries: attempt,
      };
    }

    // Validation failed — store errors for refinement
    lastErrors = validation.errors;

    if (attempt === MAX_RETRIES) {
      return {
        status: 'error',
        message: `Extraction failed after ${MAX_RETRIES + 1} attempts. Zod validation errors remain.`,
        errors: lastErrors,
        rawOutput: rawJson,
      };
    }

    // Will retry with refinement prompt on next iteration
  }

  // Should never reach here, but TypeScript needs it
  return { status: 'error', message: 'Unexpected extraction failure.' };
}

// ── Gemini API Proxy Call ────────────────────────────────────────────
async function callGeminiProxy(prompt: string): Promise<string> {
  const response = await fetch(`${API_BASE}/api/gemini`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ prompt }),
  });

  if (!response.ok) {
    const errBody = await response.text();
    throw new Error(`API returned ${response.status}: ${errBody}`);
  }

  const data = await response.json();

  // Extract the text content from Gemini's response format
  // Gemini returns: { candidates: [{ content: { parts: [{ text: "..." }] } }] }
  const text = data?.candidates?.[0]?.content?.parts?.[0]?.text;

  if (!text) {
    throw new Error('No text content in Gemini response');
  }

  return text;
}

/**
 * Helper to strip markdown formatting (like ```json ... ```) 
 * that Gemini sometimes includes even when asked not to.
 */
function extractJson(text: string): string {
  // If text contains ```json ... ```, extract the content
  const match = text.match(/```json\s?([\s\S]*?)\s?```/);
  if (match && match[1]) {
    return match[1].trim();
  }
  
  // If it's just ``` ... ```
  const genericMatch = text.match(/```\s?([\s\S]*?)\s?```/);
  if (genericMatch && genericMatch[1]) {
    return genericMatch[1].trim();
  }

  return text.trim();
}
