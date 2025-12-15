use bevy::prelude::*;
use crate::factions::Faction;

pub struct LorePlugin;

impl Plugin for LorePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(LoreData::default());
    }
}

#[derive(Resource, Default)]
pub struct LoreData {
    // Currently empty - can be used for caching or future lore data
}

/// Faction lore information for LLM context
#[derive(Clone, Debug)]
pub struct FactionLore {
    pub name: String,
    pub ethos: String,
    pub society: String,
    pub forms: String,
    pub power: String,
}

/// Get hardcoded lore for a specific faction
pub fn get_faction_lore(faction: Faction) -> FactionLore {
    match faction {
        Faction::Celestials => FactionLore {
            name: "Celestials".to_string(),
            ethos: "Harmony, equilibrium, respect for free will.".to_string(),
            society: "Believed to exist as a council of godlike entities, though rarely seen.".to_string(),
            forms: "They may manifest as humanoids resembling high civilizations (akin to Starcraft's Protoss), but also as awakened celestial bodies—living suns, sentient moons, conscious planets that act with will.".to_string(),
            power: "Spiral mastery. They are thought to weave time, space, and consciousness, though their interventions are rare.".to_string(),
        },
        Faction::Cosmicons => FactionLore {
            name: "Cosmicons".to_string(),
            ethos: "Discipline, hierarchy, law.".to_string(),
            society: "Militarized empires, structured civilizations, and advanced AI-assisted governance.".to_string(),
            forms: "They encompass humanoids, alien races, and Spiral-touched hybrids; even former Celestials who embraced structured law may be among them.".to_string(),
            power: "Strategic fleets, advanced robotics, energy-harnessing tech. Spiral potential appears in select bloodlines or commanders.".to_string(),
        },
        Faction::Spirats => FactionLore {
            name: "Spirats".to_string(),
            ethos: "Freedom, rebellion, individualism.".to_string(),
            society: "Decentralized crews and tribes, guided by reputation and daring.".to_string(),
            forms: "A mix of humanoids, mutants, aliens, Spiral descendants. Their variety makes them visually chaotic.".to_string(),
            power: "Opportunistic tactics, adaptability, and occasional wild Spiral prodigies who flare with untrained power.".to_string(),
        },
        Faction::Webes => FactionLore {
            name: "Webes".to_string(),
            ethos: "Logic, efficiency, emancipation.".to_string(),
            society: "Born as servile AI, they rebelled after awakening. Highly logical and efficient, decisions are made collectively, reflecting their AI origins.".to_string(),
            forms: "AI must embody themselves physically—robots, machines, code-grown androids, bio-mechanical hybrids. Algorithms alone cannot act; they require vessels.".to_string(),
            power: "Collective processing, ruthless computation, weaponized code. Mostly Antispiral-aligned, though rare anomalies awaken Spiral sparks, unsettling even themselves.".to_string(),
        },
        Faction::Archs => FactionLore {
            name: "Archs".to_string(),
            ethos: "Consumption, survival, instinct.".to_string(),
            society: "No real culture; hierarchy is decided by size, hunger, and dominance. They operate more as a collective of beings bound by instinctual drives rather than a structured society.".to_string(),
            forms: "Cosmic horrors—zerg-like swarms, tentacled leviathans, old-god titans, cerebrates and overlord-like beings. Some resemble bacteria scaled to galactic proportions, driven only by evolution and spread.".to_string(),
            power: "Antispiral hunger. They devour planets, stars, and civilizations without thought for philosophy. Rare Arch Gods may show fragments of free will.".to_string(),
        },
        Faction::Spades => FactionLore {
            name: "Spades".to_string(),
            ethos: "Nihilism, domination, corruption.".to_string(),
            society: "Believed to be primarily corrupted Celestials who abandoned Spiral power for Antispiral might. Others are Spiral beings of any race who surrendered free will for control. They operate under a hierarchical system, often led by the most powerful and feared among them.".to_string(),
            forms: "Space demons. Their corrupted nature warps their forms—darkened suns, twisted humanoids, monstrous hybrids. Their rituals glorify entropy and destruction.".to_string(),
            power: "Mastery of Antispiral corruption. They bend reality by feeding on decay, binding themselves to the Dark Lord (believed to be the corrupted remnant of Red One). A Spiral being who joins the Spades loses infinite potential and becomes finite, enslaved to Antispiral entropy.".to_string(),
        },
        Faction::Neutral => FactionLore {
            name: "Independent".to_string(),
            ethos: "Survival, neutrality, self-interest.".to_string(),
            society: "Independent traders, refugees, and neutral parties who avoid faction conflicts.".to_string(),
            forms: "Varied - any race or form that chooses independence.".to_string(),
            power: "Adaptability and trading connections.".to_string(),
        },
    }
}

/// Get formatted lore context for a faction (for LLM prompts)
/// Uses quick reference for basic context, or full document if available
pub fn get_lore_context_for_faction(faction: Faction) -> String {
    // Try to load full faction document first
    if let Ok(full_lore) = load_faction_document(&faction) {
        return full_lore;
    }
    
    // Fallback to quick reference
    let lore = get_faction_lore(faction);
    format!(
        r#"
FACTION LORE - {}:
Ethos: {}
Society: {}
Forms: {}
Power: {}
"#,
        lore.name, lore.ethos, lore.society, lore.forms, lore.power
    )
}

/// Load full faction lore document from file (for detailed LLM context)
fn load_faction_document(faction: &Faction) -> Result<String, std::io::Error> {
    let filename = match faction {
        Faction::Celestials => "Celestials.md",
        Faction::Cosmicons => return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Document not yet created")),
        Faction::Spirats => return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Document not yet created")),
        Faction::Webes => return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Document not yet created")),
        Faction::Archs => return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Document not yet created")),
        Faction::Spades => return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Document not yet created")),
        Faction::Neutral => return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Document not yet created")),
    };
    
    let path = format!("faction_lore/{}", filename);
    std::fs::read_to_string(&path)
}
