/**
 * LabWise API — Cloudflare Workers + Hono
 *
 * Endpoints:
 *   POST /api/gemini   — Proxies prompts to Gemini 2.5 Flash (protects API key)
 *   GET  /api/health   — Health check
 *
 * The GEMINI_API_KEY is stored as a Cloudflare secret (or in .dev.vars for local dev).
 */

import { Hono } from "hono";
import { cors } from "hono/cors";

type Env = {
  Bindings: {
    GEMINI_API_KEY: string;
  };
};

const app = new Hono<Env>();

// ── CORS — allow the frontend to call this API ──────────────────────
app.use(
  "/api/*",
  cors({
    origin: ["http://localhost:5173", "http://localhost:4173"],
    allowMethods: ["GET", "POST", "OPTIONS"],
    allowHeaders: ["Content-Type"],
    maxAge: 86400,
  })
);

// ── Health Check ─────────────────────────────────────────────────────
app.get("/api/health", (c) => {
  return c.json({ status: "ok", service: "labwise-api", timestamp: new Date().toISOString() });
});

// ── Gemini Proxy ─────────────────────────────────────────────────────
app.post("/api/gemini", async (c) => {
  const apiKey = c.env.GEMINI_API_KEY;

  if (!apiKey) {
    return c.json({ error: "GEMINI_API_KEY not configured" }, 500);
  }

  let body: { prompt: string; model?: string };
  try {
    body = await c.req.json();
  } catch {
    return c.json({ error: "Invalid JSON body. Expected: { prompt: string, model?: string }" }, 400);
  }

  if (!body.prompt || typeof body.prompt !== "string") {
    return c.json({ error: "Missing or invalid 'prompt' field" }, 400);
  }

  const model = body.model || "gemini-2.0-flash";
  const geminiUrl = `https://generativelanguage.googleapis.com/v1beta/models/${model}:generateContent?key=${apiKey}`;

  try {
    const geminiResponse = await fetch(geminiUrl, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        contents: [
          {
            parts: [{ text: body.prompt }],
          },
        ],
        generationConfig: {
          temperature: 0.1,       // Low temp for deterministic circuit extraction
          maxOutputTokens: 8192,
        },
      }),
    });

    if (!geminiResponse.ok) {
      const errText = await geminiResponse.text();
      console.error("Gemini API error:", geminiResponse.status, errText);
      return c.json(
        { error: `Gemini API returned ${geminiResponse.status}`, details: errText },
        geminiResponse.status as any
      );
    }

    const geminiData = await geminiResponse.json();
    return c.json(geminiData);
  } catch (err) {
    console.error("Gemini proxy error:", err);
    return c.json({ error: "Failed to reach Gemini API", details: String(err) }, 502);
  }
});

export default app;
