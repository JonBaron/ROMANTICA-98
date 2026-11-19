// Nettleserverten for invit.wasm: canvas i stedet for VGA, tastatur i stedet for
// port 60h, og chiptune3 (libopenmpt) i stedet for USM Play.

const CHIPTUNE = 'https://cdn.jsdelivr.net/npm/chiptune3@0.8.10/chiptune3.js';
const W = 320;
const H = 200;
const RETRACE_MS = 1000 / 70;

const SCAN_ESC = 0x01;
const SCAN_LEFT = 0x4b;
const SCAN_RIGHT = 0x4d;

const $ = (id) => document.getElementById(id);
const canvas = $('screen');
const ctx = canvas.getContext('2d');

const wasm = await WebAssembly.instantiate(await (await fetch('invit.wasm')).arrayBuffer(), {});
const demo = wasm.instance.exports;
demo.demo_init();

// Port 60h holder siste scankode, og ESC må sees i minst tre retracer for å
// avslutte, så et kort trykk varer like lenge som et typisk fysisk tastetrykk.
const MIN_PRESS_TICKS = 6;
const held = new Set();
const tapped = new Map();
const keyNames = { Escape: 'esc', ArrowLeft: 'left', ArrowRight: 'right' };

function press(key) {
  held.add(key);
  tapped.set(key, MIN_PRESS_TICKS);
}

addEventListener('keydown', (e) => {
  if (keyNames[e.key]) {
    press(keyNames[e.key]);
    e.preventDefault();
  }
});
addEventListener('keyup', (e) => held.delete(keyNames[e.key]));
addEventListener('blur', () => held.clear());

for (const button of document.querySelectorAll('#pad button')) {
  const key = button.dataset.key;
  button.addEventListener('pointerdown', () => press(key));
  for (const ev of ['pointerup', 'pointerleave', 'pointercancel']) {
    button.addEventListener(ev, () => held.delete(key));
  }
}

function scancode() {
  const down = (key) => held.has(key) || tapped.has(key);
  const code = down('esc') ? SCAN_ESC : down('left') ? SCAN_LEFT : down('right') ? SCAN_RIGHT : 0;
  for (const [key, ticks] of tapped) {
    if (ticks > 1) tapped.set(key, ticks - 1);
    else tapped.delete(key);
  }
  return code;
}

function draw() {
  const pixels = new Uint8ClampedArray(demo.memory.buffer, demo.demo_frame(), W * H * 4);
  ctx.putImageData(new ImageData(pixels, W, H), 0, 0);
}

let player = null;

function run() {
  $('setup').style.display = 'none';
  canvas.style.display = 'block';
  $('pad').style.display = 'flex';

  let last = performance.now();
  let pending = 0;
  let running = true;

  function frame(now) {
    pending = Math.min(pending + now - last, 100);
    last = now;
    while (running && pending >= RETRACE_MS) {
      running = demo.demo_tick(scancode()) !== 0;
      pending -= RETRACE_MS;
    }
    draw();
    if (running) requestAnimationFrame(frame);
    else end();
  }
  requestAnimationFrame(frame);
}

function end() {
  player?.stop();
  canvas.style.display = 'none';
  $('pad').style.display = 'none';
  const out = $('end');
  out.style.display = 'block';
  out.innerHTML =
    'C:\\ROMINV&gt;invit\n\n' +
    'det var egentlig alt det... bare du scrolla!\n\n' +
    ' a TP-multimedia produksjon tm\n\n' +
    'C:\\ROMINV&gt;<a href="">invit</a>_';
}

async function startWithSound() {
  $('sound').disabled = $('nosound').disabled = true;
  const audio = new AudioContext();
  try {
    const { ChiptuneJsPlayer } = await import(CHIPTUNE);
    const music = await (await fetch('HIGH_HPS.XM')).arrayBuffer();
    player = new ChiptuneJsPlayer({ repeatCount: -1, context: audio });
    player.gain.connect(audio.destination);
    await new Promise((resolve, reject) => {
      player.onInitialized(resolve);
      player.onError(reject);
      setTimeout(() => reject(new Error('timeout')), 8000);
    });
    player.play(music);
    document.body.dataset.music = 'on';
  } catch (e) {
    console.warn('Ingen musikk:', e);
    document.body.dataset.music = 'off';
    player = null;
  }
  run();
}

$('sound').addEventListener('click', startWithSound);
$('nosound').addEventListener('click', run);
