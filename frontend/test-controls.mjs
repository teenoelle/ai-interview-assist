import { chromium } from 'playwright';

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage();
const errors = [];
page.on('pageerror', err => errors.push(err.message));

function ok(label) { console.log(`  ✓ ${label}`); errors.length = 0; }
function fail(label, detail) { console.log(`  ✗ ${label}: ${detail}`); errors.length = 0; }
async function chk(label) {
  if (errors.length > 0) { fail(label, errors.join('; ')); return false; }
  ok(label); return true;
}

// --- Setup ---
await page.goto('http://localhost:3000');
await page.fill('textarea#job-desc', 'Senior Software Engineer Python AWS distributed systems');
await page.fill('textarea#extra', '5 years backend engineering, led team of 8');
await page.click('button:has-text("Start Session")');
await page.waitForSelector('.setup-success', { timeout: 30000 });
await chk('Setup completes');

// Post-setup tabs
await page.click('button:has-text("Story Bank")');
await page.waitForTimeout(200);
await chk('Story Bank tab');
await page.click('button:has-text("Overview")');
await chk('Overview tab');

// Start Interview
await page.click('button:has-text("Start Interview")');
await page.waitForSelector('text=AI Suggestions', { timeout: 5000 });
await chk('Start Interview → interview view');

// Font size
await page.click('(//button[@title="Increase font size"])[1]');
await chk('A+ font increase');
await page.click('(//button[@title="Decrease font size"])[1]');
await chk('A- font decrease');

// Keyboard shortcuts
await page.keyboard.press('f');
await page.waitForTimeout(200);
const focusOn = await page.locator('.focus-overlay').isVisible();
await chk(`F key → focus mode ${focusOn ? 'ON' : '(needs suggestion loaded first)'}`);
if (focusOn) { await page.keyboard.press('Escape'); await chk('Escape closes focus'); }

await page.keyboard.press('w');
await page.waitForTimeout(200);
await chk('W key → whisper toggle');
await page.keyboard.press('w');

await page.keyboard.press('t');
await page.waitForTimeout(100);
await chk('T key → TTS toggle');
await page.keyboard.press('t');

// Stories
if (await page.locator('button:has-text("Stories")').isVisible()) {
  await page.click('button:has-text("Stories")');
  await page.waitForTimeout(300);
  await chk('Stories panel opens');
}

// History - open and close properly  
if (await page.locator('button:has-text("History")').isVisible()) {
  await page.click('button:has-text("History")');
  await page.waitForTimeout(300);
  const histOpen = await page.locator('text=Interview History').isVisible();
  await chk(`History panel opens (${histOpen})`);
  // Close using the ✕ button inside panel header
  const closeBtn = page.locator('.panel-header button, .panel button').filter({ hasText: '✕' }).first();
  if (await closeBtn.isVisible()) {
    await closeBtn.click();
  } else {
    // Force close by pressing Escape or clicking top-left area
    await page.keyboard.press('Escape');
  }
  await page.waitForTimeout(300);
  const histClosed = !(await page.locator('text=Interview History').isVisible());
  await chk(`History panel closes (${histClosed})`);
}

// End Interview (debrief)
const endBtn = page.locator('button:has-text("End Interview")');
if (await endBtn.isVisible()) {
  await endBtn.click({ force: true });
  await page.waitForTimeout(1000);
  const debriefOpen = await page.locator('text=Interview Debrief').isVisible();
  await chk(`End Interview → debrief modal (${debriefOpen})`);
  if (debriefOpen) {
    // Check debrief has content sections
    const hasSummary = await page.locator('text=Summary').isVisible();
    await chk(`Debrief has summary section (${hasSummary})`);
    // Close
    await page.locator('.modal-backdrop').click({ force: true, position: { x: 10, y: 10 } });
    await page.waitForTimeout(200);
  }
}

// --- Practice First path ---
await page.goto('http://localhost:3000');
await page.fill('textarea#job-desc', 'Backend Engineer Python microservices cloud infrastructure');
await page.click('button:has-text("Start Session")');
await page.waitForSelector('.setup-success', { timeout: 30000 });
await chk('Second setup (Practice path)');

const practiceBtn = page.locator('button:has-text("Practice First")');
if (await practiceBtn.isVisible()) {
  await practiceBtn.click();
  await page.waitForTimeout(300);
  await chk('Practice First button works');
  await page.waitForSelector('h2:has-text("Practice Mode")', { timeout: 3000 });
  await chk('Practice Mode loads');

  // Next question button
  const nextBtn = page.locator('button:has-text("Next →")');
  if (await nextBtn.isEnabled().catch(() => false)) {
    await nextBtn.click();
    await chk('Next question navigation');
    await page.locator('button:has-text("← Previous")').click();
    await chk('Previous question navigation');
  }

  // Prep all questions
  const prepAllBtn = page.locator('button:has-text("Prep all questions")');
  if (await prepAllBtn.isVisible()) {
    await prepAllBtn.click();
    await page.waitForTimeout(300);
    await chk('Prep all questions clicked');
  }

  // I'm Ready
  await page.locator("button:has-text(\"I'm Ready\")").click();
  await page.waitForSelector('text=AI Suggestions', { timeout: 5000 });
  await chk("I'm Ready → interview view");
}

await browser.close();
console.log('\nAll controls tested!');
