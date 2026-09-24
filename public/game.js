'use strict';
/* Star Drifter — node-map space roguelike (vanilla JS + canvas, zero deps).
 * Port of the Bevy design: node map, click-to-travel (1 fuel/jump),
 * events with choices + requirements, fuel/scrap/hull, 6 factions,
 * 10 sector types, procedural expansion. No LLM: static event deck
 * with danger-weighted outcomes.
 */

const TAU = Math.PI * 2;
const clamp = (v, a, b) => v < a ? a : v > b ? b : v;
function mulberry32(seed) {
  let a = seed >>> 0;
  return () => {
    a |= 0; a = (a + 0x6D2B79F5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

// ---------- data ----------
const FACTIONS = [
  { id: 'celestials', name: 'CELESTIALS', bonus: 'Start +20 hull. Stations repair more.', color: '#7dd3fc' },
  { id: 'cosmicons', name: 'COSMICONS', bonus: 'Start +15 fuel. Order provides.', color: '#a5b4fc' },
  { id: 'spirats', name: 'SPIRATS', bonus: 'Pirates take half tribute. Start +10 scrap.', color: '#f0abfc' },
  { id: 'webes', name: 'WEBES', bonus: 'Anomalies always pay out. Start +10 fuel.', color: '#67e8f9' },
  { id: 'archs', name: 'ARCHS', bonus: 'Devour: +scrap from empty sectors.', color: '#fda4af' },
  { id: 'spades', name: 'SPADES', bonus: 'Dark Rift pays double scrap.', color: '#c4b5fd' },
];
// colors mirror map.rs node palette
const SECTOR_STYLE = {
  Empty: { color: '#94a3b8', icon: 'empty' },
  Nebula: { color: '#818cf8', icon: 'nebula' },
  AsteroidField: { color: '#cbd5e1', icon: 'asteroid' },
  Station: { color: '#fbbf24', icon: 'station' },
  Distress: { color: '#fb923c', icon: 'empty' },
  Combat: { color: '#f87171', icon: 'combat' },
  Anomaly: { color: '#c084fc', icon: 'anomaly' },
  DarkRift: { color: '#e879f9', icon: 'darkrift' },
  CelestialSite: { color: '#7dd3fc', icon: 'celestial' },
  AetheriumField: { color: '#22d3ee', icon: 'aetherium' },
};
const PREFIX = {
  Empty: ['Void', 'Silent', 'Barren', 'Hollow'], Nebula: ['Crimson', 'Azure', 'Stellar', 'Mystic'],
  AsteroidField: ['Shattered', 'Broken', 'Drifting', 'Ancient'], Station: ['Haven', 'Refuge', 'Outpost', 'Trading'],
  Distress: ['Lost', 'Abandoned', 'Forgotten', 'Derelict'], Combat: ['Contested', 'Hostile', 'War-torn', 'Dangerous'],
  Anomaly: ['Strange', 'Twisted', 'Anomalous', 'Warped'], DarkRift: ['Dark', 'Void', 'Abyssal', 'Shadow'],
  CelestialSite: ['Sacred', 'Ancient', 'Divine', 'Eternal'], AetheriumField: ['Gleaming', 'Radiant', 'Precious', 'Crystalline'],
};
const SUFFIX = {
  Empty: ['Expanse', 'Reach', 'Void', 'Zone'], Nebula: ['Nebula', 'Cloud', 'Mist', 'Veil'],
  AsteroidField: ['Field', 'Belt', 'Cluster', 'Debris'], Station: ['Station', 'Port', 'Hub', 'Dock'],
  Distress: ['Wreck', 'Hulk', 'Grave', 'Ruin'], Combat: ['Battleground', 'Warzone', 'Sector', 'Front'],
  Anomaly: ['Anomaly', 'Phenomenon', 'Distortion', 'Rift'], DarkRift: ['Rift', 'Chasm', 'Abyss', 'Maw'],
  CelestialSite: ['Shrine', 'Temple', 'Sanctum', 'Monument'], AetheriumField: ['Mines', 'Crystals', 'Deposits', 'Veins'],
};
// Event deck: choice -> outcomes (good/bad, danger-weighted). Deltas: [fuel, scrap, hull].
const EVENTS = {
  merchant: {
    title: 'Traveling Merchant', desc: 'A merchant ship hails you, offering to trade supplies.',
    choices: [
      { t: 'Trade 10 scrap for fuel', req: { scrap: 10 }, out: [['Fuel cells secured. +6 fuel.', [6, -10, 0]]] },
      { t: 'Trade 4 fuel for scrap', req: { fuel: 4 }, out: [['Scrap converted. +14 scrap.', [-4, 14, 0]]] },
      { t: 'Decline and continue', req: {}, out: [['You drift on.', [0, 0, 0]]] },
    ],
  },
  anomaly: {
    title: 'Cosmic Anomaly', desc: 'Your sensors detect a strange energy signature ahead.',
    choices: [
      { t: 'Investigate the anomaly', req: {}, webes: true, out: [['Reality folds. You harvest charged particles. +4 fuel, +6 scrap.', [4, 6, 0]], ['Feedback surge! Systems fried. -15 hull.', [0, 0, -15]]] },
      { t: 'Scan from a safe distance', req: {}, out: [['Clean readings. Charts updated. +4 scrap.', [0, 4, 0]], ['Static. Nothing but noise.', [0, 0, 0]]] },
      { t: 'Ignore and continue', req: {}, out: [['You drift on.', [0, 0, 0]]] },
    ],
  },
  derelict: {
    title: 'Derelict Ship', desc: 'You discover the wreckage of an ancient vessel drifting in space.',
    choices: [
      { t: 'Board and explore', req: {}, out: [['Survivor cache! +12 scrap, +3 fuel.', [3, 12, 0]], ['Hull collapse! You barely escape. -12 hull.', [0, 0, -12]]] },
      { t: 'Salvage from outside', req: {}, out: [['Sheared plating recovered. +7 scrap.', [0, 7, 0]]] },
      { t: 'Leave it alone', req: {}, out: [['You drift on.', [0, 0, 0]]] },
    ],
  },
  pirates: {
    title: 'Spirat Raiders', desc: 'Spirat pirates emerge from an asteroid field, demanding tribute!',
    choices: [
      { t: 'Fight the pirates', req: {}, out: [['Pirates routed! Their hold is yours. +14 scrap.', [0, 14, -6]], ['They fight dirty. You escape bleeding. -22 hull.', [0, 0, -22]]] },
      { t: 'Pay tribute', req: { scrap: 8 }, tribute: true, out: [['They take the scrap and vanish.', [0, -8, 0]]] },
      { t: 'Burn 3 fuel to outrun them', req: { fuel: 3 }, out: [['Clean getaway. -3 fuel.', [-3, 0, 0]], ['They clip your engines. -3 fuel, -8 hull.', [-3, 0, -8]]] },
    ],
  },
  patrol: {
    title: 'Faction Patrol', desc: 'A patrol ship approaches your vessel.',
    choices: [
      { t: 'Hail them peacefully', req: {}, out: [['Protocols exchanged. They share charts. +3 fuel.', [3, 0, 0]], ['They scan you for contraband and fine you. -6 scrap.', [0, -6, 0]]] },
      { t: 'Prepare for combat', req: {}, out: [['Show of force works. They back off, dropping supplies. +8 scrap.', [0, 8, 0]], ['Skirmish! You win but scarred. -12 hull, +5 scrap.', [0, 5, -12]]] },
      { t: 'Burn 2 fuel to avoid them', req: { fuel: 2 }, out: [['Silent running. -2 fuel.', [-2, 0, 0]]] },
    ],
  },
  station: {
    title: 'Station Services', desc: 'The dockmaster offers repairs and trade.',
    choices: [
      { t: 'Repair hull (12 scrap)', req: { scrap: 12 }, repair: true, out: [['Hull welded and sealed.', [0, -12, 0]]] },
      { t: 'Buy fuel (8 scrap)', req: { scrap: 8 }, out: [['Tanks topped. +8 fuel.', [8, -8, 0]]] },
      { t: 'Sell charts (free)', req: {}, out: [['Your maps fetch a price. +8 scrap.', [0, 8, 0]]] },
    ],
  },
  distress: {
    title: 'Distress Beacon', desc: 'A damaged ship requests assistance.',
    choices: [
      { t: 'Answer the call', req: {}, out: [['Grateful crew pays in fuel. +6 fuel, +4 scrap.', [6, 4, 0]], ['A trap! Spirats spring the ambush. -16 hull.', [0, 0, -16]]] },
      { t: 'Cautious approach (2 fuel)', req: { fuel: 2 }, out: [['Real survivors. They reward caution. +5 scrap.', [-2, 5, 0]], ['Trap spotted in time. You burn away. -2 fuel.', [-2, 0, 0]]] },
      { t: 'Ignore and continue', req: {}, out: [['You drift on.', [0, 0, 0]]] },
    ],
  },
  mining: {
    title: 'Aetherium Field', desc: 'Rare Aetherium crystals float in the void. Mining is profitable but dangerous.',
    choices: [
      { t: 'Mine carefully', req: {}, out: [['Rich vein! +12 scrap.', [0, 12, 0]], ['Crystal shard storm. -10 hull, +6 scrap.', [0, 6, -10]]] },
      { t: 'Strip-mine (risky)', req: {}, out: [['Jackpot! +20 scrap.', [0, 20, -6]], ['Cave-in! -20 hull, +8 scrap.', [0, 8, -20]]] },
      { t: 'Leave it alone', req: {}, out: [['You drift on.', [0, 0, 0]]] },
    ],
  },
  celestial: {
    title: 'Celestial Ruins', desc: 'Ancient ruins pulse with residual power.',
    choices: [
      { t: 'Commune with the ruins', req: {}, out: [['The ancients approve. Hull knits itself. +20 hull.', [0, 0, 20]], ['Visions overwhelm the crew. Systems short. -8 hull, +6 scrap.', [0, 6, -8]]] },
      { t: 'Harvest residual energy', req: {}, out: [['Cells charged. +7 fuel.', [7, 0, 0]]] },
      { t: 'Leave it alone', req: {}, out: [['You drift on.', [0, 0, 0]]] },
    ],
  },
  combat: {
    title: 'Hostile Contact', desc: 'Hostile ships block your path!',
    choices: [
      { t: 'Fight through', req: {}, out: [['Enemy destroyed. Salvage secured. +12 scrap.', [0, 12, -8]], ['Outgunned! You limp away. -24 hull.', [0, 0, -24]]] },
      { t: 'Burn 4 fuel to evade', req: { fuel: 4 }, out: [['Lost them in the dust. -4 fuel.', [-4, 0, 0]], ['Parting shot hits home. -4 fuel, -10 hull.', [-4, 0, -10]]] },
    ],
  },
  darkrift: {
    title: 'Dark Rift Fragment', desc: 'A fragment of the Rift itself. Dangerous but potentially rewarding.',
    choices: [
      { t: 'Harvest the fragment', req: {}, spades: true, out: [['Impossible matter secured. +16 scrap.', [0, 16, -6]], ['The Rift pushes back. -22 hull.', [0, 0, -22]]] },
      { t: 'Skirt the edge', req: {}, out: [['Picked clean what drifted out. +7 scrap.', [0, 7, 0]]] },
      { t: 'Leave it alone', req: {}, out: [['You drift on.', [0, 0, 0]]] },
    ],
  },
  empty: {
    title: 'Silent Void', desc: 'Empty space. Nothing but dust and old light.',
    choices: [
      { t: 'Scoop dust (Archs feast)', req: {}, archs: true, out: [['Matter is matter. +6 scrap.', [0, 6, 0]]] },
      { t: 'Drift and rest', req: {}, out: [['Quiet repairs. +6 hull.', [0, 0, 6]], ['Nothing happens. The void watches.', [0, 0, 0]]] },
    ],
  },
};
const SECTOR_EVENT = {
  Station: 'station', Distress: 'distress', Combat: 'combat', Anomaly: 'anomaly',
  DarkRift: 'darkrift', CelestialSite: 'celestial', AetheriumField: 'mining',
  Empty: 'empty', Nebula: null, AsteroidField: null,
};
function randomEvent(rng, danger) {
  const r = rng() * 100;
  if (r < 28) return 'merchant';
  if (r < 46) return 'anomaly';
  if (r < 64) return 'derelict';
  if (r < 80) return danger >= 4 ? 'pirates' : 'patrol';
  return 'patrol';
}

// ---------- theme (single source: theme.css; canvas follows DOM) ----------
const TH = {};
function readTheme() {
  const cs = getComputedStyle(document.documentElement);
  for (const k of ['ink', 'muted', 'faint', 'line', 'accent', 'accent-soft', 'accent2', 'green', 'red', 'gold', 'surface', 'bg']) {
    TH[k] = cs.getPropertyValue('--' + k).trim();
  }
}
const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d');
const menuEl = document.getElementById('menu');
const overEl = document.getElementById('over');
const modalEl = document.getElementById('modal');
const hudEl = document.getElementById('hud');
let W = 0, H = 0;
function resize() {
  const dpr = Math.min(2, window.devicePixelRatio || 1);
  W = window.innerWidth; H = window.innerHeight;
  canvas.width = Math.floor(W * dpr); canvas.height = Math.floor(H * dpr);
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
}
window.addEventListener('resize', resize); resize();
readTheme();

// ---------- state ----------
const G = {
  screen: 'menu', faction: null,
  nodes: new Map(), current: 0, nextId: 1,
  fuel: 50, scrap: 15, hull: 100, maxHull: 100,
  jumps: 0, kills: 0,
  cam: { x: 0, y: 0, z: 1 },
  activeEvent: null, outcome: null,
  best: +(localStorage.getItem('sd-best') || 0),
  rng: mulberry32(1),
  stars: [],
};
for (let i = 0; i < 200; i++) G.stars.push({ x: Math.random(), y: Math.random(), s: Math.random() * 1.5 + 0.4 });

function sectorType(rng, distance) {
  const df = Math.min(5, distance / 10);
  const r = rng() * 100;
  if (r <= 25) return 'Empty';
  if (r <= 40) return 'Nebula';
  if (r <= 55) return 'AsteroidField';
  if (r <= 65) return 'Station';
  if (r <= 75) return 'Distress';
  if (r <= 85) return 'Combat';
  if (r <= 90) return df > 2 && rng() < 0.3 ? 'DarkRift' : 'Anomaly';
  if (r <= 98) return df > 1 && rng() < 0.4 ? 'CelestialSite' : 'Station';
  return df > 3 && rng() < 0.2 ? 'AetheriumField' : 'AsteroidField';
}
function sectorName(rng, type) {
  const p = PREFIX[type], s = SUFFIX[type];
  return p[Math.floor(rng() * p.length)] + ' ' + s[Math.floor(rng() * s.length)];
}
function addNode(type, x, y) {
  const id = G.nextId++;
  G.nodes.set(id, { id, type, name: sectorName(G.rng, type), x, y, visited: false, links: [] });
  return id;
}
function link(a, b) {
  const A = G.nodes.get(a), B = G.nodes.get(b);
  if (!A.links.includes(b)) A.links.push(b);
  if (!B.links.includes(a)) B.links.push(a);
}
// Port of Bevy generate_nodes_around: forward 120° cone away from source,
// reuse an existing node in range before creating, push-out on collision.
function hashId(id) {
  let h = (id * 2654435761) >>> 0;
  h ^= h >>> 15; h = (h * 0x85ebca6b) >>> 0; h ^= h >>> 13;
  return h >>> 0;
}
function expandAround(id, avoidId) {
  const src = G.nodes.get(id);
  const avoid = avoidId != null ? G.nodes.get(avoidId) : null;
  const MIN_SEP = 120, BASE_D = 300, RANGE = BASE_D * 1.8;
  let baseA;
  if (avoid && (src.x - avoid.x) ** 2 + (src.y - avoid.y) ** 2 > 0.0001) {
    baseA = Math.atan2(src.y - avoid.y, src.x - avoid.x);
  } else {
    baseA = (hashId(id) / 0xFFFFFFFF) * TAU; // deterministic fallback
  }
  const slots = 2 + Math.floor(G.rng() * 3);
  const ARC = (Math.PI * 2) / 3; // 120 degrees
  const step = slots > 1 ? ARC / (slots - 1) : 0;
  const used = new Set();
  for (let i = 0; i < slots; i++) {
    const angle = baseA - ARC / 2 + step * i + (G.rng() - 0.5) * 0.3;
    const dx = Math.cos(angle), dy = Math.sin(angle);
    // 1. connect to an existing node in range + direction instead of creating
    let best = null, bd = RANGE;
    for (const o of G.nodes.values()) {
      if (o.id === id || o.id === avoidId || src.links.includes(o.id) || used.has(o.id)) continue;
      const dc = Math.hypot(o.x - src.x, o.y - src.y);
      if (dc > RANGE || dc < MIN_SEP) continue;
      if (((o.x - src.x) * dx + (o.y - src.y) * dy) / (dc || 1) <= 0.5) continue; // ~60°
      const di = Math.hypot(o.x - (src.x + dx * BASE_D), o.y - (src.y + dy * BASE_D));
      if (di < bd) { bd = di; best = o; }
    }
    if (best) { used.add(best.id); link(id, best.id); continue; }
    // 2. new node, pushing the radius out until clear
    let radius = BASE_D, px = src.x + dx * radius, py = src.y + dy * radius;
    for (;;) {
      let close = false;
      for (const o of G.nodes.values()) {
        if (o.id === id) continue;
        if (Math.hypot(o.x - px, o.y - py) < MIN_SEP) { close = true; break; }
      }
      if (!close || radius > BASE_D * 3) break;
      radius += 50;
      px = src.x + dx * radius; py = src.y + dy * radius;
    }
    link(id, addNode(sectorType(G.rng, G.jumps), px, py));
  }
}

function startRun(faction) {
  G.faction = faction;
  G.rng = mulberry32((Math.random() * 0xFFFFFFFF) >>> 0);
  G.nodes.clear(); G.nextId = 1;
  G.fuel = 50 + (faction.id === 'cosmicons' ? 15 : faction.id === 'webes' ? 10 : 0);
  G.scrap = 15 + (faction.id === 'spirats' ? 10 : 0);
  G.maxHull = 100;
  G.hull = 100 + (faction.id === 'celestials' ? 20 : 0);
  G.jumps = 0; G.kills = 0;
  G.activeEvent = null; G.outcome = null;
  const start = addNode('Station', 0, 0);
  G.nodes.get(start).visited = true;
  G.current = start;
  const n0 = 3 + Math.floor(G.rng() * 3);
  for (let i = 0; i < n0; i++) {
    const a = (i / n0) * TAU + G.rng() * 0.5;
    link(start, addNode(sectorType(G.rng, 0), Math.cos(a) * 260, Math.sin(a) * 260));
  }
  G.cam = { x: 0, y: 0, z: Math.min(1, Math.min(W, H) / 700) };
  G.screen = 'play';
  menuEl.classList.add('hidden');
  overEl.classList.add('hidden');
  hudEl.classList.remove('hidden');
  hideModal();
  paintHUD();
  banner('DRIFT BEGUN <small>CLICK A CONNECTED NODE · 1 FUEL PER JUMP</small>');
}

function canPay(req) {
  if (req.fuel && G.fuel < req.fuel) return false;
  if (req.scrap && G.scrap < req.scrap) return false;
  return true;
}
function travelTo(id) {
  if (G.activeEvent || G.screen !== 'play') return;
  const cur = G.nodes.get(G.current);
  if (!cur.links.includes(id) || G.fuel < 1) {
    if (G.fuel < 1) gameOver('OUT OF FUEL — ADRIFT IN THE RIFT');
    return;
  }
  G.fuel -= 1; G.jumps++;
  const node = G.nodes.get(id);
  const first = !node.visited;
  node.visited = true;
  G.current = id;
  if (first) expandAround(id, cur.id);
  paintHUD();
  if (G.fuel <= 0) return gameOver('OUT OF FUEL — ADRIFT IN THE RIFT');
  openEventFor(node);
}

function openEventFor(node) {
  let key = SECTOR_EVENT[node.type];
  if (!key || G.rng() < 0.3) key = randomEvent(G.rng, dangerOf(node));
  const ev = JSON.parse(JSON.stringify(EVENTS[key]));
  ev.subtitle = `${node.name} · ${node.type.toUpperCase()}`;
  G.activeEvent = ev;
  G.outcome = null;
  showModal();
}

function dangerOf(node) {
  const base = { Empty: 0, Nebula: 1, AsteroidField: 2, Station: 0, Distress: 3, Combat: 5, Anomaly: 4, DarkRift: 8, CelestialSite: 6, AetheriumField: 7 }[node.type] || 0;
  return Math.min(9, base + Math.floor(G.jumps / 6));
}

function choose(i) {
  const ev = G.activeEvent;
  if (!ev || G.outcome) return;
  const c = ev.choices[i];
  if (!c || !canPay(c.req || {})) return;
  // faction shortcuts: fixed good outcome, no gamble
  let pick = null;
  if (c.webes && G.faction.id === 'webes') pick = 0;
  else if (c.tribute && G.faction.id === 'spirats') { applyDelta([0, -Math.ceil(8 / 2), 0]); return closeEvent(`${ev.title} — the Spirats respect their own. Half tribute accepted.`); }
  else if (c.repair) {
    const amt = G.faction.id === 'celestials' ? 45 : 30;
    G.scrap -= 12; G.hull = Math.min(G.maxHull, G.hull + amt);
    paintHUD(); return closeEvent(`${ev.title} — hull welded and sealed. +${amt} hull.`);
  } else if (c.archs && G.faction.id === 'archs') pick = 0;
  else if (c.spades && G.faction.id === 'spades') { applyDelta([0, 32, -6]); return closeEvent(`${ev.title} — the Rift feeds its own. +32 scrap.`); }
  if (pick === null) {
    const danger = dangerOf(G.nodes.get(G.current));
    const badOdds = clamp(0.25 + danger * 0.06 - (G.faction.id === 'celestials' ? 0.1 : 0), 0.1, 0.75);
    pick = (c.out.length > 1 && G.rng() < badOdds) ? 1 : 0;
  }
  const [text, delta] = c.out[pick];
  applyDelta(delta);
  paintHUD();
  closeEvent(`${ev.title} — ${text}`);
}

function applyDelta([f, s, h]) {
  G.fuel = Math.max(0, G.fuel + f);
  G.scrap = Math.max(0, G.scrap + Math.round(s));
  G.hull = clamp(G.hull + h, 0, G.maxHull);
  if (h < 0 && s > 0) G.kills += 0; // combat salvage counts as progress, not kills
}

function closeEvent(outcomeText) {
  G.activeEvent = null;
  G.outcome = null;
  hideModal();
  if (G.hull <= 0) return gameOver('HULL BREACHED — CLAIMED BY THE RIFT');
  if (G.fuel <= 0) {
    // stranded unless current node links somewhere free... fuel is per jump, so dead
    return gameOver('OUT OF FUEL — ADRIFT IN THE RIFT');
  }
  banner(outcomeText.split('—')[1]?.trim().toUpperCase().slice(0, 60) || 'EVENT RESOLVED');
}

function gameOver(reason) {
  G.screen = 'over';
  hideModal();
  const score = G.jumps * 10 + G.scrap + G.kills * 5;
  if (score > G.best) { G.best = score; localStorage.setItem('sd-best', String(score)); }
  document.getElementById('over-title').textContent = reason;
  document.getElementById('over-stats').textContent =
    `Jumps ${G.jumps} · Scrap ${G.scrap} · Best ${G.best} · ${G.faction.name}`;
  overEl.classList.remove('hidden');
}

// ---------- modal + hud ----------
function showModal() {
  const ev = G.activeEvent;
  document.getElementById('m-title').textContent = ev.title;
  document.getElementById('m-sub').textContent = ev.subtitle;
  document.getElementById('m-desc').textContent = ev.desc;
  const box = document.getElementById('m-choices');
  box.innerHTML = '';
  ev.choices.forEach((c, i) => {
    const b = document.createElement('button');
    const cost = [...(c.req?.scrap ? [`${c.req.scrap}⛁`] : []), ...(c.req?.fuel ? [`${c.req.fuel}⛽`] : [])].join(' ');
    b.innerHTML = `<b>${i + 1}. ${c.t}</b>${cost ? `<span>${cost}</span>` : ''}`;
    if (!canPay(c.req || {})) b.classList.add('locked');
    b.addEventListener('click', () => choose(i));
    box.appendChild(b);
  });
  modalEl.classList.remove('hidden');
}
function hideModal() { modalEl.classList.add('hidden'); }
function paintHUD() {
  hudEl.innerHTML =
    `<span class="hud-chip" style="color:${G.faction.color}">⬢ ${G.faction.name}</span>` +
    `<span class="hud-chip">⛽ <b>${Math.floor(G.fuel)}</b></span>` +
    `<span class="hud-chip">⛁ <b>${G.scrap}</b></span>` +
    `<span class="hud-chip">🛡 <b>${Math.ceil(G.hull)}</b></span>` +
    `<span class="spacer"></span>` +
    `<span class="hud-chip">J${G.jumps} · BEST ${G.best}</span>`;
}
function banner(html) {
  const b = document.getElementById('banner');
  b.innerHTML = html;
  b.classList.remove('hidden');
  b.style.animation = 'none'; void b.offsetWidth; b.style.animation = '';
  clearTimeout(banner._t);
  banner._t = setTimeout(() => b.classList.add('hidden'), 2600);
}

// ---------- input: pan/zoom/click + keys ----------
const drag = { on: false, sx: 0, sy: 0, cx: 0, cy: 0, moved: false };
function toWorld(px, py) {
  return { x: (px - W / 2) / G.cam.z + G.cam.x, y: (py - H / 2) / G.cam.z + G.cam.y };
}
canvas.addEventListener('pointerdown', e => {
  drag.on = true; drag.moved = false;
  drag.sx = e.clientX; drag.sy = e.clientY; drag.cx = G.cam.x; drag.cy = G.cam.y;
});
window.addEventListener('pointermove', e => {
  if (!drag.on) return;
  const dx = e.clientX - drag.sx, dy = e.clientY - drag.sy;
  if (Math.abs(dx) + Math.abs(dy) > 6) drag.moved = true;
  G.cam.x = drag.cx - dx / G.cam.z;
  G.cam.y = drag.cy - dy / G.cam.z;
});
window.addEventListener('pointerup', e => {
  if (!drag.on) return;
  drag.on = false;
  if (drag.moved || G.screen !== 'play' || G.activeEvent) return;
  const w = toWorld(e.clientX, e.clientY);
  let best = null, bd = (34 / G.cam.z) ** 2;
  for (const n of G.nodes.values()) {
    const d = (n.x - w.x) ** 2 + (n.y - w.y) ** 2;
    if (d < bd) { bd = d; best = n; }
  }
  if (best) travelTo(best.id);
});
canvas.addEventListener('wheel', e => {
  e.preventDefault();
  G.cam.z = clamp(G.cam.z * (e.deltaY < 0 ? 1.12 : 0.89), 0.3, 2.5);
}, { passive: false });
window.addEventListener('keydown', e => {
  if (G.activeEvent && ['Digit1', 'Digit2', 'Digit3', 'Digit4'].includes(e.code)) {
    choose(+e.code.slice(5) - 1);
  }
  if (e.code === 'Escape' && G.activeEvent) { /* events must resolve, no dismiss */ }
});

// hex color + alpha -> rgba (theme tokens stay the single source)
function hexA(hex, a) {
  const n = parseInt(hex.slice(1), 16);
  return `rgba(${(n >> 16) & 255},${(n >> 8) & 255},${n & 255},${a})`;
}
const iconImgs = {};
for (const k of new Set(Object.values(SECTOR_STYLE).map(s => s.icon))) {
  const img = new Image();
  img.src = 'assets/icons/' + k + '.svg';
  iconImgs[k] = img;
}

// ---------- render ----------
function draw() {
  ctx.clearRect(0, 0, W, H);
  ctx.fillStyle = TH.ink;
  for (const st of G.stars) {
    ctx.globalAlpha = 0.2 + st.s * 0.3;
    ctx.fillRect(st.x * W, st.y * H, st.s, st.s);
  }
  ctx.globalAlpha = 1;
  if (G.screen === 'menu' || G.faction === null && G.screen !== 'play') return;
  const { x: cx, y: cy, z } = G.cam;
  const X = wx => (wx - cx) * z + W / 2;
  const Y = wy => (wy - cy) * z + H / 2;
  // links
  for (const n of G.nodes.values()) {
    for (const l of n.links) {
      if (l < n.id) continue;
      const m = G.nodes.get(l);
      const isCur = n.id === G.current || m.id === G.current;
      const bothVis = n.visited && m.visited;
      ctx.strokeStyle = isCur ? hexA(TH.green, 0.85) : bothVis ? hexA(TH.faint, 0.4) : hexA(TH.muted, 0.35);
      ctx.lineWidth = isCur ? 2.5 : 1.2;
      ctx.beginPath(); ctx.moveTo(X(n.x), Y(n.y)); ctx.lineTo(X(m.x), Y(m.y)); ctx.stroke();
    }
  }
  // nodes
  const cur = G.nodes.get(G.current);
  for (const n of G.nodes.values()) {
    const st = SECTOR_STYLE[n.type];
    const x = X(n.x), y = Y(n.y);
    if (x < -60 || x > W + 60 || y < -60 || y > H + 60) continue;
    const r = (n.id === G.current ? 22 : 16) * Math.max(0.7, Math.min(1.3, z));
    const reachable = cur && cur.links.includes(n.id);
    // halo for reachable
    if (reachable && n.id !== G.current) {
      ctx.strokeStyle = hexA(TH.green, 0.5);
      ctx.lineWidth = 1.5;
      ctx.beginPath(); ctx.arc(x, y, r + 7 + 2 * Math.sin(performance.now() / 400), 0, TAU); ctx.stroke();
    }
    ctx.fillStyle = TH.surface;
    ctx.beginPath(); ctx.arc(x, y, r, 0, TAU); ctx.fill();
    ctx.strokeStyle = n.id === G.current ? TH.green : n.visited ? TH.faint : st.color;
    ctx.lineWidth = n.id === G.current ? 3 : 2;
    ctx.beginPath(); ctx.arc(x, y, r, 0, TAU); ctx.stroke();
    const img = iconImgs[st.icon];
    if (img && img.complete && img.naturalWidth) {
      ctx.drawImage(img, x - r * 0.55, y - r * 0.55, r * 1.1, r * 1.1);
    } else {
      ctx.fillStyle = st.color;
      ctx.beginPath(); ctx.arc(x, y, r * 0.35, 0, TAU); ctx.fill();
    }
    if (z > 0.55) {
      ctx.fillStyle = n.id === G.current ? TH.green : TH.muted;
      ctx.font = `${Math.max(10, 11 * z)}px 'Work Sans', system-ui, sans-serif`;
      ctx.textAlign = 'center';
      ctx.fillText(n.name, x, y + r + 14);
    }
  }
  // current pulse
  if (cur) {
    const p = (performance.now() / 900) % 1;
    ctx.strokeStyle = hexA(TH.green, 0.6 * (1 - p));
    ctx.lineWidth = 2;
    ctx.beginPath(); ctx.arc(X(cur.x), Y(cur.y), 24 + p * 22, 0, TAU); ctx.stroke();
  }
}

let last = 0;
function frame(ts) {
  requestAnimationFrame(frame);
  last = ts;
  draw();
}

// ---------- menu ----------
const fdiv = document.getElementById('factions');
for (const f of FACTIONS) {
  const b = document.createElement('button');
  b.innerHTML = `<b style="color:${f.color}">${f.name}</b><span>${f.bonus}</span>`;
  b.addEventListener('click', () => startRun(f));
  fdiv.appendChild(b);
}
document.getElementById('again').addEventListener('click', () => startRun(G.faction));
window.__sd = { G, travelTo, choose, startRun }; // test hook
requestAnimationFrame(frame);
