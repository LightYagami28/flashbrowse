/**
 * Evaluate task success criteria via Playwright on a live page.
 */

/**
 * @param {import('playwright').Page} page
 * @param {object} criterion
 */
export async function checkCriterion(page, criterion) {
  const kind = criterion.kind;
  const sel = criterion.selector;
  if (kind === "dom_text") {
    const text = await page.locator(sel).innerText();
    if (criterion.equals != null && text.trim() !== criterion.equals) {
      return { ok: false, detail: `text ${JSON.stringify(text.trim())} != ${criterion.equals}` };
    }
    if (criterion.includes != null && !text.includes(criterion.includes)) {
      return { ok: false, detail: `text missing ${criterion.includes}` };
    }
    return { ok: true };
  }
  if (kind === "element_state") {
    const prop = criterion.prop ?? "value";
    const value = await page.locator(sel).evaluate((el, p) => {
      if (p === "value") return el.value;
      if (p === "checked") return el.checked;
      return el.getAttribute(p);
    }, prop);
    const got = value === true || value === false ? String(value) : String(value ?? "");
    if (criterion.equals != null && got !== criterion.equals) {
      return { ok: false, detail: `${prop}=${JSON.stringify(got)} != ${criterion.equals}` };
    }
    return { ok: true };
  }
  return { ok: false, detail: `unknown kind ${kind}` };
}

export async function checkSuccess(page, criteria) {
  const failures = [];
  for (const c of criteria ?? []) {
    const r = await checkCriterion(page, c);
    if (!r.ok) failures.push(r.detail);
  }
  return { ok: failures.length === 0, failures };
}