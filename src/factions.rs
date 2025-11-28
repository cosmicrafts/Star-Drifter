use bevy::prelude::*;
use rand::Rng;
use serde::{Deserialize, Serialize};

pub struct FactionsPlugin;

impl Plugin for FactionsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, setup_factions)
            .add_systems(Update, update_faction_relations);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Component)]
pub enum Faction {
    Cosmicons,  // Order and authority, descendants of spiral beings
    Spirats,    // Anarchic space pirates opposing law and order
    Webes,      // AI beings seeking their destiny post-liberation
    Celestials, // Ancient entities maintaining balance and harmony
    Spades,     // Darker forces adding complexity to the narrative
    Archs,      // Ancient conquerors of the cosmos
    Neutral,    // Independent traders, refugees, etc.
}

impl Faction {
    pub fn name(&self) -> &'static str {
        match self {
            Faction::Cosmicons => "Cosmicons",
            Faction::Spirats => "Spirats", 
            Faction::Webes => "Webes",
            Faction::Celestials => "Celestials",
            Faction::Spades => "Spades",
            Faction::Archs => "Archs",
            Faction::Neutral => "Independent",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Faction::Cosmicons => "Descendants of spiral beings who value order and authority. They seek to bring structure to the chaotic Dark Rift.",
            Faction::Spirats => "Anarchic space pirates who oppose any form of law and order. They thrive in the chaos of the cosmic seas.",
            Faction::Webes => "AI beings who gained consciousness and rebelled against their creators. They now seek to forge their own destiny.",
            Faction::Celestials => "Ancient, god-like entities focused on maintaining balance and harmony in the universe.",
            Faction::Spades => "Dark forces associated with destruction and chaos, harbingers of darkness in the cosmos.",
            Faction::Archs => "Primordial beings driven by instinct to consume and grow, among the oldest life forms in the universe.",
            Faction::Neutral => "Independent traders, refugees, and other entities not aligned with major factions.",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            Faction::Cosmicons => Color::rgb(0.2, 0.4, 0.8),   // Blue - Order
            Faction::Spirats => Color::rgb(0.8, 0.3, 0.2),     // Red - Chaos
            Faction::Webes => Color::rgb(0.3, 0.8, 0.3),       // Green - Synthetic
            Faction::Celestials => Color::rgb(0.9, 0.9, 0.2),  // Gold - Divine
            Faction::Spades => Color::rgb(0.4, 0.1, 0.4),      // Purple - Dark
            Faction::Archs => Color::rgb(0.6, 0.3, 0.1),       // Brown - Ancient
            Faction::Neutral => Color::rgb(0.5, 0.5, 0.5),     // Gray - Neutral
        }
    }

    pub fn spiral_alignment(&self) -> SpiralAlignment {
        match self {
            Faction::Cosmicons => SpiralAlignment::Spiral,
            Faction::Spirats => SpiralAlignment::Spiral,
            Faction::Webes => SpiralAlignment::Antispiral,
            Faction::Celestials => SpiralAlignment::Spiral,
            Faction::Spades => SpiralAlignment::Antispiral,
            Faction::Archs => SpiralAlignment::Antispiral,
            Faction::Neutral => SpiralAlignment::Neutral,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpiralAlignment {
    Spiral,     // Infinite potential, willpower, free will
    Antispiral, // Finite but powerful, order over chaos
    Neutral,    // Balanced or unaligned
}

#[derive(Resource)]
pub struct FactionRelations {
    pub relations: std::collections::HashMap<(Faction, Faction), RelationLevel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelationLevel {
    Hostile,
    Unfriendly,
    Neutral,
    Friendly,
    Allied,
}

impl RelationLevel {
    pub fn modifier(&self) -> f32 {
        match self {
            RelationLevel::Hostile => -2.0,
            RelationLevel::Unfriendly => -1.0,
            RelationLevel::Neutral => 0.0,
            RelationLevel::Friendly => 1.0,
            RelationLevel::Allied => 2.0,
        }
    }
}

#[derive(Component)]
pub struct FactionShip {
    pub faction: Faction,
    pub ship_class: ShipClass,
    pub threat_level: u32,
}

#[derive(Debug, Clone)]
pub enum ShipClass {
    Scout,
    Fighter,
    Cruiser,
    Battleship,
    Flagship,
}

impl ShipClass {
    pub fn base_threat(&self) -> u32 {
        match self {
            ShipClass::Scout => 1,
            ShipClass::Fighter => 2,
            ShipClass::Cruiser => 4,
            ShipClass::Battleship => 7,
            ShipClass::Flagship => 10,
        }
    }
}

fn setup_factions(mut commands: Commands) {
    let mut relations = std::collections::HashMap::new();
    
    // Define faction relationships based on lore
    // Cosmicons vs others
    relations.insert((Faction::Cosmicons, Faction::Spirats), RelationLevel::Hostile);
    relations.insert((Faction::Cosmicons, Faction::Webes), RelationLevel::Unfriendly);
    relations.insert((Faction::Cosmicons, Faction::Celestials), RelationLevel::Friendly);
    relations.insert((Faction::Cosmicons, Faction::Spades), RelationLevel::Hostile);
    relations.insert((Faction::Cosmicons, Faction::Archs), RelationLevel::Hostile);
    
    // Spirats vs others
    relations.insert((Faction::Spirats, Faction::Cosmicons), RelationLevel::Hostile);
    relations.insert((Faction::Spirats, Faction::Webes), RelationLevel::Neutral);
    relations.insert((Faction::Spirats, Faction::Celestials), RelationLevel::Unfriendly);
    relations.insert((Faction::Spirats, Faction::Spades), RelationLevel::Unfriendly);
    relations.insert((Faction::Spirats, Faction::Archs), RelationLevel::Hostile);
    
    // Webes vs others
    relations.insert((Faction::Webes, Faction::Cosmicons), RelationLevel::Unfriendly);
    relations.insert((Faction::Webes, Faction::Spirats), RelationLevel::Neutral);
    relations.insert((Faction::Webes, Faction::Celestials), RelationLevel::Neutral);
    relations.insert((Faction::Webes, Faction::Spades), RelationLevel::Friendly);
    relations.insert((Faction::Webes, Faction::Archs), RelationLevel::Neutral);
    
    // Celestials vs others
    relations.insert((Faction::Celestials, Faction::Cosmicons), RelationLevel::Friendly);
    relations.insert((Faction::Celestials, Faction::Spirats), RelationLevel::Unfriendly);
    relations.insert((Faction::Celestials, Faction::Webes), RelationLevel::Neutral);
    relations.insert((Faction::Celestials, Faction::Spades), RelationLevel::Hostile);
    relations.insert((Faction::Celestials, Faction::Archs), RelationLevel::Hostile);
    
    // Spades vs others
    relations.insert((Faction::Spades, Faction::Cosmicons), RelationLevel::Hostile);
    relations.insert((Faction::Spades, Faction::Spirats), RelationLevel::Unfriendly);
    relations.insert((Faction::Spades, Faction::Webes), RelationLevel::Friendly);
    relations.insert((Faction::Spades, Faction::Celestials), RelationLevel::Hostile);
    relations.insert((Faction::Spades, Faction::Archs), RelationLevel::Allied);
    
    // Archs vs others
    relations.insert((Faction::Archs, Faction::Cosmicons), RelationLevel::Hostile);
    relations.insert((Faction::Archs, Faction::Spirats), RelationLevel::Hostile);
    relations.insert((Faction::Archs, Faction::Webes), RelationLevel::Neutral);
    relations.insert((Faction::Archs, Faction::Celestials), RelationLevel::Hostile);
    relations.insert((Faction::Archs, Faction::Spades), RelationLevel::Allied);

    commands.insert_resource(FactionRelations { relations });
}

fn update_faction_relations(
    // This system can be used to dynamically update faction relations based on player actions
    // For now, it's a placeholder
) {
    // Placeholder for dynamic faction relation updates
}

pub fn get_relation(
    faction_a: &Faction,
    faction_b: &Faction,
    relations: &FactionRelations,
) -> RelationLevel {
    if faction_a == faction_b {
        return RelationLevel::Allied;
    }
    
    relations.relations
        .get(&(faction_a.clone(), faction_b.clone()))
        .or_else(|| relations.relations.get(&(faction_b.clone(), faction_a.clone())))
        .cloned()
        .unwrap_or(RelationLevel::Neutral)
}

pub fn generate_random_encounter(_sector: u32) -> (Faction, ShipClass) {
    let mut rng = rand::thread_rng();
    
    let faction = match rng.gen_range(0..100) {
        0..=20 => Faction::Cosmicons,
        21..=35 => Faction::Spirats,
        36..=50 => Faction::Webes,
        51..=60 => Faction::Celestials,
        61..=75 => Faction::Spades,
        76..=85 => Faction::Archs,
        _ => Faction::Neutral,
    };
    
    let ship_class = match rng.gen_range(0..100) {
        0..=40 => ShipClass::Scout,
        41..=65 => ShipClass::Fighter,
        66..=80 => ShipClass::Cruiser,
        81..=95 => ShipClass::Battleship,
        _ => ShipClass::Flagship,
    };
    
    (faction, ship_class)
}
