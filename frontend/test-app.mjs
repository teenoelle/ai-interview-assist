import { chromium } from 'playwright';

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage();

// Capture console errors
const errors = [];
page.on('console', msg => { if (msg.type() === 'error') errors.push(msg.text()); });
page.on('pageerror', err => errors.push('PAGE ERROR: ' + err.message));

console.log('1. Loading app...');
await page.goto('http://localhost:3000');
await page.waitForLoadState('networkidle');
console.log('   Title:', await page.title());

// Check setup form is visible
const h2 = await page.textContent('h2');
console.log('   Page heading:', h2);

// Fill in job description
console.log('2. Filling setup form...');
await page.fill('textarea#job-desc', 'Senior Software Engineer, Python, distributed systems, AWS');
await page.fill('textarea#extra', '5 years backend engineering, led team of 8');

// Click Start Session
console.log('3. Clicking Start Session...');
await page.click('button:has-text("Start Session")');

// Wait for setup to complete (up to 30s)
console.log('   Waiting for setup to complete...');
try {
  await page.waitForSelector('.setup-success', { timeout: 30000 });
  console.log('   Setup complete banner visible!');
} catch (e) {
  console.log('   ERROR: Setup success banner not found:', e.message);
  if (errors.length) console.log('   Console errors:', errors);
  await browser.close();
  process.exit(1);
}

// Check Start Interview button
const startBtn = page.locator('button:has-text("Start Interview")');
const startVisible = await startBtn.isVisible();
console.log('4. Start Interview button visible:', startVisible);

// Check Practice First button
const practiceBtn = page.locator('button:has-text("Practice First")');
const practiceVisible = await practiceBtn.isVisible();
console.log('   Practice First button visible:', practiceVisible);

// Test Practice First first
if (practiceVisible) {
  console.log('5. Clicking Practice First...');
  await practiceBtn.click();
  await page.waitForTimeout(500);
  const practicePanel = await page.locator('text=Practice Mode').isVisible();
  console.log('   Practice Mode visible:', practicePanel);
  
  // Go back - click "I\'m Ready" button
  const readyBtn = page.locator("button:has-text(\"I'm Ready\")");
  if (await readyBtn.isVisible()) {
    console.log('   Clicking Im Ready to go to interview...');
    await readyBtn.click();
    await page.waitForTimeout(500);
  }
}

// Now test Start Interview
// Reload to get back to setup or check if we're in interview mode
const interviewHeader = await page.locator('text=AI Suggestions').isVisible();
console.log('6. Interview view (AI Suggestions panel) visible:', interviewHeader);

// Check for any errors
if (errors.length > 0) {
  console.log('CONSOLE ERRORS DETECTED:', errors);
} else {
  console.log('No console errors!');
}

// Now reload and test Start Interview directly
console.log('\n--- Testing Start Interview button directly ---');
await page.goto('http://localhost:3000');
await page.waitForLoadState('networkidle');
await page.fill('textarea#job-desc', 'Backend Engineer role');
await page.click('button:has-text("Start Session")');
await page.waitForSelector('.setup-success', { timeout: 30000 });

const startBtn2 = page.locator('button:has-text("Start Interview")');
console.log('Start Interview button exists:', await startBtn2.isVisible());
await startBtn2.click();
await page.waitForTimeout(1000);

const inInterview = await page.locator('text=AI Suggestions').isVisible();
console.log('After clicking Start Interview - AI Suggestions visible:', inInterview);

const errorMsg = await page.locator('.error').isVisible().catch(() => false);
if (errorMsg) {
  const errorText = await page.locator('.error').textContent().catch(() => '');
  console.log('Error message shown:', errorText);
}

if (errors.length > 0) {
  console.log('Console errors:', errors);
}

await browser.close();
console.log('\nTest complete!');
