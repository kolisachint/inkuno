// Screenshot harness for the prototype HTML (PROTO env, default v2.html).
// The right panel starts closed (v3); states that need it open it themselves.
// Each shot opens a fresh page (no state carried over), sets the theme,
// performs the actions that reach the state, then captures the window
// (#app) or the full viewport for dialogs that live outside it.
const path = require('path');
const fs = require('fs');
const { chromium } = require('playwright');

const SRC = 'file://' + (process.env.PROTO || require('path').resolve(__dirname, '../../../docs/design/prototypes/v2.html'));
const OUT = process.env.OUT || require('path').resolve(__dirname, '../../../target/visual/proto');
const VIEW = { width: 1300, height: 940 };
const THEMES = ['light', 'dark'];

const sleep = ms => new Promise(r => setTimeout(r, ms));

async function loadPage(page) {
  try {
    await page.goto(SRC, { waitUntil: 'load', timeout: 20000 });
  } catch (e) {
    await page.goto(SRC, { waitUntil: 'domcontentloaded', timeout: 20000 });
  }
}

async function ready(page) {
  await page.evaluate(() => document.fonts.ready);
  await sleep(500);
}

// Select a substring inside the rendered note and fire the mouseup the prototype listens for.
async function selectText(page, needle) {
  await page.evaluate((needle) => {
    const doc = document.querySelector('.doc');
    const w = document.createTreeWalker(doc, NodeFilter.SHOW_TEXT);
    let n;
    while ((n = w.nextNode())) {
      const i = n.data.indexOf(needle);
      if (i >= 0) {
        const r = document.createRange();
        r.setStart(n, i);
        r.setEnd(n, i + needle.length);
        const s = getSelection();
        s.removeAllRanges();
        s.addRange(r);
        document.dispatchEvent(new MouseEvent('mouseup', { bubbles: true }));
        return true;
      }
    }
    throw new Error('text not found: ' + needle);
  }, needle);
  await sleep(200);
}

const openSettings = p => p.click('#side [data-act="settings"]');
const settingsPage = (name) => async p => {
  await openSettings(p);
  if (name !== 'Appearance') await p.click(`.snav [data-page="${name}"]`);
};

// Each entry: id (file prefix), pre (actions after load), check (selector that must exist), target ('app' | 'viewport').
const STATES = [
  { id: '01-default', check: '#app .doc' },
  { id: '02-sidebar-hidden', pre: p => p.keyboard.press('Control+b'), check: '.app.noside' },
  { id: '03-panel-outline', pre: p => p.keyboard.press('Control+Alt+1'), check: '#rpanel .rsw .ib.on[data-view="outline"]' },
  { id: '04-panel-comments', pre: p => p.keyboard.press('Control+Alt+2'), check: '#rpanel .cc' },
  { id: '05-panel-versions', pre: p => p.keyboard.press('Control+Alt+3'), check: '#rpanel .vrow' },
  { id: '06-panel-assistant', pre: p => p.keyboard.press('Control+Alt+4'), check: '#rpanel .ask' },
  { id: '07-panel-closed', pre: async p => { await p.click('#band [data-act="rtog"]'); await p.click('#band [data-act="rtog"]'); }, check: '.app.rhide' },
  { id: '08-palette', pre: p => p.keyboard.press('Control+p'), check: '.pal' },
  { id: '09-palette-commands', pre: async p => { await p.keyboard.press('Control+p'); await p.keyboard.type('>'); }, check: '.pal .pr' },
  { id: '10-slash-menu', pre: p => p.click('[data-act="slash"]'), check: '.slash' },
  { id: '11-selection-toolbar', pre: p => selectText(p, 'Everything in this note is GitHub Markdown'), check: '.fbar' },
  { id: '12-context-menu-file', pre: p => p.click('#side [data-ctx="road"]', { button: 'right' }), check: '.pop[role="menu"]' },
  { id: '13-context-menu-note', skip: 'prototype has no note context menu: the contextmenu handler only matches sidebar rows [data-ctx]' },
  { id: '14a-settings-appearance', pre: settingsPage('Appearance'), check: '.snav .row.sel[data-page="Appearance"]' },
  { id: '14b-settings-editor', pre: settingsPage('Editor'), check: '.snav .row.sel[data-page="Editor"]' },
  { id: '14c-settings-files', pre: settingsPage('Files'), check: '.snav .row.sel[data-page="Files"]' },
  { id: '14d-settings-shortcuts', pre: settingsPage('Shortcuts'), check: '.snav .row.sel[data-page="Shortcuts"]' },
  { id: '14e-settings-about', pre: settingsPage('About'), check: '.snav .row.sel[data-page="About"]' },
  { id: '15-focus-mode', pre: p => p.keyboard.press('F9'), check: '.app.focus' },
  { id: '16-split', pre: p => p.keyboard.press('Control+Backslash'), check: '#panes .pane:nth-child(2)' },
  { id: '17-titlebar-hidden', pre: p => p.keyboard.press('Control+Shift+S'), check: '.app.bandhide' },
  {
    id: '18-tabs-multiple',
    pre: async p => {
      await p.dblclick('#side [data-open="road"]');   // double-click keeps the tab
      await p.click('#side [data-open="read"]');      // single click: preview tab
    },
    check: '.tab.prev'
  },
  {
    id: '19-empty-pane',
    pre: async p => {
      await p.dblclick('#side [data-open="road"]');
      await p.keyboard.press('Control+w');
      await p.keyboard.press('Control+w');
    },
    check: '.empty'
  },
  {
    id: '20-toast',
    pre: async p => {
      await p.click('#side [data-ctx="ideas"]', { button: 'right' });
      await p.click('.pop .mi:has-text("Move to Trash")');
    },
    check: '.toast'
  },
  { id: '21-line-revealed', pre: p => p.locator('#app .doc .b', { hasText: 'Spacing between blocks' }).first().click(), check: '.b.live' },
  {
    id: '22-compare-version',
    pre: async p => {
      await p.keyboard.press('Control+Alt+3');
      await p.click('#rpanel [data-ver="0"]');
    },
    check: '.cmpbar'
  },
  {
    id: '23-scratch-section-expanded',
    pre: async p => {
      await p.click('#side [data-fold="Scratch"]');  // collapse
      await p.click('#side [data-fold="Scratch"]');  // expand again
    },
    check: '#side [data-open="s1"]'
  },
  // Extra dialogs and popovers found in the code
  { id: '30-design-log', target: 'viewport', pre: p => p.click('button[data-act="dlog"]'), check: '#dlog' },
  { id: '31-try-guide', pre: p => p.click('button[data-act="whattry"]'), check: '.guide' },
  { id: '32-theme-picker', pre: async p => { await settingsPage('Appearance')(p); await p.click('[data-act="themes"]'); }, check: '.pal .pr' },
  {
    id: '33-trash-info-toast',
    pre: async p => { await settingsPage('Files')(p); await p.click('[data-act="trashinfo"]'); },
    check: '.toast'
  },
  {
    id: '34-link-to-note-picker',
    pre: async p => { await p.keyboard.press('/'); await p.click('.slash .mi:has-text("Link to note")'); },
    check: '.slash .mi'
  },
  {
    id: '35-link-to-date-picker',
    pre: async p => { await p.keyboard.press('/'); await p.click('.slash .mi:has-text("Link to date")'); },
    check: '.slash .mi'
  },
  {
    id: '36-review-pending-pill',
    pre: async p => { await p.keyboard.press('Control+Alt+4'); await p.click('#rpanel [data-act="review"]'); await sleep(1300); },
    check: '.pill'
  },
  {
    id: '37-comment-draft',
    pre: async p => {
      await selectText(p, 'Everything in this note is GitHub Markdown');
      await p.click('.fbar button[aria-label="Comment"]');
    },
    check: '#cdraft'
  },
  { id: '38-palette-query', pre: async p => { await p.keyboard.press('Control+p'); await p.keyboard.type('road'); }, check: '.pal .pr' },
  {
    id: '39-shortcuts-listening',
    pre: async p => { await settingsPage('Shortcuts')(p); await p.click('.kchip[data-key="0"]'); },
    check: '.kchip.listen'
  },
  {
    id: '40-titlebar-peek',
    pre: async p => {
      await p.keyboard.press('Control+Shift+S');
      const b = await p.locator('#strip').boundingBox();
      await p.mouse.move(b.x + b.width / 2, b.y + b.height / 2);
      await sleep(400);
    },
    check: '#band.peek'
  }
];

(async () => {
  fs.mkdirSync(OUT, { recursive: true });
  const browser = await chromium.launch();
  const ctxs = {};
  for (const t of THEMES) {
    ctxs[t] = await browser.newContext({ viewport: VIEW, deviceScaleFactor: 1, colorScheme: t });
  }

  const results = [];
  const skipped = [];
  for (const st of STATES) {
    if (st.skip) { skipped.push({ id: st.id, why: st.skip }); continue; }
    for (const theme of THEMES) {
      const file = `${st.id}-${theme}.png`;
      const page = await ctxs[theme].newPage();
      const r = { file, check: '', error: '' };
      const errs = [];
      try {
        page.on('pageerror', e => errs.push('pageerror: ' + e.message));
        page.on('console', m => { if (m.type() === 'error') errs.push('console: ' + m.text()); });
        page.on('requestfailed', q => { if (!q.url().includes('fonts.googleapis') && !q.url().includes('fonts.gstatic')) errs.push('requestfailed: ' + q.url()); });
        await loadPage(page);
        await page.evaluate(t => { document.documentElement.dataset.theme = t; }, theme);
        await ready(page);
        if (st.pre) await st.pre(page);
        await sleep(400); // let transitions settle
        const dest = path.join(OUT, file);
        if (st.target === 'viewport') {
          await page.screenshot({ path: dest });
        } else {
          await page.locator('#app').screenshot({ path: dest });
        }
        if (st.check) {
          const ok = await page.evaluate(sel => !!document.querySelector(sel), st.check);
          r.check = ok ? 'ok' : `MISSING ${st.check}`;
        }
      } catch (e) {
        r.error = String(e.message || e).split('\n')[0];
      } finally {
        await page.close();
      }
      r.errs = errs;
      results.push(r);
      if (errs.length) console.log('  console:', file, errs.join(' | '));
      console.log(`${r.error ? 'ERR ' : ''}${file} ${r.check}${r.error ? ' ' + r.error : ''}`);
    }
  }
  await browser.close();

  const pngs = fs.readdirSync(OUT).filter(f => f.endsWith('.png')).length;
  console.log(`\nPNG count: ${pngs}`);
  console.log('Skipped:', JSON.stringify(skipped));
  const bad = results.filter(r => r.error || (r.check && !r.check.startsWith('ok')) || r.errs.length);
  console.log(`Console errors: ${results.reduce((n, r) => n + r.errs.length, 0)}`);
  console.log(`Problems: ${bad.length}`);
  bad.forEach(b => console.log('  ', b.file, b.check, b.error));
})().catch(e => { console.error(e); process.exit(1); });
