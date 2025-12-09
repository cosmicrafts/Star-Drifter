use bevy::prelude::*;
use rand::Rng;
use std::collections::HashMap;
use crate::factions::{Faction, generate_random_encounter};
use crate::events;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct NavigationSystemSet;

pub struct SectorPlugin;

impl Plugin for SectorPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(PendingEventTrigger::default())
            .add_systems(Startup, setup_sector_map)
            .configure_sets(Update, NavigationSystemSet.after(crate::events::EventSystemSet))
            .add_systems(Update, (
                handle_sector_navigation,
                trigger_events_after_navigation,
            ).in_set(NavigationSystemSet));
    }
}

#[derive(Resource)]
pub struct SectorMap {
    pub current_sector_id: u32,
    pub sectors: HashMap<u32, Sector>,
    pub distance_traveled: u32, // For scaling difficulty
    pub next_node_id: u32, // Track next available node ID
}

#[derive(Clone)]
pub struct Sector {
    pub _id: u32,
    pub sector_type: SectorType,
    pub name: String,
    pub description: String,
    pub connections: Vec<u32>, // IDs of connected sectors
    pub visited: bool,
    pub events: Vec<SectorEvent>,
    pub danger_level: u32,
    pub position: Vec2, // Position in world space (stored in sector for consistency)
}

#[derive(Clone, Debug)]
pub enum SectorType {
    Empty,          // Nothing of interest
    Nebula,         // Reduced sensors, possible hiding spots
    AsteroidField,  // Mining opportunities, navigation hazards
    Station,        // Trading, repairs, crew
    Distress,       // Ship in trouble
    Combat,         // Enemy encounter
    Anomaly,        // Strange cosmic phenomena
    DarkRift,       // Dangerous but rewarding areas
    CelestialSite,  // Ancient Celestial artifacts
    AetheriumField, // Rare Aetherium deposits
}

impl SectorType {
    pub fn description(&self) -> &'static str {
        match self {
            SectorType::Empty => "Empty space with nothing of particular interest.",
            SectorType::Nebula => "A colorful nebula that interferes with sensors but provides cover.",
            SectorType::AsteroidField => "A dense field of asteroids rich in minerals.",
            SectorType::Station => "A space station offering services to travelers.",
            SectorType::Distress => "A distress beacon emanates from this location.",
            SectorType::Combat => "Hostile ships patrol this area.",
            SectorType::Anomaly => "Strange energy readings suggest something unusual here.",
            SectorType::DarkRift => "A fragment of the mysterious Dark Rift - dangerous but potentially rewarding.",
            SectorType::CelestialSite => "Ancient ruins left by the Celestials, humming with residual power.",
            SectorType::AetheriumField => "Rare Aetherium crystals float in the cosmic void here.",
        }
    }

    pub fn base_danger(&self) -> u32 {
        match self {
            SectorType::Empty => 0,
            SectorType::Nebula => 1,
            SectorType::AsteroidField => 2,
            SectorType::Station => 0,
            SectorType::Distress => 3,
            SectorType::Combat => 5,
            SectorType::Anomaly => 4,
            SectorType::DarkRift => 8,
            SectorType::CelestialSite => 6,
            SectorType::AetheriumField => 7,
        }
    }
}

#[derive(Clone)]
pub struct SectorEvent {
    pub event_type: EventType,
    pub description: String,
    pub faction: Option<Faction>,
    pub _triggered: bool,
}

#[derive(Clone)]
pub enum EventType {
    Encounter,
    Discovery,
    Hazard,
    Opportunity,
    Story,
}


// Determine sector number procedurally based on node ID
// Each sector contains approximately 10 nodes
fn get_sector_number(node_id: u32) -> u32 {
    node_id / 10
}

fn setup_sector_map(mut commands: Commands) {
    let mut sectors = HashMap::new();
    let mut rng = rand::thread_rng();
    
    // Start with a single starting node at origin
    let starting_node_id = 0u32;
    let starting_sector = generate_sector(
        starting_node_id,
        SectorType::Station,
        &mut rng,
        0,
        Vec2::ZERO, // Start at origin
    );
    sectors.insert(starting_node_id, starting_sector);
    
    // Generate 3-5 initial nodes around the starting node (radial distribution for initial setup)
    let initial_nodes = rng.gen_range(3..=5);
    let mut next_id = 1u32;
    const INITIAL_DISTANCE: f32 = 150.0;
    let angle_step = (std::f32::consts::PI * 2.0) / initial_nodes as f32;
    
    for i in 0..initial_nodes {
        let angle = i as f32 * angle_step;
        let pos = Vec2::new(angle.cos() * INITIAL_DISTANCE, angle.sin() * INITIAL_DISTANCE);
        
        let distance = 1;
        let sector_type = generate_random_sector_type(&mut rng, distance);
        let mut new_node = generate_sector(next_id, sector_type, &mut rng, distance, pos);
        
        // Connect bidirectionally to starting node
        if let Some(starting) = sectors.get_mut(&starting_node_id) {
            starting.connections.push(next_id);
        }
        new_node.connections.push(starting_node_id);
        
        sectors.insert(next_id, new_node);
        next_id += 1;
    }
    
    commands.insert_resource(SectorMap {
        current_sector_id: starting_node_id,
        sectors,
        distance_traveled: 0,
        next_node_id: next_id,
    });
}


fn generate_sector(
    id: u32,
    sector_type: SectorType,
    rng: &mut rand::rngs::ThreadRng,
    distance: u32,
    position: Vec2,
) -> Sector {
    let name = generate_sector_name(&sector_type, id);
    let description = sector_type.description().to_string();
    let events = generate_sector_events(&sector_type, rng);
    let danger_level = calculate_danger_level(distance, &sector_type);
    
    // Connections will be added during map generation, not here
    let connections = Vec::new();
    
    Sector {
        _id: id,
        sector_type,
        name,
        description,
        connections,
        visited: false,
        events,
        danger_level,
        position,
    }
}


fn generate_random_sector_type(rng: &mut rand::rngs::ThreadRng, distance: u32) -> SectorType {
    // Scale rarity with distance traveled
    let distance_factor = (distance as f32 / 10.0).min(5.0); // Cap at 5x
    
    match rng.gen_range(0..100) {
        0..=25 => SectorType::Empty,
        26..=40 => SectorType::Nebula,
        41..=55 => SectorType::AsteroidField,
        56..=65 => SectorType::Station,
        66..=75 => SectorType::Distress,
        76..=85 => SectorType::Combat,
        86..=90 => SectorType::Anomaly,
        91..=95 => {
            // Dark Rift becomes more common as distance increases
            if distance_factor > 2.0 && rng.gen_bool(0.3) {
                SectorType::DarkRift
            } else {
                SectorType::Anomaly
            }
        }
        96..=98 => {
            // Celestial Sites appear more often at higher distances
            if distance_factor > 1.0 && rng.gen_bool(0.4) {
                SectorType::CelestialSite
            } else {
                SectorType::Station
            }
        }
        _ => {
            // Aetherium Fields are rare but scale with distance
            if distance_factor > 3.0 && rng.gen_bool(0.2) {
                SectorType::AetheriumField
            } else {
                SectorType::AsteroidField
            }
        }
    }
}

fn generate_sector_name(sector_type: &SectorType, _id: u32) -> String {
    let prefixes = match sector_type {
        SectorType::Empty => vec!["Void", "Silent", "Barren", "Hollow"],
        SectorType::Nebula => vec!["Crimson", "Azure", "Stellar", "Mystic"],
        SectorType::AsteroidField => vec!["Shattered", "Broken", "Drifting", "Ancient"],
        SectorType::Station => vec!["Haven", "Refuge", "Outpost", "Trading"],
        SectorType::Distress => vec!["Lost", "Abandoned", "Forgotten", "Derelict"],
        SectorType::Combat => vec!["Contested", "Hostile", "War-torn", "Dangerous"],
        SectorType::Anomaly => vec!["Strange", "Twisted", "Anomalous", "Warped"],
        SectorType::DarkRift => vec!["Dark", "Void", "Abyssal", "Shadow"],
        SectorType::CelestialSite => vec!["Sacred", "Ancient", "Divine", "Eternal"],
        SectorType::AetheriumField => vec!["Gleaming", "Radiant", "Precious", "Crystalline"],
    };

    let suffixes = match sector_type {
        SectorType::Empty => vec!["Expanse", "Reach", "Void", "Zone"],
        SectorType::Nebula => vec!["Nebula", "Cloud", "Mist", "Veil"],
        SectorType::AsteroidField => vec!["Field", "Belt", "Cluster", "Debris"],
        SectorType::Station => vec!["Station", "Port", "Hub", "Dock"],
        SectorType::Distress => vec!["Wreck", "Hulk", "Grave", "Ruin"],
        SectorType::Combat => vec!["Battleground", "Warzone", "Sector", "Front"],
        SectorType::Anomaly => vec!["Anomaly", "Phenomenon", "Distortion", "Rift"],
        SectorType::DarkRift => vec!["Rift", "Chasm", "Abyss", "Maw"],
        SectorType::CelestialSite => vec!["Shrine", "Temple", "Sanctum", "Monument"],
        SectorType::AetheriumField => vec!["Mines", "Crystals", "Deposits", "Veins"],
    };

    let mut rng = rand::thread_rng();
    let prefix = prefixes[rng.gen_range(0..prefixes.len())];
    let suffix = suffixes[rng.gen_range(0..suffixes.len())];
    
    format!("{} {}", prefix, suffix)
}

fn generate_sector_events(sector_type: &SectorType, rng: &mut rand::rngs::ThreadRng) -> Vec<SectorEvent> {
    let mut events = Vec::new();
    
    match sector_type {
        SectorType::Combat => {
            let (faction, ship_class) = generate_random_encounter(0);
            events.push(SectorEvent {
                event_type: EventType::Encounter,
                description: format!("A {} {} ship blocks your path!", faction.name(), format!("{:?}", ship_class)),
                faction: Some(faction),
                _triggered: false,
            });
        }
        SectorType::Distress => {
            if rng.gen_bool(0.7) {
                events.push(SectorEvent {
                    event_type: EventType::Opportunity,
                    description: "A damaged ship requests assistance.".to_string(),
                    faction: None,
                    _triggered: false,
                });
            } else {
                events.push(SectorEvent {
                    event_type: EventType::Hazard,
                    description: "The distress signal is a trap!".to_string(),
                    faction: Some(Faction::Spirats),
                    _triggered: false,
                });
            }
        }
        SectorType::AetheriumField => {
            events.push(SectorEvent {
                event_type: EventType::Discovery,
                description: "Rare Aetherium crystals detected! Mining could be profitable but dangerous.".to_string(),
                faction: None,
                _triggered: false,
            });
        }
        SectorType::CelestialSite => {
            events.push(SectorEvent {
                event_type: EventType::Story,
                description: "Ancient Celestial ruins pulse with mysterious energy.".to_string(),
                faction: Some(Faction::Celestials),
                _triggered: false,
            });
        }
        _ => {
            // Random chance for events in other sectors
            if rng.gen_bool(0.3) {
                let (faction, _) = generate_random_encounter(0);
                events.push(SectorEvent {
                    event_type: EventType::Encounter,
                    description: format!("You encounter a {} patrol.", faction.name()),
                    faction: Some(faction),
                    _triggered: false,
                });
            }
        }
    }
    
    events
}

fn calculate_danger_level(distance: u32, sector_type: &SectorType) -> u32 {
    let base = sector_type.base_danger();
    let distance_bonus = distance / 5; // Every 5 sectors increases danger
    base + distance_bonus
}

fn handle_sector_navigation(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut sector_map: ResMut<SectorMap>,
    mut game_data: ResMut<crate::game::GameData>,
    mut event_writer: MessageWriter<crate::events::GameEvent>,
    mut active_event: ResMut<crate::events::ActiveEvent>,
    input_consumed: Res<crate::events::InputConsumed>,
    pending_trigger: ResMut<PendingEventTrigger>,
) {
    // Don't allow navigation if an event is currently active
    // Numbers should only be used for event choices when an event is active
    if active_event.event.is_some() {
        return;
    }
    
    // Improved: More responsive keyboard input handling
    if let Some(current_sector) = sector_map.sectors.get(&sector_map.current_sector_id) {
        let connections = current_sector.connections.clone();
        
        // Handle navigation to all connected nodes using number keys 1-9
        // Improved: Use array for cleaner key mapping
        let key_map = [
            KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3,
            KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6,
            KeyCode::Digit7, KeyCode::Digit8, KeyCode::Digit9,
        ];
        
        // All connections are valid - non-existent nodes will be generated on-demand
        for (i, &target_id) in connections.iter().enumerate() {
            if i >= key_map.len() {
                break;
            }
            
            let key = key_map[i];
            
            // Skip if this key was already consumed by event system
            if input_consumed.keys.contains(&key) {
                continue;
            }
            
            // Improved: Use just_pressed for immediate, responsive input
            if keyboard.just_pressed(key) {
                try_travel_to_sector(
                    &mut sector_map,
                    &mut game_data,
                    target_id,
                    &mut event_writer,
                    &mut active_event,
                    Some(pending_trigger),
                );
                break; // Only process first matching key press
            }
        }
    }
}

// Generate new nodes around a node when traveling to it
// New nodes are generated in directions AWAY from the source node (outward expansion like neural connections)
fn generate_nodes_around(
    sector_map: &mut SectorMap,
    node_id: u32,
    source_node_id: Option<u32>,
    rng: &mut rand::rngs::ThreadRng,
    distance: u32,
) {
    const BASE_DISTANCE: f32 = 300.0;
    const MIN_SEPARATION: f32 = 120.0; // Minimum distance between nodes
    const CONNECTION_RANGE: f32 = BASE_DISTANCE * 1.8; // Range to search for existing nodes to connect
    
    // Check if node exists
    let current_pos = if let Some(node) = sector_map.sectors.get(&node_id) {
        node.position
    } else {
        return;
    };
    
    // Get current node's existing connections to avoid reconnecting
    let existing_connections: std::collections::HashSet<u32> = sector_map.sectors
        .get(&node_id)
        .map(|s| s.connections.iter().copied().collect())
        .unwrap_or_default();
    
    // Calculate "forward" direction (away from source)
    let base_angle = if let Some(source_id) = source_node_id {
        if let Some(source_node) = sector_map.sectors.get(&source_id) {
            let direction = current_pos - source_node.position;
            if direction.length_squared() > 0.0001 {
                direction.y.atan2(direction.x) // Direction from source to current
            } else {
                // Too close, use deterministic hash
                let h = hash_sector_id(node_id);
                hash_to_float(h) * 2.0 * std::f32::consts::PI
            }
        } else {
            let h = hash_sector_id(node_id);
            hash_to_float(h) * 2.0 * std::f32::consts::PI
        }
    } else {
        // No source (starting node) - use deterministic hash
        let h = hash_sector_id(node_id);
        hash_to_float(h) * 2.0 * std::f32::consts::PI
    };
    
    // Generate 2-4 connection slots in a forward cone (120 degrees)
    let connection_slots = rng.gen_range(2..=4);
    let arc_span = std::f32::consts::PI * 2.0 / 3.0; // 120 degrees
    let angle_step = if connection_slots > 1 {
        arc_span / (connection_slots as f32 - 1.0)
    } else {
        0.0
    };
    
    // Track which existing nodes we've already considered for connection
    let mut used_existing_nodes = std::collections::HashSet::new();
    
    for i in 0..connection_slots {
        // Calculate angle within the forward cone
        let offset_from_center = -arc_span / 2.0 + angle_step * (i as f32);
        let jitter = (rng.gen::<f32>() - 0.5) * 0.3; // Small random variation
        let angle = base_angle + offset_from_center + jitter;
        
        // Calculate ideal position for this connection
        let dir_vec = Vec2::new(angle.cos(), angle.sin());
        let ideal_pos = current_pos + dir_vec * BASE_DISTANCE;
        
        // First, try to find an existing node nearby to connect to
        let mut found_existing = false;
        let mut best_existing_id = None;
        let mut best_distance = CONNECTION_RANGE;
        
        for (other_id, other_sector) in sector_map.sectors.iter() {
            // Skip self, source, and already connected nodes
            if *other_id == node_id 
                || source_node_id == Some(*other_id)
                || existing_connections.contains(other_id)
                || used_existing_nodes.contains(other_id) {
                continue;
            }
            
            let other_pos = other_sector.position;
            let distance_to_ideal = other_pos.distance(ideal_pos);
            let distance_to_current = other_pos.distance(current_pos);
            
            // Check if this node is in a reasonable range and direction
            if distance_to_current <= CONNECTION_RANGE 
                && distance_to_current >= MIN_SEPARATION
                && distance_to_ideal < best_distance {
                // Verify it's in roughly the right direction (within 60 degrees of ideal)
                let to_other = (other_pos - current_pos).normalize();
                let ideal_dir = dir_vec;
                let dot_product = to_other.dot(ideal_dir);
                if dot_product > 0.5 { // ~60 degrees
                    best_existing_id = Some(*other_id);
                    best_distance = distance_to_ideal;
                    found_existing = true;
                }
            }
        }
        
        if found_existing {
            // Connect to existing node
            if let Some(existing_id) = best_existing_id {
                used_existing_nodes.insert(existing_id);
                
                // Add bidirectional connection
                if let Some(current) = sector_map.sectors.get_mut(&node_id) {
                    if !current.connections.contains(&existing_id) {
                        current.connections.push(existing_id);
                    }
                }
                if let Some(existing) = sector_map.sectors.get_mut(&existing_id) {
                    if !existing.connections.contains(&node_id) {
                        existing.connections.push(node_id);
                    }
                }
            }
        } else {
            // No existing node found, create a new one
            let new_node_id = sector_map.next_node_id;
            sector_map.next_node_id += 1;
            
            // Try to find a position that doesn't collide with existing nodes
            let mut radius = BASE_DISTANCE;
            let mut candidate_pos = current_pos + dir_vec * radius;
            
            // Check for collisions and adjust radius if needed
            loop {
                let mut too_close = false;
                for (other_id, other_sector) in sector_map.sectors.iter() {
                    if *other_id == node_id {
                        continue;
                    }
                    if other_sector.position.distance(candidate_pos) < MIN_SEPARATION {
                        too_close = true;
                        break;
                    }
                }
                
                if !too_close {
                    break;
                }
                
                radius += 50.0;
                if radius > BASE_DISTANCE * 3.0 {
                    // Give up, use this position even if close
                    break;
                }
                candidate_pos = current_pos + dir_vec * radius;
            }
            
            // Create new node with calculated position
            let sector_type = generate_random_sector_type(rng, distance);
            let mut new_node = generate_sector(new_node_id, sector_type, rng, distance, candidate_pos);
            
            // Connect bidirectionally to the current node
            new_node.connections.push(node_id);
            sector_map.sectors.insert(new_node_id, new_node);
            
            // Add reverse connection
            if let Some(current) = sector_map.sectors.get_mut(&node_id) {
                if !current.connections.contains(&new_node_id) {
                    current.connections.push(new_node_id);
                }
            }
        }
    }
}

// Helper functions for deterministic hashing
fn hash_sector_id(id: u32) -> u32 {
    let mut hash = id;
    hash ^= hash >> 16;
    hash = hash.wrapping_mul(0x85ebca6b);
    hash ^= hash >> 13;
    hash = hash.wrapping_mul(0xc2b2ae35);
    hash ^= hash >> 16;
    hash
}

fn hash_to_float(hash: u32) -> f32 {
    (hash as f32) / (u32::MAX as f32)
}

#[derive(Resource, Default)]
pub struct PendingEventTrigger {
    pub sector_id: Option<u32>,
}

pub fn try_travel_to_sector(
    sector_map: &mut SectorMap,
    game_data: &mut crate::game::GameData,
    target_sector_id: u32,
    _event_writer: &mut MessageWriter<events::GameEvent>,
    _active_event: &mut ResMut<events::ActiveEvent>,
    pending_trigger: Option<ResMut<PendingEventTrigger>>,
) {
    // Check fuel
    if game_data.fuel < 1.0 {
        return;
    }
    
    // Check if target node is connected
    if let Some(current_sector) = sector_map.sectors.get(&sector_map.current_sector_id) {
        if !current_sector.connections.contains(&target_sector_id) {
            return; // Not connected
        }
    }
    
    let source_sector_id = sector_map.current_sector_id;
    let mut rng = rand::thread_rng();
    
    // Generate target node if it doesn't exist (it's a reserved ID)
    if !sector_map.sectors.contains_key(&target_sector_id) {
        let distance = sector_map.distance_traveled + 1;
        let sector_type = generate_random_sector_type(&mut rng, distance);
        
        // Calculate position for target node (in direction from source to where target should be)
        let source_pos = sector_map.sectors.get(&source_sector_id)
            .map(|s| s.position)
            .unwrap_or(Vec2::ZERO);
        
        // Use deterministic hash to calculate position for non-existent node
        let target_hash = hash_sector_id(target_sector_id);
        let angle = hash_to_float(target_hash) * 2.0 * std::f32::consts::PI;
        let target_pos = source_pos + Vec2::new(angle.cos() * 300.0, angle.sin() * 300.0);
        
        let mut new_node = generate_sector(target_sector_id, sector_type, &mut rng, distance, target_pos);
        
        // Connect back to source (bidirectional)
        new_node.connections.push(source_sector_id);
        sector_map.sectors.insert(target_sector_id, new_node);
        
        // Ensure source has connection to target
        if let Some(source) = sector_map.sectors.get_mut(&source_sector_id) {
            if !source.connections.contains(&target_sector_id) {
                source.connections.push(target_sector_id);
            }
        }
    }
    
    // Travel to target node
    sector_map.current_sector_id = target_sector_id;
    sector_map.distance_traveled += 1;
    game_data.fuel -= 1.0;
    game_data.current_sector = get_sector_number(target_sector_id); // Update sector number procedurally
    
    // Check if this is the first time visiting this node BEFORE marking as visited
    let first_time_here = if let Some(sector) = sector_map.sectors.get(&target_sector_id) {
        !sector.visited
    } else {
        false
    };
    
    // Mark as visited
    if let Some(sector) = sector_map.sectors.get_mut(&target_sector_id) {
        sector.visited = true;
    }
    
    // Generate new nodes around the target node (procedural expansion, away from source)
    // Only expand if this is the first time visiting this node
    if first_time_here {
        let distance = sector_map.distance_traveled;
        generate_nodes_around(sector_map, target_sector_id, Some(source_sector_id), &mut rng, distance);
    }
    
    // Mark that we need to trigger an event for this sector
    if let Some(mut trigger) = pending_trigger {
        trigger.sector_id = Some(target_sector_id);
    }
}

/// System to trigger events after navigation completes
fn trigger_events_after_navigation(
    sector_map: Res<crate::sector::SectorMap>,
    pending_trigger: Option<ResMut<PendingEventTrigger>>,
    mut event_writer: MessageWriter<events::GameEvent>,
    mut active_event: ResMut<events::ActiveEvent>,
    game_data: Res<crate::game::GameData>,
    mut llm_request_queue: Option<ResMut<crate::llm::LlmRequestQueue>>,
    event_history: Option<Res<crate::llm::EventHistory>>,
) {
    if let Some(mut trigger) = pending_trigger {
        if let Some(sector_id) = trigger.sector_id.take() {
            events::trigger_event_for_sector(
                &sector_map,
                sector_id,
                &mut event_writer,
                &mut active_event,
                &game_data,
                llm_request_queue.as_deref_mut(),
                event_history.as_deref(),
            );
        }
    }
}