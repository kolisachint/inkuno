const { chromium } = require('playwright');
const fs = require('fs');
const D = process.env.OUT || require('path').resolve(__dirname, '../../../target/visual/wf-proto');
const URL = 'file://' + (process.env.PROTO || require('path').resolve(__dirname, '../../../docs/design/prototypes/v2.html'));
require('fs').mkdirSync(D,{recursive:true});
const results = [];

const SUMMARY = () => ({
  tabs: S.panes.map((p, i) => ({ f: S.focus === i ? 1 : 0, t: p.tabs.map(t => (t.k === 'note' ? NOTES[t.id].title : t.k === 'set' ? 'Settings' : 'Cmp:' + t.ver.m) + (t.preview ? '(prev)' : '') + (p.act === t ? '*' : '')) })),
  cls: [...app.classList].join(' '),
  side: S.side, rview: S.rview, rhide: S.rhide, mode: S.mode, page: S.page, reveal: S.reveal, focusMode: S.focusMode,
  toast: document.querySelector('.toast')?.innerText || '',
  pill: document.querySelector('.pill')?.innerText || '',
  pop: [...document.querySelectorAll('.pop')].map(p => p.className).join('|'),
  fbar: !!document.querySelector('.fbar'),
  slash: !!document.querySelector('.slash'),
  pal: !!document.querySelector('.pal'),
  sideTitles: [...document.querySelectorAll('#side .tree .row, #side .scr .row')].map(r => r.innerText.trim()).join(' / '),
  palInput: document.getElementById('palq')?.value ?? null,
  live: (S.panes[S.focus].act && S.panes[S.focus].act.live) ?? null,
});

async function wordPoint(page, word) {
  return page.evaluate((w) => {
    const walker = document.createTreeWalker(document.querySelector('.doc'), NodeFilter.SHOW_TEXT);
    let n;
    while ((n = walker.nextNode())) {
      const i = n.textContent.indexOf(w);
      if (i >= 0) {
        const r = document.createRange(); r.setStart(n, i); r.setEnd(n, i + w.length);
        n.parentElement.scrollIntoView({ block: 'center' }); const r2 = document.createRange(); r2.setStart(n, i); r2.setEnd(n, i + w.length); const b = r2.getBoundingClientRect(); return { x: b.x + b.width / 2, y: b.y + b.height / 2 };
      }
    }
    return null;
  }, word);
}

async function fresh(browser) {
  const ctx = await browser.newContext({ viewport: { width: 1300, height: 940 }, deviceScaleFactor: 1, colorScheme: 'light' });
  const page = await ctx.newPage();
  await page.goto(URL);
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(300);
  return page;
}

async function step(page, scn, name, desc, sel, fn) {
  let err = null;
  try { await fn(); } catch (e) { err = e.message.split('\n')[0].slice(0, 160); }
  await page.waitForTimeout(300);
  await page.locator('#app').screenshot({ path: `${D}/${scn}-${name}.png` });
  const st = await page.evaluate(SUMMARY);
  results.push({ scn, name, desc, sel, err, st });
  console.log(`${scn}-${name} | ${desc} | err=${err} | tabs=${JSON.stringify(st.tabs)} | toast=${st.toast} | pop=${st.pop} | fbar=${st.fbar} slash=${st.slash} pal=${st.pal} | cls=${st.cls}`);
}

(async () => {
  const browser = await chromium.launch();
  const ready = async () => fresh(browser);
  const nodeEval = (page, fn, arg) => page.evaluate(fn, arg);

  // ---------- W1 first-note ----------
  {
    const page = await ready();
    await step(page, 'W1', '01', 'start state', 'none', async () => {});
    await step(page, 'W1', '02', 'new note', 'Control+Shift+N', async () => { await page.keyboard.press('Control+Shift+N'); });
    await step(page, 'W1', '03', 'type # Groceries / - [ ] milk / - [ ] eggs', 'keyboard.type on focused body', async () => {
      await page.keyboard.type('# Groceries'); await page.keyboard.press('Enter');
      await page.keyboard.type('- [ ] milk'); await page.keyboard.press('Enter');
      await page.keyboard.type('- [ ] eggs');
    });
    await step(page, 'W1', '04', 'click elsewhere in pane', '.pscroll click', async () => {
      await page.locator('.pscroll').first().click({ position: { x: 60, y: 420 } });
    });
    await step(page, 'W1', '05', 'toggle first task box (Week plan)', '#side [data-open=week], .box', async () => {
      await page.locator('#side [data-open="week"]').click();
      await page.locator('.box').first().click();
    });
    await step(page, 'W1', '06', 'note in sidebar tree', 'screenshot sidebar', async () => {});
  }

  // ---------- W2 navigate ----------
  {
    const page = await ready();
    await step(page, 'W2', '01', 'Ctrl+P palette', 'Control+p', async () => { await page.keyboard.press('Control+p'); });
    await step(page, 'W2', '02', 'type road', 'keyboard.type road', async () => { await page.keyboard.type('road'); });
    await step(page, 'W2', '03', 'Enter opens Roadmap', 'Enter', async () => { await page.keyboard.press('Enter'); });
    await step(page, 'W2', '04', 'single-click Ideas (preview)', '#side [data-open=ideas]', async () => { await page.locator('#side [data-open="ideas"]').click(); });
    await step(page, 'W2', '05', 'double-click Ideas (pinned)', '#side [data-open=ideas] dblclick', async () => { await page.locator('#side [data-open="ideas"]').dblclick(); });
    await step(page, 'W2', '06', 'Ctrl+W closes tab', 'Control+w', async () => { await page.keyboard.press('Control+w'); });
    await step(page, 'W2', '07', 'Ctrl+Shift+T reopen', 'Control+Shift+T', async () => { await page.keyboard.press('Control+Shift+T'); });
  }

  // ---------- W3 format ----------
  {
    const page = await ready();
    await step(page, 'W3', '01', 'double-click word "quiet" shows selection toolbar', 'dblclick word', async () => {
      const p = await wordPoint(page, 'quiet'); await page.mouse.dblclick(p.x, p.y);
    });
    await step(page, 'W3', '02', 'Bold via toolbar', '.fbar button[title^=Bold]', async () => {
      await page.locator('.fbar button[title^="Bold"]').click();
    });
    await step(page, 'W3', '03', 'slash menu on new line', 'press /', async () => { await page.keyboard.press('/'); });
    await step(page, 'W3', '04', 'pick Table from slash menu', '.slash .mi:has-text(Table)', async () => {
      await page.locator('.slash .mi:has-text("Table")').first().click();
    });
    await step(page, 'W3', '05', 'click rendered heading reveals Markdown', '.doc .b.k-h click', async () => {
      await page.locator('.doc .b.k-h').first().click();
    });
  }

  // ---------- W4 split ----------
  {
    const page = await ready();
    await step(page, 'W4', '01', 'Ctrl+\\ split right', 'Control+Backslash', async () => { await page.keyboard.press('Control+Backslash'); });
    await step(page, 'W4', '02', 'open Roadmap in right pane', '#side [data-open=road]', async () => { await page.locator('#side [data-open="road"]').click(); });
    await step(page, 'W4', '03', 'Ctrl+1 focus left', 'Control+1', async () => { await page.keyboard.press('Control+1'); });
    await step(page, 'W4', '04', 'close split (Ctrl+\\)', 'Control+Backslash', async () => { await page.keyboard.press('Control+Backslash'); });
  }

  // ---------- W5 right panel ----------
  {
    const page = await ready();
    await step(page, 'W5', '01', 'Ctrl+Shift+O outline', 'Control+Shift+O', async () => { await page.keyboard.press('Control+Shift+O'); });
    await step(page, 'W5', '02', 'click outline entry Tasks (scroll)', '.ol Tasks', async () => { await page.locator('.ol', { hasText: 'Tasks' }).first().click(); });
    await step(page, 'W5', '03', 'comments view Ctrl+Alt+2', 'Control+Alt+2', async () => { await page.keyboard.press('Control+Alt+2'); });
    await step(page, 'W5', '04', 'add comment on selection "quiet"', 'dblclick, .fbar Comment, #cdraft, Enter', async () => {
      const p = await wordPoint(page, 'quiet'); await page.mouse.dblclick(p.x, p.y);
      await page.waitForTimeout(150);
      await page.locator('.fbar button[aria-label="Comment"]').click();
      await page.locator('#cdraft').fill('Keep it short');
      await page.keyboard.press('Enter');
    });
    await step(page, 'W5', '05', 'resolve the new comment', '[data-act=resolve] (last)', async () => {
      await page.locator('[data-act="resolve"]').last().click();
    });
    await step(page, 'W5', '06', 'versions view Ctrl+Alt+3', 'Control+Alt+3', async () => { await page.keyboard.press('Control+Alt+3'); });
    await step(page, 'W5', '07', 'save named version Ctrl+Alt+S', 'Control+Alt+S, #vname, Enter', async () => {
      await page.keyboard.press('Control+Alt+KeyS');
      await page.keyboard.type('Before review');
      await page.keyboard.press('Enter');
    });
    await step(page, 'W5', '08', 'compare with new version', '.vrow first', async () => { await page.locator('.vrow').first().click(); });
    await step(page, 'W5', '09', 'assistant view Ctrl+Alt+4', 'Control+Alt+4', async () => { await page.keyboard.press('Control+Alt+4'); });
  }

  // ---------- W6 files ----------
  {
    const page = await ready();
    await step(page, 'W6', '01', 'right-click Roadmap (context menu)', 'contextmenu [data-ctx=road]', async () => { await page.locator('#side [data-ctx="road"]').click({ button: 'right' }); });
    await step(page, 'W6', '02', 'Rename', '.pop .mi Rename', async () => { await page.locator('.pop .mi:has-text("Rename")').click(); });
    await step(page, 'W6', '03', 'type new name + Enter', 'keyboard', async () => { await page.keyboard.type('Roadmap v2'); await page.keyboard.press('Enter'); });
    await step(page, 'W6', '04', 'New folder (probe: right-click Projects folder row)', 'contextmenu [data-fold=Projects]', async () => {
      await page.locator('#side [data-fold="Projects"]').click({ button: 'right' });
    });
    await step(page, 'W6', '05', 'Move to Trash (toast + Undo)', 'contextmenu road -> Move to Trash', async () => {
      await page.locator('#side [data-ctx="road"]').click({ button: 'right' });
      await page.locator('.pop .mi:has-text("Move to Trash")').click();
    });
    await step(page, 'W6', '06', 'Scratch: new scratch note', 'Control+n', async () => { await page.keyboard.press('Control+n'); });
  }

  // ---------- W7 chrome ----------
  {
    const page = await ready();
    await step(page, 'W7', '01', 'Ctrl+B hide sidebar', 'Control+B', async () => { await page.keyboard.press('Control+b'); });
    await step(page, 'W7', '02', 'Ctrl+Shift+S hide title bar', 'Control+Shift+S', async () => { await page.keyboard.press('Control+Shift+S'); });
    await step(page, 'W7', '03', 'hover strip, bar slides back', 'mouse over #strip', async () => {
      const b = await page.locator('#strip').boundingBox(); await page.mouse.move(b.x + b.width / 2, b.y + b.height / 2);
    });
    await step(page, 'W7', '04', 'F9 focus mode', 'F9', async () => { await page.mouse.move(600, 600); await page.keyboard.press('F9'); });
    await step(page, 'W7', '05', 'Esc leaves focus', 'Escape', async () => { await page.keyboard.press('Escape'); });
  }

  // ---------- W8 settings ----------
  {
    const page = await ready();
    await step(page, 'W8', '01', 'palette "> settings" -> Settings', 'Control+K, type ">settings", Enter', async () => {
      await page.keyboard.press('Control+k'); await page.keyboard.type('>settings'); await page.waitForTimeout(200); await page.keyboard.press('Enter');
    });
    await step(page, 'W8', '02', 'Appearance: Dark', '[data-set=mode][data-v=Dark]', async () => { await page.locator('[data-set="mode"][data-v="Dark"]').click(); });
    await step(page, 'W8', '03', 'Editor page', '[data-page=Editor]', async () => { await page.locator('[data-page="Editor"]').click(); });
    await step(page, 'W8', '04', 'Shortcuts page via F1', 'F1', async () => { await page.keyboard.press('F1'); });
    await step(page, 'W8', '05', 'toggle Show Markdown on current line (Editor)', '[data-page=Editor] [data-toggle=reveal]', async () => {
      await page.locator('[data-page="Editor"]').click();
      await page.locator('[data-toggle="reveal"]').click();
    });
    await step(page, 'W8', '06', 'close settings back to note', 'Control+w', async () => { await page.keyboard.press('Control+w'); });
  }

  fs.writeFileSync(`${D}/results.json`, JSON.stringify(results, null, 1));
  await browser.close();
  console.log('done', results.length);
})().catch(e => { console.error('FATAL', e); process.exit(1); });
