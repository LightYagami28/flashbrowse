/**
 * Chat completions for task eval: OpenAI or any OpenAI-compatible API
 * (OpenRouter via OPENROUTER_API_KEY).
 */

function apiKey() {
  const key = process.env.OPENROUTER_API_KEY ?? process.env.OPENAI_API_KEY;
  if (!key) {
    throw new Error(
      "Set OPENROUTER_API_KEY or OPENAI_API_KEY for agent runs (or use --dry-run)",
    );
  }
  return key;
}

function baseUrl() {
  if (process.env.LLM_BASE_URL) {
    return process.env.LLM_BASE_URL.replace(/\/$/, "");
  }
  if (process.env.OPENROUTER_API_KEY) {
    return "https://openrouter.ai/api/v1";
  }
  return "https://api.openai.com/v1";
}

export function defaultModel() {
  return (
    process.env.OPENROUTER_MODEL ??
    process.env.OPENAI_MODEL ??
    (process.env.OPENROUTER_API_KEY
      ? "nvidia/nemotron-3.5-super-vl:free"
      : "gpt-4o-mini")
  );
}

export async function chat(model, messages, tools) {
  const key = apiKey();
  const url = `${baseUrl()}/chat/completions`;
  const headers = {
    Authorization: `Bearer ${key}`,
    "Content-Type": "application/json",
  };
  if (process.env.OPENROUTER_API_KEY) {
    headers["HTTP-Referer"] =
      process.env.OPENROUTER_HTTP_REFERER ?? "https://github.com/flashbrowse";
    headers["X-Title"] = process.env.OPENROUTER_X_TITLE ?? "flashbrowse-eval";
  }
  const res = await fetch(url, {
    method: "POST",
    headers,
    body: JSON.stringify({
      model,
      temperature: 0,
      messages,
      tools,
    }),
  });
  const body = await res.json();
  if (!res.ok) {
    const msg =
      body.error?.message ??
      (typeof body.error === "string" ? body.error : JSON.stringify(body));
    throw new Error(msg);
  }
  const message = body.choices[0].message;
  message.usage = body.usage;
  return message;
}