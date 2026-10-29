import { chromium } from '@playwright/test';
import { readFile, writeFile } from 'node:fs/promises';
const browser = await chromium.launch({
  headless: true,
  executablePath: process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE || '/usr/bin/google-chrome',
  args: ['--no-sandbox'],
});
const page = await browser.newPage({
  viewport: { width: 1680, height: 1200 },
  deviceScaleFactor: 1,
  reducedMotion: 'reduce',
});
for (const [name, title, subtitle, source = name] of [
  [
    'overview-with-history',
    'Your hardware, clearly.',
    'One interface. Light and dark. Powered by Rust.',
    'native-linux-overview',
  ],
  [
    'diagnostics',
    'Diagnostics, on your terms.',
    'Memory, storage, processor: choose what matters.',
    'native-linux-diagnostics',
  ],
  ['comparison', 'Changes, side by side.', 'Keep a baseline. Compare before and after.'],
  [
    'cleanup',
    'Make room. Stay in control.',
    'Preview first. Review every item. Confirm when ready.',
    'native-linux-clean-rules',
  ],
  [
    'applications',
    'Your applications, in one place.',
    'Find, launch and manage installed software.',
    'native-linux-applications',
  ],
]) {
  const light = (await readFile(`docs/screenshots/${source}-light.png`)).toString('base64');
  const dark = (await readFile(`docs/screenshots/${source}-dark.png`)).toString('base64');
  const logo = (await readFile('public/computy-logo.png')).toString('base64');
  const html = `<!doctype html><html lang="en"><meta charset="UTF-8"><style>*{box-sizing:border-box}body{margin:0;background:#eaf0e4;color:#24402e;font-family:Arial,sans-serif}.presentation{width:1680px;height:1200px;padding:60px 90px;position:relative;overflow:hidden}.header{display:flex;align-items:center;gap:15px}.logo{width:53px;height:53px}.wordmark{font-size:30px;letter-spacing:-1.6px;font-weight:700}.edition{margin-left:auto;font-size:11px;letter-spacing:2px;color:#8c9f76}.copy{margin:29px 0 31px;display:flex;align-items:flex-end;justify-content:space-between}.copy h1{font-size:35px;letter-spacing:-1.4px;margin:0 0 10px}.copy p{font-size:14px;color:#81956b;margin:0}.mode{font-size:10px;letter-spacing:1.4px;color:#78935c;display:flex;gap:11px;align-items:center}.mode i{height:20px;width:1px;background:#cbd8bb;transform:rotate(25deg)}.window{height:820px;width:1500px;border:1px solid #d4dfc8;border-radius:17px;overflow:hidden;position:relative;box-shadow:0 28px 65px #24402126;background:#f8faf5}.chrome{height:34px;position:relative;display:flex;align-items:center;padding:0 15px;gap:5px;background:#fdfefb;border-bottom:1px solid #91a57b22}.chrome:before{content:'';position:absolute;inset:0;background:#22331e;clip-path:polygon(61.945% 0,100% 0,100% 100%,61% 100%)}.chrome span{position:relative;height:7px;width:7px;background:#d4dfc6;border-radius:50%}.chrome small{position:absolute;left:50%;transform:translateX(-50%);font-size:9px;color:#93a97d}.screen{position:absolute;top:34px;bottom:0;left:0;right:0;overflow:hidden}.screen img{position:absolute;top:0;left:0;width:100%;height:auto;display:block}.dark{clip-path:polygon(61% 0,100% 0,100% 100%,39% 100%)}.seam{position:absolute;top:34px;left:50%;height:calc(100% - 34px);width:1px;background:#d9e9cbb3;transform:skewX(-22.6deg);transform-origin:center}.footer{display:flex;align-items:center;justify-content:space-between;margin-top:27px;font-size:10px;letter-spacing:1px;color:#91a47b}.footer b{color:#6b8750;font-weight:600;letter-spacing:1.7px}.tag{border:1px solid #ccdbbb;padding:7px 11px;border-radius:5px;font-size:9px;letter-spacing:1.4px}</style><main class="presentation"><div class="header"><img class="logo" src="data:image/png;base64,${logo}"><span class="wordmark">computy.</span><span class="edition">TAKE CARE OF YOUR COMPUTER.</span></div><div class="copy"><div><h1>${title}</h1><p>${subtitle}</p></div><div class="mode">LIGHT<i></i>DARK</div></div><div class="window"><div class="chrome"><span></span><span></span><span></span><small>COMPUTY · HARDWARE & MAINTENANCE</small></div><div class="screen"><img src="data:image/png;base64,${light}"></div><div class="screen dark"><img src="data:image/png;base64,${dark}"></div><div class="seam"></div></div><div class="footer"><span>RUST + TAURI <span style="margin:0 14px">/</span> LINUX · WINDOWS · MACOS</span><span class="tag">ACTUAL APPLICATION SCREENSHOTS</span><span>CREATED BY <b>NDNCI ↗</b></span></div></main></html>`;
  await page.setContent(html);
  await page
    .locator('img')
    .evaluateAll((images) => Promise.all(images.map((image) => image.decode())));
  await page.screenshot({ path: `docs/screenshots/presentation-${name}.png` });
}
await browser.close();
console.log('5 diagonal presentation images created from actual light/dark captures.');
