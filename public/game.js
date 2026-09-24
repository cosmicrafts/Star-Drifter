'use strict';
/* Star Drifter MVP — procedural canvas arcade. No build step, no deps.
 * Faithful to the legacy Bevy design: 6 factions, 10 sector types,
 * Aetherium, Hull, drift chains, the closing Dark Rift storm.
 */

// ---------- utils ----------
const TAU = Math.PI * 2;
const clamp = (v, a, b) => v < a ? a : v > b ? b : v;
const dist2 = (ax, ay, bx, by) => { const dx = ax - bx, dy = ay - by; return dx * dx + dy * dy; };
// ponytail: mulberry32, deterministic sectors from seed. Upgrade path: crypto seed per run.
function mulberry32(seed) {
  let a = seed >>> 0;
  return () => {
    a |= 0; a = (a + 0x6D2B79F5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

// ---------- data (ported from the Bevy design doc) ----------
const FACTIONS = [
  { id: 'celestials', name: 'CELESTIALS', bonus: 'Stations repair +50%, storm hurts less', color: '#7dd3fc' },
  { id: 'cosmicons', name: 'COSMICONS', bonus: '+40 max Hull, order holds', color: '#a5b4fc' },
  { id: 'spirats', name: 'SPIRATS', bonus: 'Drift chains build 2x faster', color: '#f0abfc' },
  { id: 'webes', name: 'WEBES', bonus: '+60% pickup range, cold logic', color: '#67e8f9' },
  { id: 'archs', name: 'ARCHS', bonus: 'Devour small asteroids for fuel', color: '#fda4af' },
  { id: 'spades', name: 'SPADES', bonus: '+50% score inside the storm', color: '#c4b5fd' },
];
// danger mirrors SectorType::base_danger from sector.rs
const SECTORS = [
  { type: 'Empty', danger: 0, rocks: 6, crystals: 6, anomalies: 0, nebula: 0, station: false },
  { type: 'Nebula', danger: 1, rocks: 8, crystals: 6, anomalies: 1, nebula: 4, station: false },
  { type: 'Asteroid Field', danger: 2, rocks: 16, crystals: 7, anomalies: 1, nebula: 0, station: false },
  { type: 'Station', danger: 0, rocks: 5, crystals: 5, anomalies: 0, nebula: 1, station: true },
  { type: 'Distress', danger: 3, rocks: 12, crystals: 8, anomalies: 1, nebula: 1, station: false },
  { type: 'Anomaly', danger: 4, rocks: 10, crystals: 7, anomalies: 3, nebula: 2, station: false },
  { type: 'Combat', danger: 5, rocks: 14, crystals: 8, anomalies: 1, nebula: 0, station: false, mines: 4 },
  { type: 'Celestial Site', danger: 6, rocks: 12, crystals: 9, anomalies: 2, nebula: 2, station: false },
  { type: 'Aetherium Field', danger: 7, rocks: 14, crystals: 12, anomalies: 2, nebula: 1, station: false },
  { type: 'Dark Rift', danger: 8, rocks: 18, crystals: 10, anomalies: 2, nebula: 3, station: false, mines: 6 },
];
const NAME_A = ['Vel', 'Kor', 'Thal', 'Zer', 'Mir', 'Xan', 'Ostr', 'Bel', 'Dra', 'Nyx', 'Sol', 'Rho'];
const NAME_B = ['aris', ' Prime', ' Drift', ' Hollow', ' Verge', ' Maw', ' Shallows', ' Deep', ' Gate', ' Expanse'];

// ---------- state ----------
const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
let W = 0, H = 0, DPR = 1;
function resize() {
  DPR = Math.min(2, window.devicePixelRatio || 1);
  W = window.innerWidth; H = window.innerHeight;
  canvas.width = Math.floor(W * DPR); canvas.height = Math.floor(H * DPR);
  ctx.setTransform(DPR, 0, 0, DPR, 0, 0);
}
window.addEventListener('resize', resize); resize();

const G = {
  screen: 'menu', // menu | play | over
  faction: null,
  ship: null, rocks: [], crystals: [], mines: [], anomalies: [],
  clouds: [], station: null, gate: null,
  sectorIdx: 0, sector: null, seed: 1,
  score: 0, chain: 0, chainT: 0,
  stormR: 0, stormMax: 0,
  quota: 0, bannerT: 0,
  best: +(localStorage.getItem('sd-best') || 0),
  stars: [],
};
for (let i = 0; i < 160; i++) G.stars.push({ x: Math.random(), y: Math.random(), s: Math.random() * 1.6 + 0.4 });

// ---------- input ----------
const keys = {};
window.addEventListener('keydown', e => {
  keys[e.code] = true;
  if (['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'Space'].includes(e.code)) e.preventDefault();
});
window.addEventListener('keyup', e => { keys[e.code] = false; });
const touch = { active: false, x: 0, y: 0 };
canvas.addEventListener('pointerdown', e => { touch.active = true; touch.x = e.clientX; touch.y = e.clientY; });
window.addEventListener('pointermove', e => { if (touch.active) { touch.x = e.clientX; touch.y = e.clientY; } });
window.addEventListener('pointerup', () => { touch.active = false; });

// ---------- generation ----------
function genSector(idx) {
  const rng = mulberry32((G.seed ^ (idx * 2654435761)) >>> 0);
  const base = SECTORS[Math.floor(rng() * SECTORS.length)];
  const depth = Math.floor(idx / 2);
  const R = Math.max(W, H);
  const cx = W / 2, cy = H / 2;
  const put = (away = 120) => {
    for (let t = 0; t < 40; t++) {
      const x = rng() * W, y = rng() * H;
      if (dist2(x, y, cx, cy) > away * away) return { x, y };
    }
    return { x: rng() * W, y: rng() * H };
  };
  const n = (v, per) => Math.round(v + depth * per);
  G.sector = base;
  G.rocks = []; G.crystals = []; G.mines = []; G.anomalies = []; G.clouds = [];
  for (let i = 0; i < n(base.rocks, 1.2); i++) {
    const p = put();
    G.rocks.push({ x: p.x, y: p.y, vx: (rng() - 0.5) * 40, vy: (rng() - 0.5) * 40, r: 12 + rng() * 26, rot: rng() * TAU, vr: (rng() - 0.5) * 1.2, small: false });
  }
  for (let i = 0; i < n(base.crystals, 0.4); i++) {
    const p = put(90);
    G.crystals.push({ x: p.x, y: p.y, r: 9, ph: rng() * TAU });
  }
  for (let i = 0; i < (base.mines || 0) + Math.floor(depth / 3); i++) {
    const p = put(160);
    G.mines.push({ x: p.x, y: p.y, r: 11, ph: rng() * TAU });
  }
  for (let i = 0; i < base.anomalies; i++) {
    const p = put(140);
    G.anomalies.push({ x: p.x, y: p.y, r: 16, ph: rng() * TAU });
  }
  for (let i = 0; i < base.nebula; i++) G.clouds.push({ x: rng() * W, y: rng() * H, r: 90 + rng() * 130 });
  G.station = base.station ? { ...put(200), r: 26, used: false } : null;
  G.gate = null;
  G.quota = Math.min(9, 5 + Math.floor(idx / 2));
  G.stormMax = R * 0.75;
  G.stormR = G.stormMax;
  G.sectorName = NAME_A[Math.floor(rng() * NAME_A.length)] + NAME_B[Math.floor(rng() * NAME_B.length)];
  G.collected = 0;
  banner(`${G.sectorName} <small>${base.type.toUpperCase()} · DANGER ${Math.min(9, base.danger + depth)} · COLLECT ${G.quota} AETHERIUM</small>`);
}

function startRun(faction) {
  G.faction = faction;
  G.seed = (Math.random() * 0xFFFFFFFF) >>> 0;
  G.sectorIdx = 0; G.score = 0; G.chain = 0; G.chainT = 0;
  const hull = faction.id === 'cosmicons' ? 140 : 100;
  G.ship = { x: W / 2, y: H / 2, vx: 0, vy: 0, a: -Math.PI / 2, hull, maxHull: hull, fuel: 100, invuln: 0, thrusting: false, turning: 0 };
  genSector(0);
  G.screen = 'play';
  document.getElementById('menu').classList.add('hidden');
  document.getElementById('over').classList.add('hidden');
}

function gameOver(reason) {
  G.screen = 'over';
  if (G.score > G.best) { G.best = G.score; localStorage.setItem('sd-best', String(G.best)); }
  document.getElementById('over-title').textContent = reason === 'fuel' ? 'ADRIFT IN THE RIFT' : 'HULL BREACHED';
  document.getElementById('over-stats').textContent =
    `Score ${G.score} · Best ${G.best} · Sector ${G.sectorIdx + 1} (${G.sectorName}) · ${G.faction.name}`;
  const ov = document.getElementById('over');
  ov.classList.remove('hidden');
}

// ---------- update ----------
function mult() { return 1 + Math.min(4, G.chain * 0.25); }
function addScore(p, x, y) {
  let v = Math.round(p * mult());
  if (G.faction.id === 'spades' && outsideStorm(x, y)) v = Math.round(v * 1.5);
  G.score += v;
  if (v >= 20) floaters.push({ x, y, t: 0, txt: '+' + v });
}
const floaters = [];

function outsideStorm(x, y) {
  return dist2(x, y, W / 2, H / 2) > G.stormR * G.stormR;
}

function update(dt) {
  const s = G.ship;
  if (!s) return;
  // steering
  let rot = 0;
  if (keys.ArrowLeft || keys.KeyA) rot -= 1;
  if (keys.ArrowRight || keys.KeyD) rot += 1;
  let thrust = !!(keys.ArrowUp || keys.KeyW || keys.Space);
  if (touch.active) {
    const dx = touch.x - s.x, dy = touch.y - s.y;
    if (Math.hypot(dx, dy) > 24) {
      const want = Math.atan2(dy, dx);
      let d = want - s.a;
      while (d > Math.PI) d -= TAU; while (d < -Math.PI) d += TAU;
      rot = clamp(d * 2.2, -1, 1);
      thrust = Math.abs(d) < 1.2;
    }
  }
  s.a += rot * 3.4 * dt;
  s.turning = rot;
  // nebula drag
  let drag = 0.55, inNebula = false;
  for (const c of G.clouds) {
    if (dist2(s.x, s.y, c.x, c.y) < c.r * c.r) { drag = 1.6; inNebula = true; break; }
  }
  if (thrust && s.fuel > 0) {
    s.vx += Math.cos(s.a) * 300 * dt;
    s.vy += Math.sin(s.a) * 300 * dt;
    s.fuel = Math.max(0, s.fuel - 3.2 * dt);
    s.thrusting = true;
  } else s.thrusting = false;
  const damp = Math.exp(-drag * dt);
  s.vx *= damp; s.vy *= damp;
  const sp = Math.hypot(s.vx, s.vy), maxSp = 430;
  if (sp > maxSp) { s.vx *= maxSp / sp; s.vy *= maxSp / sp; }
  s.x = (s.x + s.vx * dt + W) % W;
  s.y = (s.y + s.vy * dt + H) % H;
  if (s.invuln > 0) s.invuln -= dt;
  // drift chain: sliding sideways while turning
  const fx = Math.cos(s.a), fy = Math.sin(s.a);
  const lat = Math.abs(s.vx * -fy + s.vy * fx);
  const chainRate = G.faction.id === 'spirats' ? 2 : 1;
  if (lat > 110 && Math.abs(rot) > 0.2 && sp > 150) {
    G.chain += chainRate * dt * 2;
    G.chainT = G.faction.id === 'spirats' ? 1.8 : 1.2;
  } else if (G.chainT > 0) {
    G.chainT -= dt;
    if (G.chainT <= 0) G.chain = 0;
  } else G.chain = Math.max(0, G.chain - dt * 3);
  // rocks drift + collide
  const pickupR = G.faction.id === 'webes' ? 64 : 40;
  for (let i = G.rocks.length - 1; i >= 0; i--) {
    const r = G.rocks[i];
    r.x = (r.x + r.vx * dt + W) % W; r.y = (r.y + r.vy * dt + H) % H; r.rot += r.vr * dt;
    const rr = r.r + 10;
    if (dist2(s.x, s.y, r.x, r.y) < rr * rr) {
      if (G.faction.id === 'archs' && r.r < 22) {
        // Archs devour small rocks
        G.rocks.splice(i, 1);
        s.fuel = Math.min(100, s.fuel + 8);
        addScore(15, r.x, r.y);
        continue;
      }
      if (s.invuln <= 0) {
        s.hull -= 12; s.invuln = 1.1; G.chain = 0; G.chainT = 0;
        const d = Math.max(1, Math.hypot(s.x - r.x, s.y - r.y));
        s.vx += (s.x - r.x) / d * 260; s.vy += (s.y - r.y) / d * 260;
        shake(7);
        if (s.hull <= 0) return gameOver('hull');
      }
    }
  }
  // crystals
  for (let i = G.crystals.length - 1; i >= 0; i--) {
    const c = G.crystals[i];
    c.ph += dt * 3;
    if (dist2(s.x, s.y, c.x, c.y) < pickupR * pickupR) {
      G.crystals.splice(i, 1);
      G.collected++;
      s.fuel = Math.min(100, s.fuel + 4);
      addScore(25, c.x, c.y);
      if (G.collected >= G.quota && !G.gate) {
        const a = Math.random() * TAU;
        G.gate = { x: W / 2 + Math.cos(a) * G.stormR * 0.55, y: H / 2 + Math.sin(a) * G.stormR * 0.55, r: 30, ph: 0 };
        banner('WARP GATE OPEN <small>FLY INTO THE RING</small>');
      }
    }
  }
  // mines (Combat/DarkRift sectors)
  for (let i = G.mines.length - 1; i >= 0; i--) {
    const m = G.mines[i];
    m.ph += dt * 4;
    if (dist2(s.x, s.y, m.x, m.y) < 26 * 26 && s.invuln <= 0) {
      G.mines.splice(i, 1);
      s.hull -= 20; s.invuln = 1.1; G.chain = 0; G.chainT = 0;
      shake(10);
      if (s.hull <= 0) return gameOver('hull');
    }
  }
  // anomalies teleport
  for (const a of G.anomalies) {
    a.ph += dt * 2;
    if (dist2(s.x, s.y, a.x, a.y) < 26 * 26 && s.invuln <= 0) {
      s.x = Math.random() * W; s.y = Math.random() * H;
      s.vx *= 0.3; s.vy *= 0.3; s.invuln = 1.5;
      addScore(100, s.x, s.y);
      banner('ANOMALY JUMP <small>+BONUS</small>');
    }
  }
  // station repair
  if (G.station && !G.station.used && dist2(s.x, s.y, G.station.x, G.station.y) < 40 * 40) {
    G.station.used = true;
    const heal = G.faction.id === 'celestials' ? 45 : 30;
    s.hull = Math.min(s.maxHull, s.hull + heal);
    addScore(50, G.station.x, G.station.y);
    banner('HULL RESTORED <small>+' + heal + '</small>');
  }
  // warp gate
  if (G.gate) {
    G.gate.ph += dt * 3;
    if (dist2(s.x, s.y, G.gate.x, G.gate.y) < 34 * 34) {
      G.sectorIdx++;
      addScore(150, s.x, s.y);
      s.fuel = Math.min(100, s.fuel + 25);
      genSector(G.sectorIdx);
      s.x = W / 2; s.y = H / 2; s.vx = 0; s.vy = 0;
      return;
    }
  }
  // storm closes in
  const stormSpeed = 9 + G.sector.danger * 1.6 + G.sectorIdx * 0.7;
  G.stormR = Math.max(120, G.stormR - stormSpeed * dt);
  if (outsideStorm(s.x, s.y)) {
    const dps = G.faction.id === 'celestials' ? 7 : 10;
    s.hull -= dps * dt;
    if (Math.random() < dt * 6) floaters.push({ x: s.x, y: s.y - 20, t: 0, txt: 'RIFT!' });
    if (s.hull <= 0) return gameOver('hull');
  }
  if (s.fuel <= 0 && sp < 20) return gameOver('fuel');
  for (let i = floaters.length - 1; i >= 0; i--) {
    floaters[i].t += dt;
    if (floaters[i].t > 1) floaters.splice(i, 1);
  }
}

let shakeT = 0, shakeM = 0;
function shake(m) { shakeM = m; shakeT = 0.35; }

// ---------- render ----------
function banner(html) {
  const b = document.getElementById('banner');
  b.innerHTML = html;
  b.classList.remove('hidden');
  void b.offsetWidth;
  b.style.animation = 'none'; void b.offsetWidth; b.style.animation = '';
  clearTimeout(banner._t);
  banner._t = setTimeout(() => b.classList.add('hidden'), 2600);
}

function draw(dt) {
  ctx.clearRect(0, 0, W, H);
  let sx = 0, sy = 0;
  if (shakeT > 0) { shakeT -= dt; sx = (Math.random() - 0.5) * shakeM; sy = (Math.random() - 0.5) * shakeM; }
  ctx.save(); ctx.translate(sx, sy);
  // stars
  ctx.fillStyle = '#fff';
  for (const st of G.stars) {
    ctx.globalAlpha = 0.25 + st.s * 0.3;
    ctx.fillRect(st.x * W, st.y * H, st.s, st.s);
  }
  ctx.globalAlpha = 1;
  if (G.screen !== 'play' || !G.ship) { ctx.restore(); return; }
  const t = performance.now() / 1000;
  // nebula clouds
  for (const c of G.clouds) {
    const g = ctx.createRadialGradient(c.x, c.y, 0, c.x, c.y, c.r);
    g.addColorStop(0, 'rgba(99,102,241,0.20)');
    g.addColorStop(1, 'rgba(99,102,241,0)');
    ctx.fillStyle = g;
    ctx.beginPath(); ctx.arc(c.x, c.y, c.r, 0, TAU); ctx.fill();
  }
  const s = G.ship;
  // storm: dark outside, violet rim
  ctx.save();
  ctx.beginPath(); ctx.rect(0, 0, W, H);
  ctx.arc(W / 2, H / 2, G.stormR, 0, TAU, true);
  ctx.fillStyle = 'rgba(76,29,149,0.28)';
  ctx.fill('evenodd');
  ctx.strokeStyle = `rgba(192,132,252,${0.5 + 0.3 * Math.sin(t * 4)})`;
  ctx.lineWidth = 3;
  ctx.beginPath(); ctx.arc(W / 2, H / 2, G.stormR, 0, TAU); ctx.stroke();
  ctx.restore();
  // station
  if (G.station) {
    ctx.strokeStyle = G.station.used ? '#475569' : '#34d399';
    ctx.lineWidth = 3;
    ctx.strokeRect(G.station.x - 18, G.station.y - 18, 36, 36);
    ctx.fillStyle = G.station.used ? '#475569' : '#34d399';
    ctx.font = '11px monospace'; ctx.textAlign = 'center';
    ctx.fillText(G.station.used ? 'SPENT' : '+HULL', G.station.x, G.station.y + 34);
  }
  // warp gate
  if (G.gate) {
    ctx.strokeStyle = `rgba(34,211,238,${0.6 + 0.4 * Math.sin(G.gate.ph)})`;
    ctx.lineWidth = 4;
    ctx.beginPath(); ctx.arc(G.gate.x, G.gate.y, G.gate.r, 0, TAU); ctx.stroke();
    ctx.beginPath(); ctx.arc(G.gate.x, G.gate.y, G.gate.r * 0.6, 0, TAU); ctx.stroke();
  }
  // anomalies
  for (const a of G.anomalies) {
    ctx.strokeStyle = `rgba(240,171,252,${0.5 + 0.4 * Math.sin(a.ph)})`;
    ctx.lineWidth = 2;
    for (let k = 0; k < 3; k++) {
      ctx.beginPath(); ctx.arc(a.x, a.y, 6 + k * 6 + Math.sin(a.ph + k) * 2, 0, TAU); ctx.stroke();
    }
  }
  // crystals
  for (const c of G.crystals) {
    const p = 1 + 0.2 * Math.sin(c.ph);
    ctx.save(); ctx.translate(c.x, c.y); ctx.rotate(c.ph * 0.4); ctx.scale(p, p);
    ctx.fillStyle = '#22d3ee';
    ctx.beginPath();
    ctx.moveTo(0, -9); ctx.lineTo(6, 0); ctx.lineTo(0, 9); ctx.lineTo(-6, 0);
    ctx.closePath(); ctx.fill();
    ctx.restore();
  }
  // mines
  for (const m of G.mines) {
    ctx.fillStyle = `rgba(248,113,113,${0.7 + 0.3 * Math.sin(m.ph)})`;
    ctx.beginPath(); ctx.arc(m.x, m.y, 7, 0, TAU); ctx.fill();
    ctx.strokeStyle = '#f87171'; ctx.lineWidth = 1.5;
    for (let k = 0; k < 8; k++) {
      const a = k / 8 * TAU + m.ph * 0.2;
      ctx.beginPath();
      ctx.moveTo(m.x + Math.cos(a) * 7, m.y + Math.sin(a) * 7);
      ctx.lineTo(m.x + Math.cos(a) * 12, m.y + Math.sin(a) * 12);
      ctx.stroke();
    }
  }
  // rocks
  ctx.strokeStyle = '#94a3b8'; ctx.lineWidth = 2;
  for (const r of G.rocks) {
    ctx.save(); ctx.translate(r.x, r.y); ctx.rotate(r.rot);
    ctx.beginPath();
    const n = 8;
    for (let k = 0; k <= n; k++) {
      const a = k / n * TAU;
      const rr = r.r * (0.78 + 0.22 * Math.sin(k * 2.7 + r.r));
      const px = Math.cos(a) * rr, py = Math.sin(a) * rr;
      if (k === 0) ctx.moveTo(px, py); else ctx.lineTo(px, py);
    }
    ctx.stroke(); ctx.restore();
  }
  // ship (blink while invulnerable)
  if (s.invuln <= 0 || Math.floor(t * 12) % 2 === 0) {
    ctx.save(); ctx.translate(s.x, s.y); ctx.rotate(s.a);
    ctx.fillStyle = G.faction.color;
    ctx.beginPath();
    ctx.moveTo(14, 0); ctx.lineTo(-10, -9); ctx.lineTo(-5, 0); ctx.lineTo(-10, 9);
    ctx.closePath(); ctx.fill();
    if (s.thrusting) {
      ctx.fillStyle = '#fb923c';
      ctx.beginPath();
      ctx.moveTo(-6, -4); ctx.lineTo(-6 - 10 - Math.random() * 8, 0); ctx.lineTo(-6, 4);
      ctx.closePath(); ctx.fill();
    }
    ctx.restore();
  }
  // floaters
  ctx.textAlign = 'center'; ctx.font = 'bold 13px monospace';
  for (const f of floaters) {
    ctx.globalAlpha = 1 - f.t;
    ctx.fillStyle = '#fde68a';
    ctx.fillText(f.txt, f.x, f.y - f.t * 30);
  }
  ctx.globalAlpha = 1;
  ctx.restore();
  drawHUD();
}

function bar(x, y, w, frac, color) {
  ctx.fillStyle = 'rgba(255,255,255,0.12)';
  ctx.fillRect(x, y, w, 8);
  ctx.fillStyle = color;
  ctx.fillRect(x, y, w * clamp(frac, 0, 1), 8);
}

function drawHUD() {
  const s = G.ship;
  ctx.textAlign = 'left'; ctx.font = '12px monospace';
  ctx.fillStyle = '#94a3b8';
  ctx.fillText(`HULL`, 12, 20);
  bar(58, 12, 120, s.hull / s.maxHull, s.hull > 30 ? '#34d399' : '#f87171');
  ctx.fillText(`FUEL`, 12, 38);
  bar(58, 30, 120, s.fuel / 100, '#60a5fa');
  ctx.fillStyle = '#22d3ee';
  ctx.fillText(`⬢ ${G.collected}/${G.quota}`, 12, 58);
  ctx.fillStyle = '#fde68a';
  ctx.fillText(`SCORE ${G.score}`, 12, 78);
  if (G.chain > 0.5) {
    ctx.fillStyle = '#f0abfc';
    ctx.fillText(`DRIFT x${mult().toFixed(1)}`, 12, 98);
  }
  ctx.textAlign = 'right';
  ctx.fillStyle = '#94a3b8';
  ctx.fillText(`${G.sectorName} · #${G.sectorIdx + 1}`, W - 12, 20);
  ctx.fillStyle = '#64748b';
  ctx.fillText(`BEST ${G.best}`, W - 12, 38);
  if (outsideStorm(s.x, s.y)) {
    ctx.textAlign = 'center';
    ctx.fillStyle = `rgba(248,113,113,${0.6 + 0.4 * Math.sin(performance.now() / 150)})`;
    ctx.font = 'bold 15px monospace';
    ctx.fillText('⚠ DARK RIFT — GET INSIDE ⚠', W / 2, 52);
  }
}

// ---------- loop ----------
let last = 0;
function frame(ts) {
  requestAnimationFrame(frame);
  const dt = Math.min(0.05, (ts - last) / 1000 || 0.016);
  last = ts;
  if (G.screen === 'play') update(dt);
  draw(dt);
}

// ---------- menu wiring ----------
const fdiv = document.getElementById('factions');
for (const f of FACTIONS) {
  const b = document.createElement('button');
  b.innerHTML = `<b style="color:${f.color}">${f.name}</b><span>${f.bonus}</span>`;
  b.addEventListener('click', () => startRun(f));
  fdiv.appendChild(b);
}
document.getElementById('again').addEventListener('click', () => startRun(G.faction));
window.__sd = { G, startRun }; // test hook
requestAnimationFrame(frame);
