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
      { t: 'Fight the pirates', req: {}, battle: 'pirates', out: [['', [0, 0, 0]]] },
      { t: 'Pay tribute', req: { scrap: 8 }, tribute: true, out: [['They take the scrap and vanish.', [0, -8, 0]]] },
      { t: 'Burn 3 fuel to outrun them', req: { fuel: 3 }, out: [['Clean getaway. -3 fuel.', [-3, 0, 0]], ['They clip your engines. -3 fuel, -8 hull.', [-3, 0, -8]]] },
    ],
  },
  patrol: {
    title: 'Faction Patrol', desc: 'A patrol ship approaches your vessel.',
    choices: [
      { t: 'Hail them peacefully', req: {}, out: [['Protocols exchanged. They share charts. +3 fuel.', [3, 0, 0]], ['They scan you for contraband and fine you. -6 scrap.', [0, -6, 0]]] },
      { t: 'Prepare for combat', req: {}, battle: 'patrol', out: [['', [0, 0, 0]]] },
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
      { t: 'Answer the call', req: {}, out: [['Grateful crew pays in fuel. +6 fuel, +4 scrap.', [6, 4, 0]], ['A trap! Spirats spring the ambush.', [0, 0, 0], { battle: 'trap' }]] },
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
      { t: 'Fight through', req: {}, battle: 'combat', out: [['', [0, 0, 0]]] },
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

// ---------- FTL battle MVP: ships, auto-crew, stances, negotiation ----------
const WEAPONS = {
  laser: { name: 'Laser', dmg: 1, sys: 1, cd: 4, color: '#7dd3fc' },
  missile: { name: 'Missile', dmg: 3, sys: 2, cd: 7, ammo: 3, color: '#fb923c' },
  ion: { name: 'Ion', dmg: 0, sys: 3, cd: 5, color: '#c4b5fd' },
};
const FOES = {
  scout: { name: 'Spirat Scout', hull: 12, shield: 1, weapons: ['laser'], loot: [2, 8, 4], crew: 1 },
  raider: { name: 'Spirat Raider', hull: 18, shield: 1, weapons: ['laser', 'missile'], loot: [3, 12, 6], crew: 2 },
  frigate: { name: 'Order Frigate', hull: 22, shield: 2, weapons: ['laser', 'laser'], loot: [3, 10, 8], crew: 2 },
  drone: { name: 'Rogue Drone', hull: 9, shield: 0, weapons: ['ion', 'laser'], loot: [1, 6, 3], crew: 0 },
};
const CREW_NAMES = ['Rook', 'Vex', 'Sable', 'Ivo', 'Nyx', 'Pip'];
const SYS_LABEL = { weapons: 'WPN', engines: 'ENG', shields: 'SHD' };
let B = null; // battle state
function mkWeapon(key) {
  const w = WEAPONS[key];
  return { key, ...w, charge: 0, ammoLeft: w.ammo ?? null };
}
function mkCrew(n) {
  return Array.from({ length: n }, (_, i) => ({ name: CREW_NAMES[i % CREW_NAMES.length], station: 'weapons', task: 'Manning weapons' }));
}
function mkShip(side, base) {
  return {
    side, name: base.name, hull: base.hull, maxHull: base.hull,
    sh: base.shield, maxSh: base.shield, shTick: 0,
    weapons: base.weapons.map(mkWeapon),
    sys: { weapons: 2, engines: 2, shields: 2 },
    crew: mkCrew(base.crew || 0),
    target: 'weapons', manning: false,
  };
}
function playerLoadout(factionId) {
  if (G.ship) return [G.ship.w1, G.ship.w2];
  if (factionId === 'spirats') return ['laser', 'missile'];
  if (factionId === 'webes') return ['ion', 'laser'];
  return ['laser', 'laser'];
}
// stance: hostile (straight fight) | parley (talk first, holds fire) | friendly (no battle, gift)
function rollStance(who, danger, rng) {
  const r = rng();
  if (who === 'pirates') return r < 0.65 ? 'hostile' : 'parley';
  if (who === 'patrol') return r < 0.4 ? 'friendly' : r < 0.75 ? 'parley' : 'hostile';
  if (who === 'trap') return 'hostile';
  return danger >= 4 ? 'hostile' : r < 0.5 ? 'hostile' : 'parley';
}
function pickFoe(who, danger, rng) {
  if (who === 'patrol') return danger >= 4 ? 'frigate' : 'scout';
  if (who === 'trap') return 'scout';
  const r = rng();
  if (danger >= 4) return r < 0.5 ? 'raider' : 'frigate';
  return r < 0.6 ? 'scout' : r < 0.85 ? 'drone' : 'raider';
}
const TRIBUTE = { pirates: 8, patrol: 6, trap: 8, combat: 8 };
function startBattle(who, foeKey, stance) {
  const foe = FOES[foeKey];
  const player = mkShip('player', {
    name: 'Your Ship', hull: G.hull, shield: 2,
    weapons: playerLoadout(G.faction.id), crew: 2,
  });
  player.maxHull = G.maxHull;
  const enemy = mkShip('foe', foe);
  const scale = 1 + dangerOf(G.nodes.get(G.current)) * 0.06;
  enemy.hull = enemy.maxHull = Math.round(enemy.hull * scale);
  B = {
    who, foeKey, stance, over: false, paused: false, tick: 0,
    player, enemy, log: [], parleyTicks: stance === 'parley' ? 8 : 0,
    surrenderOffered: false, tribute: Math.max(4, Math.round((TRIBUTE[who] || 8) * (G.faction.id === 'spirats' && who === 'pirates' ? 0.5 : 1))),
  };
  hideModal();
  battleEl.classList.remove('hidden');
  blog(`⚔️ ${foe.name} blocks your path!`);
  if (stance === 'parley') blog('📻 They hail you. Talk or open fire — holding position.');
  paintBattle();
  B.timer = setInterval(battleTick, 600);
  window.__sd.battle = B;
}
function blog(msg) {
  B.log.unshift(`<div>t${B.tick} · ${msg}</div>`);
  if (B.log.length > 30) B.log.pop();
  const el = document.getElementById('b-log');
  if (el) el.innerHTML = B.log.join('');
}
function chargeRate(ship) {
  return (ship.manning ? 1.35 : 1) * (ship.sys.weapons > 0 ? 1 : 0.4);
}
function battleTick() {
  if (!B || B.over || B.paused) return;
  B.tick++;
  for (const ship of [B.player, B.enemy]) {
    if (ship.sys.shields > 0 && ++ship.shTick >= 4) { ship.shTick = 0; ship.sh = Math.min(ship.maxSh, ship.sh + 1); }
    for (const w of ship.weapons) {
      if (w.ammoLeft === 0) continue;
      w.charge += chargeRate(ship);
      if (w.charge >= w.cd) {
        w.charge = 0;
        if (w.ammoLeft != null) w.ammoLeft--;
        const foe = ship.side === 'player' ? B.enemy : B.player;
        if (ship.side === 'player' && !B.player.autofireOff) fireWeapon(ship, w, foe, ship.target);
        else if (ship.side === 'foe') {
          if (B.parleyTicks > 0) { w.charge = w.cd; continue; } // holding fire during parley
          fireWeapon(ship, w, foe, ['weapons', 'engines', 'shields'][Math.floor(Math.random() * 3)]);
        }
      }
    }
  }
  if (B.parleyTicks > 0) { B.parleyTicks--; if (B.parleyTicks === 0) blog('📻 Patience over. They charge weapons!'); }
  if (B.tick % 2 === 0) crewAI(B.player);
  if (B.tick % 2 === 0) crewAI(B.enemy);
  // enemy morale: weak non-zealots surrender or flee
  const foe = B.enemy;
  if (!B.surrenderOffered && foe.hull <= foe.maxHull * 0.35 && foe.hull > 0) {
    B.surrenderOffered = true;
    if (B.stance !== 'hostile' || Math.random() < 0.5) {
      B.paused = true;
      blog('🏳️ They offer surrender! Accept loot or finish them.');
      paintBattle();
      return;
    }
  }
  if (B.stance === 'hostile' && foe.hull <= foe.maxHull * 0.25 && foe.sys.engines > 0 && Math.random() < 0.2) {
    blog(`💨 ${foe.name} jumps away!`);
    return endBattle('escaped');
  }
  paintBattle();
}
function fireWeapon(ship, w, foe, targetSys) {
  const evade = foe.sys.engines >= 2 && Math.random() < 0.2;
  if (evade || Math.random() > 0.85) { blog(`${ship.side === 'player' ? 'You' : foe.name} miss${ship.side === 'player' ? '' : 'es'} (${w.name}).`); return; }
  let dmg = w.dmg, sysDmg = w.sys;
  if (foe.sh > 0 && dmg > 0) { foe.sh--; dmg--; blog(`🛡️ Shield absorbs ${w.name}.`); if (dmg <= 0 && w.key !== 'missile') return; }
  if (dmg > 0) {
    foe.hull -= dmg;
    blog(`${ship.side === 'player' ? '💥 Hit!' : '🔥 Hull hit!'} ${w.name} → ${dmg} (${foe.name} ${Math.max(0, foe.hull)}).`);
  }
  if (sysDmg > 0 && Math.random() < 0.65) {
    const sys = w.key === 'ion' ? targetSys : ['weapons', 'engines', 'shields'][Math.floor(Math.random() * 3)];
    if (foe.sys[sys] > 0) {
      foe.sys[sys] = Math.max(0, foe.sys[sys] - (w.key === 'ion' ? 2 : 1));
      blog(`⚙️ ${foe.name} ${SYS_LABEL[sys]} damaged (${foe.sys[sys]} left).`);
    }
  }
  if (foe.side === 'player') { G.hull = Math.max(0, B.player.hull); paintHUD(); }
  if (foe.hull <= 0) return endBattle(foe.side === 'foe' ? 'victory' : 'defeat');
}
// auto-crew: repair damaged systems first, else man weapons, else bridge. No manual orders.
function crewAI(ship) {
  ship.manning = false;
  const dmg = ['weapons', 'engines', 'shields'].find(s => ship.sys[s] < 2);
  let rep = 0;
  for (const c of ship.crew) {
    if (dmg && rep < 1) { c.station = dmg; c.task = `Repairing ${SYS_LABEL[dmg]}`; rep++; }
    else if (!ship.manning) { c.station = 'weapons'; c.task = 'Manning weapons'; ship.manning = true; }
    else { c.station = 'bridge'; c.task = 'On bridge'; }
  }
  if (dmg && ship.crew.some(c => c.station === dmg)) {
    ship['repair_' + dmg] = (ship['repair_' + dmg] || 0) + 1;
    if (ship['repair_' + dmg] >= 4) {
      ship['repair_' + dmg] = 0;
      ship.sys[dmg] = Math.min(2, ship.sys[dmg] + 1);
      blog(`🔧 ${ship.side === 'player' ? 'Crew restores' : ship.name + ' restores'} ${SYS_LABEL[dmg]}.`);
    }
  }
}
function endBattle(result) {
  if (!B || B.over) return;
  B.over = true;
  clearInterval(B.timer);
  G.hull = Math.max(0, B.player.hull);
  battleEl.classList.add('hidden');
  const foe = B.enemy;
  if (result === 'victory') {
    G.kills++;
    const loot = lootFor(B.who);
    const drop = Math.random() < 0.3 ? foe.weapons[Math.floor(Math.random() * foe.weapons.length)].key : null;
    const ev = {
      title: `${foe.name} Destroyed`, desc: `Salvage secured. +${loot[0]} fuel, +${loot[1]} scrap.${drop ? ` They carried a ${WEAPONS[drop].name}!` : ''}`,
      choices: drop
        ? [{ t: `Take the ${WEAPONS[drop].name}`, req: {}, loot, swap: drop, out: [[`Weapon installed.`, loot]] },
           { t: 'Leave it, take salvage', req: {}, loot, out: [[`Salvage secured.`, loot]] }]
        : [{ t: 'Collect salvage', req: {}, loot, out: [[`Salvage secured.`, loot]] }],
    };
    G.activeEvent = ev;
    paintHUD();
    showModal();
  } else if (result === 'defeat') {
    gameOver(`${foe.name} tore your ship apart.`);
  } else if (result === 'fled') {
    G.fuel = Math.max(0, G.fuel - 2);
    paintHUD();
    banner('Burned 2 fuel to escape.');
  } else if (result === 'escaped') {
    banner(`${foe.name} escaped.`);
  } else if (result === 'deal') {
    paintHUD();
    banner('Deal struck. You part ways.');
  }
  B = null;
  window.__sd.battle = null;
}
function lootFor(who) {
  const d = dangerOf(G.nodes.get(G.current));
  const base = { pirates: [2, 12], patrol: [3, 10], trap: [2, 8], combat: [3, 12] }[who] || [2, 10];
  return [base[0] + Math.floor(d / 2), base[1] + d * 2, 0];
}
// player actions (buttons; battle runs in real time)
function bTarget() {
  if (!B || B.over) return;
  const order = ['weapons', 'engines', 'shields'];
  B.player.target = order[(order.indexOf(B.player.target) + 1) % 3];
  blog(`🎯 Targeting ${SYS_LABEL[B.player.target]}.`);
  paintBattle();
}
function bPause() {
  if (!B || B.over) return;
  B.paused = !B.paused;
  blog(B.paused ? '⏸ Paused.' : '▶ Resumed.');
  paintBattle();
}
function bFlee() {
  if (!B || B.over || B.paused) return;
  const eng = B.player.sys.engines;
  if (eng <= 0) { blog('❌ Engines dead — cannot flee!'); paintBattle(); return; }
  if (Math.random() < 0.55 + 0.1 * eng) {
    blog('💨 Jump plotted — escaping!');
    endBattle('fled');
  } else {
    blog('❌ Flee failed! They fire a parting volley.');
    for (const w of B.enemy.weapons) if (w.charge >= w.cd - 2) { w.charge = w.cd; }
    battleTick();
  }
}
function bTalk(mode) {
  if (!B || B.over) return;
  B.player.autofireOff = mode === 'talk' ? true : B.player.autofireOff;
  if (mode === 'talk') {
    B.stance = 'parley'; B.parleyTicks = Math.max(B.parleyTicks, 6);
    blog(`📻 You open a channel. Demand tribute, pay ${B.tribute} scrap, or open fire.`);
  } else if (mode === 'fire') {
    B.player.autofireOff = false; B.stance = 'hostile'; B.parleyTicks = 0;
    blog('🔥 Weapons free!');
  } else if (mode === 'pay') {
    if (G.scrap < B.tribute) { blog('❌ Not enough scrap.'); paintBattle(); return; }
    G.scrap -= B.tribute;
    blog(`🤝 Paid ${B.tribute} scrap. They let you pass.`);
    endBattle('deal');
    return;
  } else if (mode === 'demand') {
    const intimidate = G.kills * 0.12 + (B.enemy.hull < B.enemy.maxHull * 0.6 ? 0.35 : 0);
    if (Math.random() < 0.25 + intimidate) {
      const loot = lootFor(B.who);
      blog(`😤 They yield! +${loot[0]} fuel, +${loot[1]} scrap.`);
      G.activeEvent = { key: '__loot', nodeId: G.current, ev: { title: 'Tribute Paid', desc: 'They hand over cargo and withdraw.', choices: [{ t: 'Take it', req: {}, loot, out: [['Tribute secured.', loot]] }] } };
      B.over = true; clearInterval(B.timer);
      battleEl.classList.add('hidden');
      B = null; window.__sd.battle = null;
      paintHUD(); showModal();
    } else {
      blog('😡 They laugh at your demand. Weapons free!');
      B.stance = 'hostile'; B.parleyTicks = 0; B.player.autofireOff = false;
    }
  } else if (mode === 'accept') {
    const loot = [Math.floor(B.enemy.maxHull / 6), Math.floor(B.enemy.maxHull / 2), 0];
    blog(`🏳️ Surrender accepted. +${loot[0]} fuel, +${loot[1]} scrap.`);
    G.activeEvent = { key: '__loot', nodeId: G.current, ev: { title: 'Surrender Accepted', desc: 'They jettison cargo and limp away.', choices: [{ t: 'Take it', req: {}, loot, out: [['Cargo secured.', loot]] }] } };
    B.over = true; clearInterval(B.timer);
    battleEl.classList.add('hidden');
    B = null; window.__sd.battle = null;
    paintHUD(); showModal();
  } else if (mode === 'refuse') {
    B.paused = false; B.stance = 'hostile';
    blog('🔥 No mercy. Finish them!');
  }
  paintBattle();
}

function sysPips(ship) {
  return ['weapons', 'engines', 'shields'].map(s =>
    `<span class="sys ${ship.sys[s] ? '' : 'off'}" title="${SYS_LABEL[s]}">${SYS_LABEL[s]}${'●'.repeat(ship.sys[s])}${'○'.repeat(2 - ship.sys[s])}</span>`).join('');
}
function shipCard(ship, foe) {
  const hullPct = Math.max(0, Math.round(100 * ship.hull / ship.maxHull));
  const weapons = ship.weapons.map(w => {
    const pct = Math.min(100, Math.round(100 * w.charge / w.cd));
    const ammo = w.ammoLeft != null ? ` ×${w.ammoLeft}` : '';
    return `<div class="wrow"><span style="color:${w.color}">▮ ${w.name}${ammo}</span><div class="wbar"><i style="width:${pct}%;background:${w.color}"></i></div></div>`;
  }).join('');
  const rooms = ['weapons', 'engines', 'shields', 'bridge'].map(r => {
    const here = ship.crew.filter(c => c.station === (r === 'bridge' ? 'bridge' : r));
    const dots = here.map(c => `<span class="cdot" title="${c.name}: ${c.task}">${c.name[0]}</span>`).join('');
    return `<div class="room" data-r="${r}"><b>${r === 'bridge' ? 'BRD' : SYS_LABEL[r]}</b>${dots}</div>`;
  }).join('');
  const tasks = ship.crew.map(c => `<div class="ctask">${c.name}: ${c.task}</div>`).join('');
  const sh = '⬢'.repeat(ship.sh) + '◇'.repeat(Math.max(0, ship.maxSh - ship.sh));
  return `<div class="ship ${foe ? 'foe' : ''}">
    <div class="shead"><b>${ship.name}</b><span class="shp">${sh}</span></div>
    <div class="hbar"><i style="width:${hullPct}%"></i><span>${Math.max(0, Math.ceil(ship.hull))}/${ship.maxHull}</span></div>
    <div class="sysrow">${sysPips(ship)}</div>
    <div class="wlist">${weapons}</div>
    <div class="rooms">${rooms}</div>
    <div class="crewlog">${tasks || '<div class="ctask">No crew — automated.</div>'}</div>
  </div>`;
}
function paintBattle() {
  if (!B) return;
  const foe = document.getElementById('b-foe'), pl = document.getElementById('b-player'), act = document.getElementById('b-actions');
  foe.innerHTML = shipCard(B.enemy, true);
  pl.innerHTML = shipCard(B.player, false);
  let btns = '';
  if (B.paused && B.surrenderOffered) {
    btns = `<button class="btn btn-primary" onclick="bTalk('accept')">Accept surrender</button>
      <button class="btn" onclick="bTalk('refuse')">No mercy</button>`;
  } else if (B.stance === 'parley' && B.parleyTicks > 0) {
    btns = `<button class="btn btn-primary" onclick="bTalk('fire')">🔥 Open fire</button>
      <button class="btn" onclick="bTalk('demand')">😤 Demand tribute</button>
      <button class="btn" onclick="bTalk('pay')">🤝 Pay ${B.tribute} scrap</button>`;
  } else {
    btns = `<button class="btn" onclick="bTarget()">🎯 ${SYS_LABEL[B.player.target]}</button>
      <button class="btn" onclick="bTalk('talk')">📻 Talk</button>
      <button class="btn" onclick="bFlee()">💨 Flee</button>
      <button class="btn" onclick="bPause()">${B.paused ? '▶' : '⏸'}</button>`;
  }
  act.innerHTML = btns + `<div id="b-log">${B.log.join('')}</div>`;
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
const battleEl = document.getElementById('battle');
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
  mouse: { x: 0.5, y: 0.5 }, // normalized hover, far-layer drift
};

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
  if (B) { clearInterval(B.timer); B = null; window.__sd.battle = null; }
  battleEl.classList.add('hidden');
  G.faction = faction;
  G.rng = mulberry32((Math.random() * 0xFFFFFFFF) >>> 0);
  G.nodes.clear(); G.nextId = 1;
  G.fuel = 50 + (faction.id === 'cosmicons' ? 15 : faction.id === 'webes' ? 10 : 0);
  G.scrap = 15 + (faction.id === 'spirats' ? 10 : 0);
  G.maxHull = 100;
  G.hull = 100 + (faction.id === 'celestials' ? 20 : 0);
  G.jumps = 0; G.kills = 0;
  G.ship = faction.id === 'spirats' ? { w1: 'laser', w2: 'missile' }
    : faction.id === 'webes' ? { w1: 'ion', w2: 'laser' } : { w1: 'laser', w2: 'laser' };
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
  if (!ev || !ev.choices || G.outcome) return;
  const c = ev.choices[i];
  if (!c || !canPay(c.req || {})) return;
  // loot pickup (post-battle / tribute / surrender)
  if (c.loot) {
    applyDelta(c.loot);
    if (c.swap) { G.ship.w2 = c.swap; }
    paintHUD();
    return closeEvent(`${ev.title} — ${c.out[0][0]}`);
  }
  // ship combat routing: stance roll decides hostile / parley / friendly
  if (c.battle) {
    const who = c.battle;
    const node = G.nodes.get(G.current);
    const danger = dangerOf(node);
    const stance = rollStance(who, danger, G.rng);
    G.activeEvent = null;
    if (stance === 'friendly') {
      const gift = who === 'patrol' ? [3, 4, 0] : [2, 6, 0];
      applyDelta(gift);
      paintHUD();
      hideModal();
      return banner(`FRIENDLY CONTACT · +${gift[0]} FUEL +${gift[1]} SCRAP`);
    }
    return startBattle(who, pickFoe(who, danger, G.rng), stance);
  }
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
  const [text, delta, extra] = c.out[pick];
  applyDelta(delta);
  paintHUD();
  if (extra && extra.battle) {
    G.activeEvent = null;
    return startBattle(extra.battle, 'scout', 'hostile');
  }
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

// ---------- parallax galaxy: procedural layers, infinite, self-culling ----------
// Each layer lives in camera space scaled by f: only visible cells are hashed,
// so off-screen stars are never computed (frustum culling by construction).
const LAYERS = [
  { f: 0.15, cell: 190, sMin: 0.5, sMax: 1.2, a: 0.45, tint: 0.05 },
  { f: 0.35, cell: 150, sMin: 0.8, sMax: 1.9, a: 0.65, tint: 0.08 },
  { f: 0.6, cell: 115, sMin: 1.2, sMax: 2.7, a: 0.9, tint: 0.11 },
];
const TINTS = ['#7dd3fc', '#fcd34d', '#c4b5fd']; // sparse blue/amber/violet
function hash2(x, y, salt) {
  let h = ((x * 374761393 + y * 668265263 + salt * 974634211) >>> 0);
  h = (h ^ (h >>> 13)) >>> 0; h = (h * 1274126177) >>> 0; h = (h ^ (h >>> 16)) >>> 0;
  return h / 0xFFFFFFFF;
}
function drawStars(t) {
  const { x: cx, y: cy, z } = G.cam;
  for (let li = 0; li < LAYERS.length; li++) {
    const L = LAYERS[li];
    // hover drift: far layers breathe with the pointer (±14px on the farthest)
    const hov = li === 0 ? 14 : li === 1 ? 6 : 0;
    const ox = (G.mouse.x - 0.5) * hov, oy = (G.mouse.y - 0.5) * hov;
    const vx0 = cx * L.f - W / 2 / z, vx1 = cx * L.f + W / 2 / z;
    const vy0 = cy * L.f - H / 2 / z, vy1 = cy * L.f + H / 2 / z;
    const c0x = Math.floor(vx0 / L.cell), c1x = Math.floor(vx1 / L.cell);
    const c0y = Math.floor(vy0 / L.cell), c1y = Math.floor(vy1 / L.cell);
    for (let gx = c0x; gx <= c1x; gx++) {
      for (let gy = c0y; gy <= c1y; gy++) {
        const h1 = hash2(gx, gy, li * 7 + 1);
        if (h1 > 0.72) continue; // density: ~72% of cells hold a star
        const wx = (gx + hash2(gx, gy, li * 7 + 2)) * L.cell;
        const wy = (gy + hash2(gx, gy, li * 7 + 3)) * L.cell;
        const sx = (wx - cx * L.f) * z + W / 2 + ox * z;
        const sy = (wy - cy * L.f) * z + H / 2 + oy * z;
        const size = (L.sMin + hash2(gx, gy, li * 7 + 4) * (L.sMax - L.sMin)) * z;
        if (size < 0.4) continue;
        const tw = 0.75 + 0.25 * Math.sin(t * (1 + hash2(gx, gy, li * 7 + 5) * 2) + h1 * TAU);
        ctx.globalAlpha = L.a * tw;
        ctx.fillStyle = hash2(gx, gy, li * 7 + 6) < L.tint
          ? TINTS[Math.floor(hash2(gx, gy, li * 7 + 7) * TINTS.length)]
          : TH.ink;
        ctx.fillRect(sx, sy, size, size);
      }
    }
  }
  ctx.globalAlpha = 1;
}
// Nebula wash: prerendered once, screen-blended, barely-there alphas.
const NEB_SPRITES = [];
function makeNebula(colorInner) {
  const c = document.createElement('canvas');
  c.width = c.height = 256;
  const g = c.getContext('2d');
  const grad = g.createRadialGradient(128, 128, 0, 128, 128, 128);
  grad.addColorStop(0, colorInner);
  grad.addColorStop(1, 'rgba(0,0,0,0)');
  g.fillStyle = grad;
  g.fillRect(0, 0, 256, 256);
  return c;
}
function drawNebulas(t) {
  if (!NEB_SPRITES.length) {
    NEB_SPRITES.push({ img: makeNebula('rgba(109,88,246,0.16)'), s: 620 }, // indigo
      { img: makeNebula('rgba(34,150,180,0.13)'), s: 520 },                 // teal
      { img: makeNebula('rgba(150,90,220,0.11)'), s: 720 });                // violet
  }
  const { x: cx, y: cy, z } = G.cam;
  const f = 0.25, cell = 950;
  const vx0 = cx * f - W / 2 / z - cell, vx1 = cx * f + W / 2 / z + cell;
  const vy0 = cy * f - H / 2 / z - cell, vy1 = cy * f + H / 2 / z + cell;
  ctx.globalCompositeOperation = 'screen';
  for (let gx = Math.floor(vx0 / cell); gx <= Math.floor(vx1 / cell); gx++) {
    for (let gy = Math.floor(vy0 / cell); gy <= Math.floor(vy1 / cell); gy++) {
      const h = hash2(gx, gy, 99);
      if (h > 0.55) continue;
      const sp = NEB_SPRITES[Math.floor(hash2(gx, gy, 98) * NEB_SPRITES.length)];
      const wx = (gx + 0.5 + (hash2(gx, gy, 97) - 0.5) * 0.6) * cell;
      const wy = (gy + 0.5 + (hash2(gx, gy, 96) - 0.5) * 0.6) * cell;
      const sx = (wx - cx * f) * z + W / 2, sy = (wy - cy * f) * z + H / 2;
      const s = sp.s * z * (0.8 + h);
      ctx.globalAlpha = 0.5 + 0.1 * Math.sin(t * 0.4 + h * TAU);
      ctx.drawImage(sp.img, sx - s / 2, sy - s / 2, s, s);
    }
  }
  // congruency: nebula-sector boost + dark-rift vignette
  const cur = G.nodes.get(G.current);
  if (cur && G.screen === 'play') {
    if (cur.type === 'Nebula') {
      ctx.globalAlpha = 0.35;
      const s = 500 * z;
      ctx.drawImage(NEB_SPRITES[0].img, W / 2 - s / 2, H / 2 - s / 2, s, s);
    } else if (cur.type === 'DarkRift') {
      const g = ctx.createRadialGradient(W / 2, H / 2, Math.min(W, H) * 0.3, W / 2, H / 2, Math.max(W, H) * 0.75);
      g.addColorStop(0, 'rgba(0,0,0,0)');
      g.addColorStop(1, `rgba(88,40,140,${0.22 + 0.06 * Math.sin(t * 2)})`);
      ctx.globalAlpha = 1;
      ctx.fillStyle = g;
      ctx.fillRect(0, 0, W, H);
    }
  }
  ctx.globalAlpha = 1;
  ctx.globalCompositeOperation = 'source-over';
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
  if (e.pointerType !== 'touch') { G.mouse.x = e.clientX / W; G.mouse.y = e.clientY / H; }
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
  const t = performance.now() / 1000;
  drawNebulas(t);
  drawStars(t);
  if (G.screen === 'menu' || G.faction === null && G.screen !== 'play') return;
  const { x: cx, y: cy, z } = G.cam;
  const X = wx => (wx - cx) * z + W / 2;
  const Y = wy => (wy - cy) * z + H / 2;
  // links (frustum-culled: skip when both ends are off-screen)
  for (const n of G.nodes.values()) {
    for (const l of n.links) {
      if (l < n.id) continue;
      const m = G.nodes.get(l);
      const x1 = X(n.x), y1 = Y(n.y), x2 = X(m.x), y2 = Y(m.y);
      if ((x1 < -40 && x2 < -40) || (x1 > W + 40 && x2 > W + 40) ||
          (y1 < -40 && y2 < -40) || (y1 > H + 40 && y2 > H + 40)) continue;
      const isCur = n.id === G.current || m.id === G.current;
      const bothVis = n.visited && m.visited;
      ctx.strokeStyle = isCur ? hexA(TH.green, 0.85) : bothVis ? hexA(TH.faint, 0.4) : hexA(TH.muted, 0.35);
      ctx.lineWidth = isCur ? 2.5 : 1.2;
      ctx.beginPath(); ctx.moveTo(x1, y1); ctx.lineTo(x2, y2); ctx.stroke();
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
window.__sd = { G, travelTo, choose, startRun, startBattle, bTarget, bTalk, bFlee, bPause }; // test hook
requestAnimationFrame(frame);
