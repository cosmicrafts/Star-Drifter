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
      { t: 'Answer the call', req: {}, maybeTrap: true, out: [['Grateful crew pays in fuel. +6 fuel, +4 scrap.', [6, 4, 0]]] },
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
  player.autofire = true; // toggle: off = hold charge, click your ship to fire the volley
  const enemy = mkShip('foe', foe);
  const scale = 1 + dangerOf(G.nodes.get(G.current)) * 0.06;
  enemy.hull = enemy.maxHull = Math.round(enemy.hull * scale);
  B = {
    who, foeKey, stance, over: false, paused: false, tick: 0,
    player, enemy, log: [], parleyTicks: stance === 'parley' ? 8 : 0,
    surrenderOffered: false, tribute: Math.max(4, Math.round((TRIBUTE[who] || 8) * (G.faction.id === 'spirats' && who === 'pirates' ? 0.5 : 1))),
  };
  // link actors: player ship + last foe actor get live hp bars
  const S = G.scene;
  if (S) {
    S.player.hp = B.player;
    let foeActor = S.actors.filter(a => a.kind === 'pirate' || a.kind === 'patrol' || a.kind === 'drone').reverse()[0];
    if (!foeActor) {
      // stage a visual enemy so the scene always shows the ship you're fighting
      const foekind = who === 'patrol' ? 'patrol' : who === 'combat' ? 'drone' : 'pirate';
      foeActor = mkActor(foekind, S.anchor.x + 44, S.anchor.y - 12, {
        color: who === 'patrol' ? '#a5b4fc' : who === 'combat' ? '#e879f9' : '#f0abfc',
        face: Math.PI, // 1: enemies face you, not away
      });
      S.actors.push(foeActor);
    }
    foeActor.hp = B.enemy; foeActor.foe = true;
    B.enemy.actor = foeActor;
    S.player.hp.actor = S.player;
  }
  const sp = who === 'pirates' || who === 'trap' ? SPEAKERS.pirate : who === 'patrol' ? SPEAKERS.patrol : SPEAKERS.logging;
  void sp;
  if (stance === 'hostile' && G.scene) {
    const fa = G.scene.actors.find(a => a.foe) || G.scene.actors[G.scene.actors.length - 1];
    if (fa) floatText(fa.x, fa.y - 60, 'HOSTILE!', '#f87171');
  }
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
  return 0.6 * (ship.manning ? 1.35 : 1) * (ship.sys.weapons > 0 ? 1 : 0.4); // 2: seconds per tick (600ms), not 1
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
        const foe = ship.side === 'player' ? B.enemy : B.player;
        if (ship.side === 'player' && !B.player.autofire) { w.charge = w.cd; continue; } // hold at full, fire manually
        w.charge = 0;
        if (w.ammoLeft != null) w.ammoLeft--;
        if (ship.side === 'player') fireWeapon(ship, w, foe, ship.target);
        else {
          if (B.parleyTicks > 0) { w.charge = w.cd; continue; } // holding fire during parley
          fireWeapon(ship, w, foe, ['weapons', 'engines', 'shields'][Math.floor(Math.random() * 3)]);
        }
        if (!B || B.over) return; // this shot may have ended the battle
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
  const A = ship.actor, Ta = shieldVisualFor(foe);
  if (A && Ta) spawnShot(A, Ta, w.color, w.key === 'missile' ? 5 : 3); // both sides, visible
  const evade = foe.sys.engines >= 2 && Math.random() < 0.2;
  if (evade || Math.random() > 0.85) {
    if (A && Ta) floatText(Ta.x, Ta.y - 30, 'MISS', '#94a3b8');
    blog(`${ship.side === 'player' ? 'You' : foe.name} miss${ship.side === 'player' ? '' : 'es'} (${w.name}).`);
    return;
  }
  let dmg = w.dmg, sysDmg = w.sys;
  if (foe.sh > 0 && dmg > 0) {
    foe.sh--; dmg--;
    G.shake = Math.min(10, (G.shake || 0) + 4);
    if (Ta) { Ta.shieldFlash = 1; }
    if (Ta) floatText(Ta.x, Ta.y - 30, 'SHIELD', '#7dd3fc');
    if (dmg <= 0 && w.key !== 'missile') return;
  }
  if (dmg > 0) {
    foe.hull -= dmg;
    G.shake = Math.min(14, (G.shake || 0) + (foe.side === 'player' ? 9 : 5)); // heavy feedback both ways
    if (Ta) { Ta.hitFlash = 1; burst(Ta.x, Ta.y, w.key === 'missile' ? '#fb923c' : '#f87171', w.key === 'missile' ? 22 : 12); }
    if (Ta) floatText(Ta.x, Ta.y - 30, `-${dmg}`, '#f87171');
    if (w.key !== 'ion' && Math.random() < 0.65 && Ta) burst(Ta.x, Ta.y, '#fbbf24', 6); // sparks
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
function shieldVisualFor(ship) {
  return ship.actor || null;
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
  const foe = B.enemy;
  const S = G.scene;
  const foeActor = S ? S.actors.filter(a => a.foe || a.kind === 'pirate' || a.kind === 'patrol' || a.kind === 'drone').reverse()[0] : null;
  if (result === 'victory') {
    if (foeActor) { foeActor.dead = true; burst(foeActor.x, foeActor.y, '#fb923c', 40); burst(foeActor.x, foeActor.y, '#f87171', 24); }
    floatText(foeActor ? foeActor.x : W / 2, foeActor ? foeActor.y : H / 2, 'DESTROYED', '#f87171');
    G.kills++;
    const loot = lootFor(B.who);
    const drop = Math.random() < 0.3 ? foe.weapons[Math.floor(Math.random() * foe.weapons.length)].key : null;
    const ev = {
      title: `${foe.name} Destroyed`,
      desc: `Salvage secured. +${loot[0]} fuel, +${loot[1]} scrap.${drop ? ` They carried a ${WEAPONS[drop].name}!` : ''}`,
      choices: drop
        ? [{ t: `Take the ${WEAPONS[drop].name}`, req: {}, loot, swap: drop, out: [[`Weapon installed.`, loot]] },
           { t: 'Leave it, take salvage', req: {}, loot, out: [[`Salvage secured.`, loot]] }]
        : [{ t: 'Collect salvage', req: {}, loot, out: [[`Salvage secured.`, loot]] }],
    };
    setTimeout(() => {
      codecSay({ ...SPEAKERS.self, text: ev.desc, choices: codecChoicesFor(ev) });
    }, 900);
    paintHUD();
  } else if (result === 'defeat') {
    const pa = S ? S.player : null;
    if (pa) { pa.dead = true; burst(pa.x, pa.y, '#f87171', 50); }
    setTimeout(() => gameOver(`${foe.name} tore your ship apart.`), 1200);
  } else if (result === 'fled') {
    G.fuel = Math.max(0, G.fuel - 2);
    paintHUD();
    if (S) {
      const pa = S.player;
      const run = setInterval(() => { pa.x -= 26; }, 50);
      setTimeout(() => clearInterval(run), 500);
    }
    banner('Burned 2 fuel to escape.');
    setTimeout(exitScene, 900);
  } else if (result === 'escaped') {
    if (foeActor) { const run = setInterval(() => { foeActor.x += 26; }, 50); setTimeout(() => clearInterval(run), 500); }
    banner(`${foe.name} escaped.`);
    setTimeout(exitScene, 900);
  } else if (result === 'deal') {
    paintHUD();
    banner('Deal struck. You part ways.');
    setTimeout(exitScene, 900);
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
// toggle autofire: off = weapons hold at full charge; click your ship again to fire the volley
function bAuto() {
  if (!B || B.over) return;
  B.player.autofire = !B.player.autofire;
  const me = G.scene ? G.scene.player : null;
  if (B.player.autofire) {
    if (me) floatText(me.x, me.y - 40, 'AUTO-FIRE ON', '#22d3ee');
  } else {
    // fire whatever is already charged, then hold
    let fired = 0;
    for (const w of B.player.weapons) {
      if (w.charge >= w.cd && w.ammoLeft !== 0) {
        w.charge = 0;
        if (w.ammoLeft != null) w.ammoLeft--;
        fireWeapon(B.player, w, B.enemy, B.player.target);
        if (!B || B.over) return;
        fired++;
      }
    }
    if (me) floatText(me.x, me.y - 40, fired ? 'VOLLEY!' : 'MANUAL FIRE', '#fcd34d');
  }
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
  B.player.autofire = mode === 'talk' ? false : B.player.autofire; // hold fire while talking
  if (mode === 'talk') {
    B.stance = 'parley'; B.parleyTicks = Math.max(B.parleyTicks, 6);
    blog(`📻 You open a channel. Demand tribute, pay ${B.tribute} scrap, or open fire.`);
  } else if (mode === 'fire') {
    B.player.autofire = true; B.stance = 'hostile'; B.parleyTicks = 0;
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
      const ev = { title: 'Tribute Paid', desc: 'They hand over cargo and withdraw.', choices: [{ t: 'Take it', req: {}, loot, out: [['Tribute secured.', loot]] }] };
      B.over = true; clearInterval(B.timer);
      B = null; window.__sd.battle = null;
      paintHUD();
      codecSay({ ...SPEAKERS.pirate, text: ev.desc, choices: codecChoicesFor(ev) });
    } else {
      codecSay({ ...SPEAKERS.pirate, text: 'You dare? Weapons free!' });
    }
  } else if (mode === 'accept') {
    const loot = [Math.floor(B.enemy.maxHull / 6), Math.floor(B.enemy.maxHull / 2), 0];
    blog(`🏳️ Surrender accepted. +${loot[0]} fuel, +${loot[1]} scrap.`);
    G.activeEvent = { key: '__loot', nodeId: G.current, ev: { title: 'Surrender Accepted', desc: 'They jettison cargo and limp away.', choices: [{ t: 'Take it', req: {}, loot, out: [['Cargo secured.', loot]] }] } };
    B.over = true; clearInterval(B.timer);
    B = null; window.__sd.battle = null;
    paintHUD();
    codecSay({ ...SPEAKERS.self, text: ev.desc, choices: codecChoicesFor(ev) });
  } else if (mode === 'refuse') {
    B.paused = false; B.stance = 'hostile';
    blog('🔥 No mercy. Finish them!');
  }
  paintBattle();
}

// ---------- battle UI: codec IS the interface (6) ----------
// phase-aware buttons; codec text shows a one-line status, no typewriter spam in fight.
function battlePhase() {
  if (B.paused && B.surrenderOffered) return 'surrender';
  if (B.stance === 'parley' && B.parleyTicks > 0) return 'parley';
  return 'fight';
}
function paintBattle() {
  if (!B) return;
  const phase = battlePhase();
  const uiKey = phase + '|' + (B.player.autofire ? 'auto' : 'man');
  if (uiKey === B.uiPhase) { battleStatusLine(); return; } // buttons stable; refresh text only
  B.uiPhase = uiKey;
  const foeSp = B.who === 'pirates' || B.who === 'trap' ? SPEAKERS.pirate
    : B.who === 'patrol' ? SPEAKERS.patrol
    : { name: B.enemy.name, color: '#e879f9', glyph: '◮' };
  if (phase === 'parley') {
    codecSay({ ...foeSp, text: 'They hold fire — for now. Talk, pay, or shoot first.', choices: [
      { t: `Open fire`, primary: true, fn: () => bTalk('fire') },
      { t: `Demand tribute`, fn: () => bTalk('demand') },
      { t: `Pay ${B.tribute}⛁`, fn: () => bTalk('pay') },
      { t: 'Flee', fn: () => bFlee() },
    ] });
  } else if (phase === 'surrender') {
    codecSay({ ...foeSp, text: 'We surrender! Take the cargo and let us live.', choices: [
      { t: 'Accept surrender', primary: true, fn: () => bTalk('accept') },
      { t: 'No mercy', fn: () => bTalk('refuse') },
    ] });
  } else {
    // real orders: fire control matters, click ships for target/volley
    codecSay({ ...SPEAKERS.self, text: battleStatusText() + ' — 🎯 click enemy ship to refocus', choices: [
      { t: B.player.autofire ? '⚙ AUTO-FIRE: ON' : '✋ MANUAL — click to FIRE', primary: !B.player.autofire, fn: () => bAuto() },
      { t: `🎯 ${SYS_LABEL[B.player.target]}`, fn: () => bTarget() },
      { t: '💨 Flee', fn: () => bFlee() },
    ] });
  }
}
function battleStatusText() {
  if (!B) return '';
  const wps = B.player.weapons.map(w => {
    const pct = Math.min(100, Math.round(100 * w.charge / w.cd));
    const ammo = w.ammoLeft != null ? ` ×${w.ammoLeft}` : '';
    return `${w.name}${ammo} ${pct >= 100 ? 'READY' : pct + '%'}`;
  }).join(' · ');
  return `${B.player.autofire ? '⚙ auto' : '✋ manual'} · ${wps}`;
}
function battleStatusLine() {
  const el = document.getElementById('codec-text');
  if (el && B && battlePhase() === 'fight') el.textContent = battleStatusText();
}

// ---------- codec dialogue (MGS-style) ----------
const codecEl = document.getElementById('codec');
function glyphSVG(glyph, color) {
  return `<svg viewBox="0 0 84 84">
    <defs><radialGradient id="pg" cx="35%" cy="30%"><stop offset="0%" stop-color="${hexA(color, 0.35)}"/><stop offset="100%" stop-color="rgba(2,6,16,0.9)"/></radialGradient></defs>
    <rect width="84" height="84" fill="url(#pg)"/>
    <path d="M0,8 H84 M0,24 H84 M0,40 H84 M0,56 H84 M0,72 H84" stroke="${hexA(color, 0.15)}" stroke-width="1"/>
    <text x="42" y="58" text-anchor="middle" font-size="44" fill="${color}" style="text-shadow:0 0 8px ${color}">${glyph}</text>
    <rect width="84" height="84" fill="none" stroke="${hexA(color, 0.5)}"/>
  </svg>`;
}
function codecPortrait(sp, t) {
  const el = document.getElementById('codec-portrait');
  el.innerHTML = glyphSVG(sp.glyph || '✦', sp.color || '#22d3ee');
  const svg = el.firstChild;
  const textEl = svg.querySelector('text');
  textEl.setAttribute('y', 58 + Math.sin(t * 6) * 1.5);
  svg.style.opacity = 0.8 + 0.2 * Math.sin(t * 8);
}
let codecTimer = null;
function codecSay(sp, t) {
  codecEl.classList.remove('hidden');
  if (codecTimer) clearInterval(codecTimer);
  document.getElementById('codec-name').textContent = sp.name || '???';
  codecPortrait(sp, 0);
  let t0 = performance.now();
  const port = () => codecPortrait(sp, (performance.now() - t0) / 1000);
  let portTick = setInterval(port, 90);
  codecTimer = portTick;
  const textEl = document.getElementById('codec-text');
  textEl.textContent = '';
  const full = sp.text || '';
  let i = 0;
  const typer = setInterval(() => {
    i += 2;
    textEl.textContent = full.slice(0, i);
    if (i % 8 === 0) {
      try { blip(520 + (full.charCodeAt(i % full.length) % 5) * 40); } catch (e) { }
    }
    if (i >= full.length) {
      clearInterval(typer);
      const box = document.getElementById('codec-choices');
      box.innerHTML = '';
      (sp.choices || []).forEach((c, ci) => {
        const b = document.createElement('button');
        b.className = 'btn' + (c.primary ? ' btn-primary' : '');
        b.innerHTML = `<b>${ci + 1}. ${c.t}</b>`;
        if (c.locked) b.classList.add('locked');
        b.addEventListener('click', () => { if (!c.locked) { clearInterval(portTick); c.fn(); } });
        box.appendChild(b);
      });
    }
  }, 18);
}
function codecClear() {
  codecEl.classList.add('hidden');
  document.getElementById('codec-choices').innerHTML = '';
  if (codecTimer) clearInterval(codecTimer);
  codecTimer = null;
}
function blip(freq) {
  const AC = window.AudioContext || window.webkitAudioContext;
  if (!AC) return;
  const ctx = blip.ctx || (blip.ctx = new AC());
  if (ctx.state === 'suspended') { ctx.resume().catch(() => { }); return; }
  const o = ctx.createOscillator(), g = ctx.createGain();
  o.type = 'square'; o.frequency.value = freq;
  g.gain.value = 0.012;
  o.connect(g); g.connect(ctx.destination);
  o.start();
  o.stop(ctx.currentTime + 0.03);
}
const SPEAKERS = {
  merchant: { name: 'TRAVELING MERCHANT', color: '#fbbf24', glyph: '⌖' },
  pirate: { name: 'SPIRAT RAIDER', color: '#f0abfc', glyph: '☠' },
  patrol: { name: 'ORDER PATROL AI', color: '#a5b4fc', glyph: '▨' },
  distress: { name: 'DISTRESS SIGNAL', color: '#fb923c', glyph: '⨂' },
  station: { name: 'DOCKMASTER', color: '#fbbf24', glyph: '⌂' },
  anomaly: { name: 'UNKNOWN SIGNAL', color: '#c084fc', glyph: '◮' },
  ruins: { name: 'CELESTIAL PRESENCE', color: '#7dd3fc', glyph: '☥' },
  rift: { name: 'THE RIFT', color: '#e879f9', glyph: '✦' },
  mining: { name: 'AGING MINER', color: '#22d3ee', glyph: '⛏' },
  logging: { name: 'SHIP COMPUTER', color: '#94a3b8', glyph: '⌬' },
  self: { name: 'SHIP COMPUTER', color: '#94a3b8', glyph: '⌬' },
};

// ---------- procedural staging per node ----------
function mkActor(kind, x, y, opts) {
  const a = Object.assign({ kind, x, y, seed: Math.random() * TAU, face: kind === 'foe' ? Math.PI : 0 }, opts || {});
  if (opts && opts.orb) { a.ax = x; a.ay = y; a.orbPh = Math.random() * TAU; a.orbSp = 0.25 + Math.random() * 0.2; }
  return a;
}
const SECTOR_STAGE = {
  Station: n => [mkActor('station', n.x + 5, n.y - 25, { color: '#fbbf24' })],
  Nebula: n => [mkActor('anomaly', n.x - 30, n.y + 40, { color: '#818cf8' })],
  AsteroidField: n => [mkActor('rocks', n.x, n.y - 40, {}), mkActor('rocks', n.x + 5, n.y + 45, {})],
  CelestialSite: n => [mkActor('ruins', n.x + 5, n.y - 30, { color: '#7dd3fc' })],
  DarkRift: n => [mkActor('rift', n.x + 5, n.y - 30, { color: '#e879f9' })],
};
const SCENES = {
  combat: (n, who) => {
    const col = who === 'pirates' ? '#f0abfc' : who === 'patrol' ? '#a5b4fc' : '#e879f9';
    const kind = who === 'pirates' ? 'pirate' : who === 'patrol' ? 'patrol' : 'drone';
    const m = who === 'pirates' ? [mkActor(kind, n.x + 42, n.y - 16, { color: col, face: Math.PI }), mkActor(kind, n.x + 55, n.y + 20, { color: '#f0abfc', face: Math.PI })] : [mkActor(kind, n.x + 44, n.y - 12, { color: col, face: Math.PI })];
    return m;
  },
  merchant: n => [mkActor('merchant', n.x + 44, n.y - 12, { color: '#fbbf24', orb: 5 })],
  patrol: n => SCENES.combat(n, 'patrol'),
  derelict: n => [mkActor('derelict', n.x + 34, n.y - 20, {})],
  anomaly: n => [mkActor('anomaly', n.x + 38, n.y - 14, {})],
  celestial: n => [mkActor('ruins', n.x + 5, n.y - 30, { color: '#7dd3fc' })],
  darkrift: n => [mkActor('rift', n.x + 5, n.y - 30, { color: '#e879f9' })],
  mining: n => [mkActor('rocks', n.x, n.y - 40, {}), mkActor('rocks', n.x - 20, n.y + 40, {})],
  distress: n => [mkActor('derelict', n.x + 32, n.y - 18, {})],
  station: n => SECTOR_STAGE.Station(n),
  pirates: n => SCENES.combat(n, 'pirates'),
  trap: n => SCENES.combat(n, 'pirates'),
};

// ---------- scene flow: click node -> fly -> resolve ----------
function enterScene(node) {
  G.scene = {
    nodeId: node.id, anchor: { x: node.x, y: node.y },
    actors: [], parts: [], shots: [], floats: [],
    player: mkActor('player', node.x - 42, node.y + 6, { color: '#22d3ee', face: 0 }),
  };
  G.scene.player.crewLine = 'Rook · Vex';
}
function exitScene() {
  codecClear();
  const node = G.nodes.get(G.current);
  if (!node) { G.scene = null; return; }
  G.scene = null;
  const mv = G.mapView || { z: Math.min(1.1, Math.min(W, H) / 820) };
  flyTo(mv.x ?? node.x, mv.y ?? node.y, mv.z, 900, () => { });
}
function travelTo(id) {
  if (G.activeEvent || G.scene || G.screen !== 'play') return;
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
  // 1-2: ship flies along the link, camera follows and dives INTO the node
  G.mapView = { x: cur.x, y: cur.y, z: G.cam.z };
  G.traveler = { x1: cur.x, y1: cur.y, x2: node.x, y2: node.y, dx: node.x - cur.x, dy: node.y - cur.y };
  const zi = clamp(Math.min(W, H) / 90, 3.5, 9); // deep: node fills the screen
  flyTo(node.x, node.y, zi, 1400, () => {
    G.traveler = null;
    enterScene(node);
    beginEncounter(node); // 5: no slop — codec opens only with the real event
  });
}

// 4: every node = its own environment, drawn dense at scene scale
const ENV = {
  Nebula: (ctx, t) => {
    for (let i = 0; i < 6; i++) {
      const h1 = hash2(7, i, 1), h2 = hash2(7, i, 2);
      const r = 28 + h1 * 30, ox = (h1 - 0.5) * 160, oy = (h2 - 0.5) * 140;
      const g = ctx.createRadialGradient(ox, oy, 0, ox, oy, r);
      g.addColorStop(0, i % 2 ? 'rgba(129,140,248,0.14)' : 'rgba(216,180,254,0.12)');
      g.addColorStop(1, 'rgba(0,0,0,0)');
      ctx.fillStyle = g;
      ctx.beginPath(); ctx.arc(ox, oy + Math.sin(t * 0.6 + i) * 4, r, 0, TAU); ctx.fill();
    }
  },
  AsteroidField: (ctx) => {
    for (let i = 0; i < 14; i++) {
      const h1 = hash2(3, i, 3), h2 = hash2(3, i, 4);
      const rx = (h1 - 0.5) * 190, ry = (h2 - 0.5) * 160, rr = (2 + h1 * 5);
      poly([rx + rr, ry, rx + rr * 0.4, ry - rr * 0.8, rx - rr * 0.7, ry - rr * 0.2, rx - rr * 0.5, ry + rr * 0.6], '#1f2937', '#334155');
    }
  },
  Station: (ctx, t) => {
    ctx.strokeStyle = 'rgba(251,191,36,0.12)'; ctx.lineWidth = 1;
    for (let i = 0; i < 3; i++) { ctx.beginPath(); ctx.arc(0, 0, 60 + i * 30, 0, TAU); ctx.stroke(); }
    for (let i = 0; i < 6; i++) { // cargo specks drifting
      const an = hash2(9, i, 5) * TAU + t * 0.05;
      ctx.fillStyle = 'rgba(148,163,184,0.5)';
      ctx.fillRect(Math.cos(an) * (70 + i * 14), Math.sin(an) * (70 + i * 14), 1.4, 1.4);
    }
  },
  Combat: (ctx, t) => {
    for (let i = 0; i < 8; i++) { // wreck shards + drifting sparks
      const h1 = hash2(11, i, 6), h2 = hash2(11, i, 7);
      const rx = (h1 - 0.5) * 180, ry = (h2 - 0.5) * 150;
      poly([rx + 6, ry, rx, ry - 4, rx - 7, ry + 3], '#111827', '#374151');
    }
    if (Math.sin(t * 5) > 0.6) {
      ctx.fillStyle = 'rgba(248,113,113,0.6)';
      ctx.fillRect((hash2(2, 2, 8) - 0.5) * 120, (hash2(2, 3, 8) - 0.5) * 100, 2, 2);
    }
  },
  Distress: (ctx, t) => {
    const p = (t * 0.5) % 1;
    ctx.strokeStyle = `rgba(251,146,60,${0.5 * (1 - p)})`; ctx.lineWidth = 1.5;
    ctx.beginPath(); ctx.arc(30, -20, 10 + p * 70, 0, TAU); ctx.stroke();
  },
  DarkRift: (ctx, t) => {
    const g = ctx.createRadialGradient(0, 0, 0, 0, 0, 90);
    g.addColorStop(0, 'rgba(0,0,0,0.85)');
    g.addColorStop(0.6, 'rgba(20,5,40,0.5)');
    g.addColorStop(1, 'rgba(0,0,0,0)');
    ctx.fillStyle = g; ctx.beginPath(); ctx.arc(0, 0, 90, 0, TAU); ctx.fill();
    for (let i = 0; i < 6; i++) { // particles sucked inward
      const an = hash2(5, i, 9) * TAU - t * 0.8, rr = 30 + (hash2(5, i, 10) * 60 * (0.5 + 0.5 * Math.sin(t + i)));
      ctx.fillStyle = 'rgba(232,121,249,0.5)';
      ctx.fillRect(Math.cos(an) * rr, Math.sin(an) * rr, 1.5, 1.5);
    }
  },
  CelestialSite: (ctx, t) => {
    for (let i = 0; i < 10; i++) {
      const h1 = hash2(13, i, 11), h2 = hash2(13, i, 12);
      const blink = 0.3 + 0.7 * Math.abs(Math.sin(t * 1.5 + i));
      ctx.fillStyle = `rgba(251,191,36,${0.5 * blink})`;
      ctx.beginPath(); ctx.arc((h1 - 0.5) * 170, (h2 - 0.5) * 140, 0.8, 0, TAU); ctx.fill();
    }
  },
  AetheriumField: (ctx, t) => {
    for (let i = 0; i < 8; i++) {
      const h1 = hash2(17, i, 13), h2 = hash2(17, i, 14);
      const rx = (h1 - 0.5) * 180, ry = (h2 - 0.5) * 150, rr = 3 + h1 * 6;
      ctx.fillStyle = `rgba(34,211,238,${0.25 + 0.15 * Math.sin(t * 2 + i)})`;
      poly([rx, ry - rr, rx + rr * 0.7, ry, rx, ry + rr, rx - rr * 0.7, ry], ctx.fillStyle);
    }
  },
  Empty: (ctx) => {
    for (let i = 0; i < 12; i++) {
      const h1 = hash2(23, i, 15), h2 = hash2(23, i, 16);
      ctx.fillStyle = 'rgba(148,163,184,0.35)';
      ctx.fillRect((h1 - 0.5) * 220, (h2 - 0.5) * 190, 1, 1);
    }
  },
};
function drawEnv(node, t) {
  const env = ENV[node.type];
  ctx.save();
  ctx.translate(X_FX(node.x), Y_FX(node.y));
  ctx.scale(G.cam.z, G.cam.z);
  if (env) env(ctx, t);
  ctx.restore();
}

function beginEncounter(node) {
  let key = SECTOR_EVENT[node.type];
  if (!key || G.rng() < 0.3) key = randomEvent(G.rng, dangerOf(node));
  const source = G.encounters.has(node.id) ? 'empty' : key;
  const ev = JSON.parse(JSON.stringify(EVENTS[source]));
  ev.subtitle = `${node.name} · ${node.type.toUpperCase()}`;
  G.activeEvent = ev;
  G.outcome = null;
  G.encounters.add(node.id);
  // 4: sector environment + procedural actors + opening codec
  const sector = (SECTOR_STAGE[node.type] ? SECTOR_STAGE[node.type](node) : []);
  const actors = (SCENES[source] || (() => []))(node);
  G.scene.actors = G.scene.actors.concat(sector, actors);
  const sp = SPEAKERS[source] || SPEAKERS.self;
  codecSay({ ...sp, text: ev.desc, choices: codecChoicesFor(ev) });
}
function codecChoicesFor(ev) {
  return ev.choices.map((c, i) => ({
    t: c.t, locked: !canPay(c.req || {}), primary: i === 0,
    fn: () => { if (canPay(c.req || {})) { codecClear(); choose(i); } },
  }));
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
  scene: null, camAnim: null, encounters: new Set(),
  best: +(localStorage.getItem('sd-best') || 0),
  rng: mulberry32(1),
  mouse: { x: 0.5, y: 0.5 }, // normalized hover, far-layer drift
  shake: 0, // screenshake magnitude
  hoverId: 0, // node under the cursor (map tooltip)
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
  codecClear();
  G.scene = null; G.camAnim = null; G.traveler = null;
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
  G.scene = null; G.camAnim = null; G.traveler = null; G.encounters = new Set();
  const start = addNode('Station', 0, 0);
  G.nodes.get(start).visited = true;
  G.encounters.add(start);
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
function openEventFor(node) { beginEncounter(node); }

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
  if (c.maybeTrap && G.rng() < 0.35) {
    G.activeEvent = null;
    return startBattle('trap', 'scout', 'hostile');
  }
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
  if (G.hull <= 0) {
    const pa = G.scene ? G.scene.player : null;
    if (pa) { pa.dead = true; burst(pa.x, pa.y, '#f87171', 40); }
    return gameOver('HULL BREACHED — CLAIMED BY THE RIFT');
  }
  if (G.fuel <= 0) return gameOver('OUT OF FUEL — ADRIFT IN THE RIFT');
  banner(outcomeText.split('—')[1]?.trim().toUpperCase().slice(0, 60) || 'EVENT RESOLVED');
  setTimeout(exitScene, 800);
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

// ---------- hud ----------
function hideModal() { modalEl.classList.add('hidden'); }
function paintHUD() {
  const hullPct = Math.round(100 * G.hull / G.maxHull);
  const fuelPct = Math.round(100 * G.fuel / 100);
  hudEl.innerHTML =
    `<div class="hud-group">` +
      `<span class="hud-chip" style="color:${G.faction.color}">⬢ ${G.faction.name}</span>` +
    `</div>` +
    `<div class="hud-group">` +
      `<div class="hud-bar-container" title="HULL: ${Math.ceil(G.hull)}/${G.maxHull}">` +
        `<span class="hud-bar-label">HULL</span>` +
        `<div class="hud-bar"><i style="width:${hullPct}%; background: var(--green);"></i></div>` +
        `<span class="hud-bar-val">${Math.ceil(G.hull)}</span>` +
      `</div>` +
      `<div class="hud-bar-container" title="FUEL: ${Math.floor(G.fuel)}">` +
        `<span class="hud-bar-label">FUEL</span>` +
        `<div class="hud-bar"><i style="width:${Math.min(100, fuelPct)}%; background: var(--accent);"></i></div>` +
        `<span class="hud-bar-val">${Math.floor(G.fuel)}</span>` +
      `</div>` +
    `</div>` +
    `<span class="hud-chip">⛁ <b>${G.scrap}</b></span>` +
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
  const { x: cx, y: cy, z: realZ } = G.cam;
  const z = Math.min(1.6, 1 + (realZ - 1) * 0.12); // background parallax cap: never goes blank
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
  const { x: cx, y: cy, z: realZ } = G.cam;
  const z = Math.min(1.5, 1 + (realZ - 1) * 0.1); // background cap
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
  // tactical hover: which node is under the cursor (map view only)
  if (!G.scene && !G.camAnim && G.screen === 'play') {
    const w = toWorld(e.clientX, e.clientY);
    let best = 0, bd = (46 / G.cam.z) ** 2;
    for (const n of G.nodes.values()) {
      const d = (n.x - w.x) ** 2 + (n.y - w.y) ** 2;
      if (d < bd) { bd = d; best = n.id; }
    }
    G.hoverId = best;
  } else G.hoverId = 0;
  if (!drag.on || G.scene || G.camAnim) return;
  const dx = e.clientX - drag.sx, dy = e.clientY - drag.sy;
  if (Math.abs(dx) + Math.abs(dy) > 6) drag.moved = true;
  G.cam.x = drag.cx - dx / G.cam.z;
  G.cam.y = drag.cy - dy / G.cam.z;
});
window.addEventListener('pointerup', e => {
  if (!drag.on) return;
  drag.on = false;
  if (drag.moved || G.screen !== 'play' || G.activeEvent || G.scene || G.camAnim || B) {
    // during a fight: click the enemy ship to refocus, click your ship to toggle autofire/fire volley
    if (B && !B.over && G.scene && !drag.moved) {
      const sx = e.clientX, sy = e.clientY;
      const foeA = G.scene.actors.find(a => a.hp === B.enemy);
      if (foeA) {
        const fx = (foeA.x - G.cam.x) * G.cam.z + W / 2, fy = (foeA.y - G.cam.y) * G.cam.z + H / 2;
        if (Math.hypot(sx - fx, sy - fy) < 70) { bTarget(); floatText(foeA.x, foeA.y - 40, `TARGET: ${SYS_LABEL[B.player.target]}`, '#f87171'); return; }
      }
      const me = G.scene.player;
      const mx = (me.x - G.cam.x) * G.cam.z + W / 2, my = (me.y - G.cam.y) * G.cam.z + H / 2;
      if (Math.hypot(sx - mx, sy - my) < 70) { bAuto(); return; }
    }
    return;
  }
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

// ---------- scene: camera flight + procedural node staging ----------
// Click a node -> camera flies in, scene builds at the node, codec talks. No pre-modal.
function flyTo(x, y, z, dur, done) {
  G.camAnim = { x0: G.cam.x, y0: G.cam.y, z0: G.cam.z, x1: x, y1: y, z1: z, t0: performance.now(), dur, done };
}
function stepCamAnim(now) {
  const a = G.camAnim;
  if (!a) return;
  let k = Math.min(1, (now - a.t0) / a.dur);
  k = k * k * k * (k * (k * 6 - 15) + 10); // smootherstep
  G.cam.x = a.x0 + (a.x1 - a.x0) * k;
  G.cam.y = a.y0 + (a.y1 - a.y0) * k;
  G.cam.z = a.z0 + (a.z1 - a.z0) * k;
  if (k >= 1) { G.camAnim = null; if (a.done) a.done(); }
}
let X_FX = x => x, Y_FX = y => y;
function poly(points, fill, stroke) {
  ctx.beginPath();
  ctx.moveTo(points[0], points[1]);
  for (let i = 2; i < points.length; i += 2) ctx.lineTo(points[i], points[i + 1]);
  ctx.closePath();
  if (fill) { ctx.fillStyle = fill; ctx.fill(); }
  if (stroke) { ctx.strokeStyle = stroke; ctx.lineWidth = 1.5; ctx.stroke(); }
}
// Vector ship shapes, drawn facing +x. a: actor, t: seconds.
function drawShipShape(a, t) {
  const c = a.color, dim = a.dead;
  ctx.save();
  ctx.translate(X_FX(a.x), Y_FX(a.y));
  const s0 = G.cam.z * 0.16; // 3: ships scale to the environment, not the screen
  ctx.scale(s0, s0);
  ctx.rotate(a.face || 0);
  ctx.globalAlpha = dim ? 0.35 : 1;
  const flick = 0.7 + 0.3 * Math.sin(t * 30 + a.seed);
  if (a.kind === 'player') {
    poly([20, 0, -12, -11, -5, 0, -12, 11], shade(c, dim), TH.ink);
    poly([8, 0, -4, -4, -4, 4], hexA(c, 0.9));
    ctx.fillStyle = `rgba(125,211,252,${0.5 * flick})`; // engine
    poly([-12, -4, -19 - 4 * flick, 0, -12, 4], `rgba(125,211,252,${0.5 * flick})`);
  } else if (a.kind === 'pirate') {
    poly([18, 0, 5, -13, -6, -6, -15, -11, -11, 0, -15, 11, -6, 6, 5, 13], shade(c, dim), TH.ink);
    poly([4, 0, -6, -3, -6, 3], hexA('#f87171', 0.9));
    ctx.fillStyle = `rgba(248,113,113,${0.5 * flick})`;
    poly([-14, -3, -20 - 3 * flick, 0, -14, 3], `rgba(248,113,113,${0.5 * flick})`);
  } else if (a.kind === 'patrol') {
    poly([20, 0, -13, -9, -6, 0, -13, 9], shade(c, dim), TH.ink);
    ctx.fillStyle = hexA(c, 0.9);
    ctx.fillRect(-2, -2, 8, 4);
    ctx.fillStyle = `rgba(165,180,252,${0.5 * flick})`;
    poly([-13, -3, -18 - 3 * flick, 0, -13, 3], `rgba(165,180,252,${0.5 * flick})`);
  } else if (a.kind === 'merchant') {
    ctx.fillStyle = shade(c, dim);
    ctx.fillRect(-18, -9, 32, 18);
    ctx.strokeStyle = TH.ink; ctx.lineWidth = 1.5; ctx.strokeRect(-18, -9, 32, 18);
    ctx.fillStyle = hexA(c, 0.8);
    ctx.beginPath(); ctx.arc(-22, -10, 6, 0, TAU); ctx.arc(-22, 10, 6, 0, TAU); ctx.fill();
    ctx.fillStyle = `rgba(251,191,36,${0.4 * flick})`;
    ctx.fillRect(-18, -2, -4 * flick - 2, 4);
  } else if (a.kind === 'drone') {
    poly([10, 0, -8, -8, -8, 8], shade(c, dim), TH.ink);
    ctx.fillStyle = hexA('#e879f9', 0.9);
    ctx.beginPath(); ctx.arc(0, 0, 2.5, 0, TAU); ctx.fill();
  } else if (a.kind === 'station') {
    ctx.strokeStyle = hexA(c, dim ? 0.4 : 0.95); ctx.lineWidth = 4;
    ctx.beginPath(); ctx.arc(0, 0, 44, 0, TAU); ctx.stroke();
    ctx.fillStyle = shade(c, dim);
    ctx.fillRect(-12, -12, 24, 24);
    ctx.strokeStyle = TH.ink; ctx.lineWidth = 1.5; ctx.strokeRect(-12, -12, 24, 24);
    for (let i = 0; i < 4; i++) {
      const an = t * 0.5 + a.seed + i * Math.PI / 2;
      ctx.fillStyle = (Math.sin(t * 3 + i * 2) > 0) ? '#fbbf24' : hexA('#fbbf24', 0.2);
      ctx.beginPath(); ctx.arc(Math.cos(an) * 44, Math.sin(an) * 44, 2.5, 0, TAU); ctx.fill();
    }
  } else if (a.kind === 'derelict') {
    ctx.fillStyle = shade('#475569', true);
    ctx.save(); ctx.rotate(0.4); ctx.fillRect(-22, -8, 18, 12); ctx.restore();
    ctx.save(); ctx.rotate(-0.3); ctx.fillRect(4, -4, 20, 10); ctx.restore();
    if (Math.sin(t * 2 + a.seed) > 0.7) {
      ctx.fillStyle = '#fbbf24';
      ctx.fillRect(6 + Math.sin(a.seed * 9) * 8, -6, 2, 2);
    }
  } else if (a.kind === 'anomaly') {
    const r = 26 + 6 * Math.sin(t * 2 + a.seed);
    const g = ctx.createRadialGradient(0, 0, 0, 0, 0, r);
    g.addColorStop(0, 'rgba(192,132,252,0.9)');
    g.addColorStop(0.5, 'rgba(129,140,248,0.35)');
    g.addColorStop(1, 'rgba(0,0,0,0)');
    ctx.fillStyle = g;
    ctx.beginPath(); ctx.arc(0, 0, r, 0, TAU); ctx.fill();
  } else if (a.kind === 'ruins') {
    for (let i = -1; i <= 1; i++) {
      poly([i * 16 - 5, 14, i * 16, -18 - (i * i) * -6, i * 16 + 5, 14], hexA(c, 0.75), TH.ink);
    }
  } else if (a.kind === 'rift') {
    ctx.strokeStyle = hexA('#e879f9', 0.85); ctx.lineWidth = 3;
    ctx.save(); ctx.rotate(Math.sin(t * 0.8 + a.seed) * 0.15);
    ctx.beginPath(); ctx.ellipse(0, 0, 12, 42, 0, 0, TAU); ctx.stroke();
    ctx.strokeStyle = hexA('#e879f9', 0.35);
    ctx.beginPath(); ctx.ellipse(0, 0, 22, 52, 0, 0, TAU); ctx.stroke();
    ctx.restore();
  } else if (a.kind === 'rocks') {
    for (let i = 0; i < 5; i++) {
      const h1 = hash2(a.seed | 0, i, 5), h2 = hash2(a.seed | 0, i, 6);
      const rx = (h1 - 0.5) * 60, ry = (h2 - 0.5) * 50, rr = 3 + h1 * 7;
      poly([rx + rr, ry, rx, ry - rr * 0.7, rx - rr, ry + rr * 0.4], '#334155', '#0f172a');
    }
  }
  ctx.restore();
  // shield bubble + hit flash (fixed screen size)
  if (!dim && a.shieldFlash > 0) {
    ctx.globalAlpha = Math.min(1, a.shieldFlash) * 0.7;
    ctx.strokeStyle = '#7dd3fc'; ctx.lineWidth = 2;
    ctx.beginPath(); ctx.arc(X_FX(a.x), Y_FX(a.y), 42, 0, TAU); ctx.stroke();
    ctx.globalAlpha = 1;
  }
  if (!dim && a.hitFlash > 0) {
    ctx.globalAlpha = Math.min(1, a.hitFlash) * 0.6;
    ctx.fillStyle = '#fff';
    ctx.beginPath(); ctx.arc(X_FX(a.x), Y_FX(a.y), 30, 0, TAU); ctx.fill();
    ctx.globalAlpha = 1;
  }
}
function shade(hex, dim) {
  return dim ? '#1e293b' : hexA(hex, 0.28);
}
// floating hp bar + crew status over a battle-linked actor
function drawActorBar(a) {
  if (!a.hp || a.dead) return;
  const s = a.hp, w = 64;
  const pct = Math.max(0, s.hull / s.maxHull);
  const ax = X_FX(a.x), ay = Y_FX(a.y); // fixed screen offsets, scene-independent
  ctx.fillStyle = 'rgba(2,6,16,0.8)';
  ctx.fillRect(ax - w / 2, ay - 46, w, 7);
  ctx.fillStyle = a.foe ? '#f87171' : '#22d3ee';
  ctx.fillRect(ax - w / 2 + 1, ay - 45, w * pct - 2, 5);
  ctx.fillStyle = '#7dd3fc';
  for (let i = 0; i < s.sh; i++) ctx.fillRect(ax - w / 2 + i * 8, ay - 54, 6, 4);
  // 3: per-weapon charge bars under the hull bar — you SEE them arm, no text decoding
  if (s.weapons && !a.foe) {
    for (let i = 0; i < s.weapons.length; i++) {
      const wp = s.weapons[i];
      const pct = Math.min(1, wp.charge / wp.cd);
      const bw = 38, bx = ax - (s.weapons.length * (bw + 4) - 4) / 2 + i * (bw + 4);
      ctx.fillStyle = 'rgba(148,163,184,0.25)';
      ctx.fillRect(bx, ay - 38, bw, 3);
      ctx.fillStyle = wp.color;
      ctx.fillRect(bx, ay - 38, bw * pct, 3);
    }
  }
  // 4: enemy systems clickable — labels under the foe, current target highlighted
  if (a.foe && window.__sd && window.__sd.battle && !window.__sd.battle.over) {
    const Bx = window.__sd.battle;
    const labels = ['weapons', 'engines', 'shields'];
    ctx.font = '9px sans-serif'; ctx.textAlign = 'center';
    for (let i = 0; i < labels.length; i++) {
      const lx = ax - 36 + i * 36;
      const hot = Bx.player.target === labels[i];
      ctx.fillStyle = hot ? '#f87171' : 'rgba(148,163,184,0.6)';
      ctx.fillText((hot ? '▶' : '') + SYS_LABEL[labels[i]], lx, ay + 52);
      if (hot) { ctx.strokeStyle = '#f87171'; ctx.lineWidth = 1; ctx.beginPath(); ctx.arc(lx, ay + 40, 12, 0, TAU); ctx.stroke(); }
    }
  }
  if (a.crewLine) {
    ctx.fillStyle = 'rgba(148,163,184,0.9)';
    ctx.font = '10px sans-serif'; ctx.textAlign = 'center';
    ctx.fillText(a.crewLine, ax, ay + 42);
  }
}
// crew dots moving inside your hull (visual echo of crewAI stations)
const ROOM_PT = { weapons: [7, 0], engines: [-9, 0], shields: [0, -6], bridge: [0, 6] };
function drawCrewDots(a, t) {
  if (!a.hp || !a.hp.crew || a.dead) return;
  const flip = a.face ? -1 : 1;
  for (let i = 0; i < a.hp.crew.length; i++) {
    const c = a.hp.crew[i];
    const p = ROOM_PT[c.station] || ROOM_PT.bridge;
    c.dx = (c.dx ?? p[0]) + (p[0] - (c.dx ?? p[0])) * 0.06;
    c.dy = (c.dy ?? p[1]) + (p[1] - (c.dy ?? p[1])) * 0.06;
    ctx.fillStyle = TH.acc;
    ctx.beginPath(); ctx.arc(X_FX(a.x) + c.dx * flip * 1.6, Y_FX(a.y) + c.dy * 1.6, 3, 0, TAU); ctx.fill();
  }
}
// fx: shots, bursts, floating text
function spawnShot(a, b, color, size) {
  const S = G.scene;
  if (!S) return;
  S.shots.push({ x1: a.x, y1: a.y, x2: b.x, y2: b.y, t: 0, dur: 0.28, color, size: size || 3 });
}
function burst(x, y, color, n) {
  const S = G.scene;
  if (!S) return;
  for (let i = 0; i < (n || 14); i++) {
    const an = Math.random() * TAU, sp = 40 + Math.random() * 160;
    S.parts.push({ x, y, vx: Math.cos(an) * sp, vy: Math.sin(an) * sp, life: 0.5 + Math.random() * 0.5, t: 0, color, size: 1 + Math.random() * 2.5 });
  }
}
function floatText(x, y, text, color) {
  const S = G.scene;
  if (!S) return;
  S.floats.push({ x, y, text, color: color || '#fff', t: 0, dur: 1.1 });
}
function stepFx(dt) {
  const S = G.scene;
  if (!S) return;
  for (const a of S.actors) {
    a.shieldFlash = Math.max(0, (a.shieldFlash || 0) - dt * 3);
    a.hitFlash = Math.max(0, (a.hitFlash || 0) - dt * 4);
    if (!a.dead && a.orb) {
      a.x = a.ax + Math.cos(performance.now() / 1000 * a.orbSp + a.orbPh) * a.orb;
      a.y = a.ay + Math.sin(performance.now() / 1000 * a.orbSp * 0.8 + a.orbPh) * a.orb * 0.6;
    }
  }
  for (const s of S.shots) s.t += dt;
  S.shots = S.shots.filter(s => s.t < s.dur);
  for (const p of S.parts) { p.t += dt; p.x += p.vx * dt; p.y += p.vy * dt; p.vx *= 0.96; p.vy *= 0.96; }
  S.parts = S.parts.filter(p => p.t < p.life);
  for (const f of S.floats) { f.t += dt; f.y -= 24 * dt; }
  S.floats = S.floats.filter(f => f.t < f.dur);
}
function drawFx() {
  const S = G.scene;
  if (!S) return;
  const t = performance.now() / 1000;
  if (S.player && !S.player.dead) drawShipShape(S.player, t);
  for (const a of S.actors) drawShipShape(a, t);
  if (S.player) drawCrewDots(S.player, t);
  for (const a of S.actors) drawActorBar(a);
  for (const s of S.shots) {
    const k = s.t / s.dur;
    const x = X_FX(s.x1 + (s.x2 - s.x1) * k), y = Y_FX(s.y1 + (s.y2 - s.y1) * k);
    ctx.strokeStyle = s.color; ctx.lineWidth = s.size; ctx.lineCap = 'round';
    ctx.beginPath();
    const x0 = X_FX(s.x1 + (s.x2 - s.x1) * Math.max(0, k - 0.15)), y0 = Y_FX(s.y1 + (s.y2 - s.y1) * Math.max(0, k - 0.15));
    ctx.moveTo(x0, y0);
    ctx.lineTo(x, y);
    ctx.stroke();
    ctx.fillStyle = '#fff';
    ctx.beginPath(); ctx.arc(x, y, s.size / 1.5, 0, TAU); ctx.fill();
  }
  for (const p of S.parts) {
    ctx.globalAlpha = 1 - p.t / p.life;
    ctx.fillStyle = p.color;
    ctx.fillRect(X_FX(p.x), Y_FX(p.y), p.size, p.size);
  }
  ctx.globalAlpha = 1;
  ctx.textAlign = 'center'; ctx.font = 'bold 13px sans-serif';
  for (const f of S.floats) {
    ctx.globalAlpha = 1 - f.t / f.dur;
    ctx.fillStyle = f.color;
    ctx.fillText(f.text, X_FX(f.x), Y_FX(f.y));
  }
  ctx.globalAlpha = 1;
}

// ---------- render ----------
function draw() {
  ctx.clearRect(-60, -60, W + 120, H + 120);
  const t = performance.now() / 1000;
  if (G.shake > 0.05) {
    ctx.save();
    ctx.translate((Math.random() - 0.5) * G.shake, (Math.random() - 0.5) * G.shake);
  }
  drawNebulas(t);
  drawStars(t);
  if (G.scene) { X_FX = wx => (wx - G.cam.x) * G.cam.z + W / 2; Y_FX = wy => (wy - G.cam.y) * G.cam.z + H / 2; }
  if (G.screen === 'menu' || G.faction === null && G.screen !== 'play') return;
  const { x: cx, y: cy, z } = G.cam;
  const X = wx => (wx - cx) * z + W / 2;
  const Y = wy => (wy - cy) * z + H / 2;
  const inScene = !!G.scene; // map UI (labels, dots, links) sleeps during a scene
  // links (frustum-culled: skip when both ends are off-screen)
  if (!inScene)
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
  if (!inScene)
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
  // tactical hover tooltip: danger + expected content before you spend fuel
  if (G.hoverId && !inScene && z > 0.5) {
    const n = G.nodes.get(G.hoverId);
    if (n) {
      const x = X(n.x), y = Y(n.y);
      const st = SECTOR_STYLE[n.type];
      const danger = dangerOf(n);
      const known = n.visited || (cur && cur.links.includes(n.id));
      const w = 170, h = known ? 66 : 40;
      const bx = clamp(x + 26, 8, W - w - 8), by = clamp(y - h - 10, 8, H - h - 8);
      ctx.fillStyle = 'rgba(2,6,16,0.92)';
      ctx.strokeStyle = hexA(st.color, 0.7);
      ctx.lineWidth = 1;
      ctx.beginPath(); ctx.rect(bx, by, w, h); ctx.fill(); ctx.stroke();
      ctx.textAlign = 'left';
      ctx.fillStyle = TH.ink; ctx.font = 'bold 12px system-ui, sans-serif';
      ctx.fillText(n.name, bx + 8, by + 17);
      ctx.fillStyle = st.color; ctx.font = '10px system-ui, sans-serif';
      ctx.fillText(n.type.toUpperCase(), bx + 8, by + 31);
      if (known) {
        ctx.fillStyle = TH.muted; ctx.font = '9px system-ui, sans-serif';
        ctx.fillText('THREAT', bx + 8, by + 49);
        for (let i = 0; i < 5; i++) {
          const on = i < Math.ceil(danger / 2);
          ctx.fillStyle = on ? (danger >= 6 ? '#f87171' : danger >= 4 ? '#fb923c' : '#fbbf24') : 'rgba(148,163,184,0.25)';
          ctx.fillRect(bx + 54 + i * 15, by + 43, 12, 7);
        }
        ctx.fillStyle = TH.faint; ctx.font = '9px system-ui, sans-serif';
        ctx.fillText(n.visited ? 'already charted' : '1 fuel to jump', bx + 8, by + 61);
      } else {
        ctx.fillStyle = TH.muted; ctx.font = '9px system-ui, sans-serif';
        ctx.fillText('2 jumps away — reachable later', bx + 8, by + 48);
      }
      ctx.textAlign = 'center';
    }
  }
  // current pulse
  if (cur && !inScene) {
    const p = (performance.now() / 900) % 1;
    ctx.strokeStyle = hexA(TH.green, 0.6 * (1 - p));
    ctx.lineWidth = 2;
    ctx.beginPath(); ctx.arc(X(cur.x), Y(cur.y), 24 + p * 22, 0, TAU); ctx.stroke();
  }
  // 2: ship visibly rides the link line into the node while the camera dives
  if (G.traveler && G.camAnim) {
    const t0 = G.camAnim.t0, dur = G.camAnim.dur;
    let k = Math.min(1, (performance.now() - t0) / dur);
    k = k * k * k * (k * (k * 6 - 15) + 10);
    const tx = G.traveler.x1 + (G.traveler.x2 - G.traveler.x1) * k;
    const ty = G.traveler.y1 + (G.traveler.y2 - G.traveler.y1) * k;
    const sav = X_FX, savy = Y_FX;
    X_FX = X; Y_FX = Y;
    drawShipShape({ kind: 'player', x: tx, y: ty, face: Math.atan2(G.traveler.dy ?? 0, G.traveler.dx ?? 0) || 0, color: '#22d3ee', seed: 0 }, t);
    // face along travel direction
    X_FX = sav; Y_FX = savy;
  }
  if (G.scene) { const scn = G.nodes.get(G.scene.nodeId); if (scn) drawEnv(scn, t); drawFx(); }
  X_FX = x => x; Y_FX = y => y;
  if (G.shake > 0.05) ctx.restore();
}

let last = 0;
function frame(ts) {
  requestAnimationFrame(frame);
  const dt = Math.min(0.05, (ts - last) / 1000);
  last = ts;
  G.shake = Math.max(0, (G.shake || 0) - dt * 26); // screenshake decay
  stepCamAnim(ts);
  if (G.scene) stepFx(dt);
  draw();
}

// ---------- menu ----------
const fdiv = document.getElementById('factions');
for (const f of FACTIONS) {
  const b = document.createElement('button');
  const guns = playerLoadout(f.id).map(k => WEAPONS[k].name).join(' · ');
  b.innerHTML =
    `<svg class="fship" viewBox="0 0 40 20" aria-hidden="true">` +
      `<polygon points="36,10 12,2 18,10 12,18" fill="${f.color}" fill-opacity="0.28" stroke="${f.color}" stroke-width="1.2"/>` +
      `<polygon points="20,10 12,7 12,13" fill="${f.color}"/>` +
      `<polygon points="6,7 0,10 6,13" fill="${f.color}" fill-opacity="0.5"/>` +
    `</svg>` +
    `<b style="color:${f.color}">${f.name}</b>` +
    `<span>${f.bonus}</span>` +
    `<em class="farms">◂ ${guns}</em>`;
  b.addEventListener('click', () => startRun(f));
  fdiv.appendChild(b);
}
document.getElementById('again').addEventListener('click', () => startRun(G.faction));
window.__sd = { G, travelTo, choose, startRun, startBattle, bTarget, bTalk, bFlee, bPause, bAuto }; // test hook
requestAnimationFrame(frame);
