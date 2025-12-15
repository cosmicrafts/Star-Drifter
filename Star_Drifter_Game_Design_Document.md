# Star Drifter - Game Design Document

## Game Overview

**Genre:** Space Exploration / Procedural Narrative / Faction-Based Roguelike

**Setting:** The Dark Rift, during the Violet Eon (Third Cosmic War era)

**Core Concept:** Drift through space, exploring procedurally generated sectors controlled by various factions. Use LLM-powered generation to create unique sectors, bosses, and narrative events based on Cosmicrafts lore. Control sectors by defeating Big Bosses, expanding your faction's influence across the Dark Rift.

---

## Core Gameplay Loop

1. **Choose Faction** → Determines playstyle, objectives, and procedural generation
2. **Explore Sectors** → Travel between nodes, encounter LLM-generated events
3. **Gather Resources** → Collect Aetherium (primary resource), manage Hull
4. **Defeat Bosses** → Progress through sector hierarchy (Bosses → Big Boss)
5. **Control Sectors** → Your faction takes control, affects future generation
6. **Expand** → Use controlled sectors and stats to influence next sector generation

---

## Faction System

### Faction Selection
Player chooses one of 6 factions at game start:
- **Celestials** - Guardians of Balance
- **Cosmicons** - Order and Authority  
- **Spirats** - Raiders of the Cosmic Seas
- **Webes** - Synthetic Seekers
- **Archs** - Primordial Devourers
- **Spades** - The Corrupted Ones

### Faction-Specific Elements

**Unique Objectives:**
- **Arch:** Consume everything, control through consumption
- **Cosmicon:** Establish order, hierarchical control
- **Spirats:** Freedom, chaos, opportunistic control
- **Webes:** Logic-based expansion, computational efficiency
- **Celestials:** Balance, harmony, selective intervention
- **Spades:** Corruption, domination, destructive control

**Unique Mechanics:**
- Each faction has faction-specific systems and bonuses
- Faction affects LLM generation context (narrative tone, available choices)
- Faction determines how sectors are controlled (consumption vs order vs chaos)

**Achievements:**
- Faction-specific achievement systems
- Unlock faction abilities and narrative options
- Track faction-specific progress and milestones

---

## Sector & Node Structure

### Sector Hierarchy

```
Sector (Region)
  └─ Big Boss (Faction Leader)
      └─ Bosses (2-4 Sub-leaders)
          └─ Captains/Tribes (3-5 per Boss)
              └─ Nodes (Individual locations)
```

### Sector Generation

**Timing:**
- **Pre-generate:** First 2-3 sectors at game start
- **On-demand:** Generate additional sectors as player explores
- **LLM-powered:** Each sector is uniquely generated based on:
  - Faction lore
  - Player stats (Aetherium, Hull, controlled sectors)
  - Previous sectors (Big Boss names, faction relationships, sector history)
  - Player faction and objectives
  - Distance from starting point
  - Recent events and choices

**Sector Contents (LLM-generated):**
- Sector name
- Controlling faction
- Big Boss (name, title, faction, personality)
- Bosses (2-4, each with name, title, boss they serve, faction)
- Captains/Tribes (3-5 per boss, controlling specific nodes)
- Initial node structure and connections

### Node System

**Node Generation:**
- **Expansion:** Always random (even starting sector, not perfect circle)
- **Connections:** 2-4 connection slots per node
- **Distance:** Random between nodes (not fixed)
- **Procedural:** Nodes generated as player explores

**Node Purposes:**
All nodes can have LLM-generated events, but nodes have different primary purposes:
- **Event Nodes:** Narrative encounters (LLM-generated)
- **Resource Nodes:** Gather Aetherium
- **Trading Nodes:** Exchange resources, interact with factions
- **Faction Base Nodes:** Interact with faction members, get missions
- **Combat Nodes:** Direct battles/encounters
- **Boss Encounter Nodes:** Face sector bosses

**Node Interaction:**
- LLM generates unique events for each node
- Events reflect node purpose and sector context
- Choices in events affect faction relationships, resources, progression

---

## Big Boss & Boss System

### Boss Discovery Mechanic

**Parallel Progression:**
- Defeat any boss → They reveal location of another boss
- Not linear - can find bosses in any order
- Defeat all bosses in sector → Reveals Big Boss location
- Nodes around bosses remain random and procedural

**Big Boss Defeat:**
- **Narrative Influence:** Not direct combat, but narrative progression
- Player influences sector enough through choices/actions
- Completing objectives, defeating bosses, making choices
- When influence threshold reached → Your faction takes control

**Sector Control:**
- Defeating Big Boss = Your faction controls the sector
- Affects future LLM generation (your faction presence)
- Visualized on map (sector color changes to your faction)
- Stats tracked for LLM context

---

## Resource System

### Aetherium (Primary Resource)

**Usage:**
- **Fuel:** Convert to travel between nodes/sectors
- **Trading:** Exchange for items, services, information
- **Repairs:** Fix Hull damage
- **General:** Primary currency for all activities

**Acquisition:**
- Resource nodes
- Event rewards (LLM-generated)
- Trading
- Defeating enemies/bosses
- Sector control bonuses

### Hull

**Mechanics:**
- Represents ship health/condition
- Can be damaged in combat/events
- **Auto-repair:** Automatic repair mechanisms (unless fleeing from battle)
- Prevents traveling with damaged hull (unless fleeing)

### Removed Resources
- **Scrap:** Not necessary (removed from system)

---

## LLM Integration

### Sector Generation Prompt

**Context Provided to LLM:**
- Faction lore (from Cosmicrafts lore document)
- Current player stats (Aetherium, Hull, controlled sectors count)
- Previous sectors information:
  - Big Boss names and titles
  - Faction relationships
  - Sector history and events
- Player faction and objectives
- Distance from starting point
- Recent events and player choices
- Game progression hints ("first sector = easier", "second sector = harder")

**LLM Generates:**
- Sector name and description
- Big Boss (name, title, faction, personality, backstory)
- Bosses (2-4, with names, titles, relationships, personalities)
- Captains/Tribes (names, controlling nodes, faction alignment)
- Node descriptions and initial events
- Faction relationships and dynamics within sector

### Event Generation Prompt

**Context Provided:**
- Sector information (name, controlling faction, Big Boss)
- Node information (purpose, controlling captain/tribe)
- Player stats (Aetherium, Hull, controlled sectors)
- Distance from start
- Recent event titles (last 5)
- Total events encountered
- Player faction and objectives

**LLM Generates:**
- Unique multi-stage events (1-3 stages)
- Narrative choices with outcomes
- Resource rewards (Aetherium, Hull changes)
- Faction relationship impacts
- Boss/Big Boss hints or encounters

---

## Map & Progression

### Map Visualization

**Display:**
- Current sector and nodes
- Controlled sectors (highlighted by player faction color)
- Connections between nodes
- Visited vs unvisited nodes
- Boss locations (when discovered)
- Big Boss location (when all bosses defeated)

**Progress Tracking:**
- Sectors controlled count
- Current Aetherium
- Hull status
- Distance traveled
- Faction relationships
- Achievements unlocked

### Progression System

**Stats Influence Generation:**
- Controlled sectors → Affects difficulty and faction presence
- Aetherium amount → Affects available choices and events
- Hull status → Affects combat/risk events
- Distance traveled → Affects sector difficulty and rarity
- Faction relationships → Affects encounters and alliances

**Tutorial System:**
- **Option 1:** Neutral starting sector (safe, teaches mechanics)
- **Option 2:** Civilization VI style - advice/tips as you progress
- Player can choose tutorial mode or skip

---

## Technical Implementation Notes

### Current System Changes Needed

**Remove:**
- Danger system (`danger_level`, `base_danger()`, `calculate_danger_level()`)
- Hardcoded sector type descriptions
- Hardcoded sector name generation
- Hardcoded event generation
- Scrap resource
- Fixed distance calculations

**Add:**
- Aetherium resource system
- Hull auto-repair system
- Faction selection at start
- Sector hierarchy (Big Boss → Bosses → Captains → Nodes)
- LLM sector generation system
- Boss discovery/reveal system
- Sector control tracking
- Faction-specific mechanics
- Achievement system

**Modify:**
- Node generation: Random expansion, random distances
- Resource system: Aetherium primary, Hull secondary
- Event system: LLM-powered, faction-aware
- Map system: Show controlled sectors, faction colors

---

## Lore Integration

### Core Lore Elements

**Era:** Violet Eon (Third Cosmic War, universe contracting into Dark Rift)

**Key Concepts:**
- Spiral Force vs Antispiral Force
- Dark Rift as epicenter of cosmic events
- Faction dynamics and relationships
- Aetherium as rare, powerful resource
- Celestial Canon (Eons, Epochs, Cycles)
- Ethereum/Nethereum realms (metaphysical planes)

**Faction Lore:**
- Each faction has detailed lore (ethos, society, forms, power)
- Faction relationships and alignments (Spiral vs Antispiral)
- Historical context (Eons, wars, events)

**LLM Uses Lore For:**
- Generating faction-appropriate content
- Creating lore-consistent narratives
- Maintaining faction relationships
- Referencing cosmic events and history
- Creating unique but lore-faithful experiences

---

## Game Flow Example

1. **Start:** Player chooses Cosmicon faction
2. **Sector 0 Generation:** LLM generates neutral/starting sector
   - Creates sector name, initial nodes
   - No Big Boss (or friendly Big Boss)
   - Tutorial events if enabled
3. **Exploration:** Player travels between nodes
   - Encounters LLM-generated events
   - Gathers Aetherium
   - Manages Hull (auto-repairs)
4. **Sector 1 Generation:** LLM generates first real sector
   - Controlled by Spirats faction
   - Big Boss: "Captain Razor" (Spirat pirate lord)
   - Bosses: 3 subordinate captains
   - Nodes controlled by various Spirat crews
5. **Boss Discovery:** Player defeats first boss
   - Boss reveals location of second boss
   - Player continues exploring nodes
   - Defeats second boss → reveals third boss
   - Defeats all bosses → reveals Big Boss location
6. **Sector Control:** Player influences sector enough
   - Cosmicon faction takes control
   - Sector changes color on map
   - Stats updated for next LLM generation
7. **Sector 2 Generation:** LLM generates next sector
   - Uses context: Player is Cosmicon, has 1 controlled sector
   - Generates appropriate challenge and narrative
   - May include Cosmicon presence (rebels or allies)
   - Continues progression...

---

## Design Principles

1. **Lore-First:** All generation respects and builds on Cosmicrafts lore
2. **Uniqueness:** Every sector, boss, and event is LLM-generated and unique
3. **Progression:** Player choices and stats influence future generation
4. **Faction Identity:** Each faction feels distinct in mechanics and narrative
5. **Exploration:** Random, procedural expansion keeps discovery fresh
6. **Narrative Depth:** LLM creates rich, multi-stage events with meaningful choices

---

## Open Questions / Future Considerations

- Specific faction mechanics details (Arch consumption, Cosmicon order, etc.)
- Combat system (if any) - turn-based, narrative, or abstracted?
- Trading system details
- Achievement system specifics
- End game conditions (control X sectors? Reach Dark Rift center?)
- Save/load system for procedural generation consistency
- Difficulty scaling formula (how does distance affect generation?)

