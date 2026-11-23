/**
 * Evaluate task success on the live MCP browser session (not a fresh page).
 */

function criterionExpression(criterion) {
  const sel = JSON.stringify(criterion.selector);
  const kind = criterion.kind;
  if (kind === "dom_text") {
    if (criterion.equals != null) {
      return `(() => { const el = document.querySelector(${sel}); if (!el) return false; return el.innerText.trim() === ${JSON.stringify(criterion.equals)}; })()`;
    }
    if (criterion.includes != null) {
      return `(() => { const el = document.querySelector(${sel}); if (!el) return false; return el.innerText.includes(${JSON.stringify(criterion.includes)}); })()`;
    }
    return `(() => !!document.querySelector(${sel}))()`;
  }
  if (kind === "element_state") {
    const prop = criterion.prop ?? "value";
    const want = JSON.stringify(criterion.equals ?? "");
    return `(() => { const el = document.querySelector(${sel}); if (!el) return false; const p = ${JSON.stringify(prop)}; let v; if (p === "value") v = el.value; else if (p === "checked") v = String(el.checked); else v = el.getAttribute(p) ?? ""; return String(v) === ${want}; })()`;
  }
  if (kind === "evaluate") {
    return `(() => !!(${criterion.expression}))()`;
  }
  throw new Error(`unknown kind ${kind}`);
}

export async function checkSuccessMcp(mcp, criteria) {
  const failures = [];
  for (const c of criteria ?? []) {
    const expression = criterionExpression(c);
    const raw = await mcp.callTool("browser_evaluate", { expression });
    const ok = /^(true|1)$/i.test(String(raw).trim());
    if (!ok) {
      const label =
        c.kind === "evaluate"
          ? `evaluate got ${JSON.stringify(raw).slice(0, 80)}`
          : `${c.kind} ${c.selector}`;
      failures.push(label);
    }
  }
  return { ok: failures.length === 0, failures };
}