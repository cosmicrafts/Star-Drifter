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
            .add_systems(Startup, (setup_sector_map, setup_map_visual))
            .configure_sets(Update, NavigationSystemSet.after(crate::events::EventSystemSet))
            .add_systems(Update, (
                handle_sector_navigation,
                update_map_visual,
                handle_node_clicks,
            ).in_set(NavigationSystemSet));
    }
}

#[derive(Resource)]
pub struct SectorMap {
    pub current_sector_id: u32,
    pub sectors: HashMap<u32, Sector>,
    pub distance_traveled: u32, // For scaling difficulty
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

#[derive(Component)]
pub struct MapNode {
    pub _sector_id: u32,
}

#[derive(Component)]
pub struct NodeLabel {
    pub _sector_id: u32,
}

#[derive(Component)]
pub struct ConnectionLine {
}

#[derive(Resource)]
pub struct MapVisual {
    pub node_entities: HashMap<u32, Entity>,
    pub connection_entities: Vec<Entity>,
}

fn setup_sector_map(mut commands: Commands) {
    let mut sectors = HashMap::new();
    let mut rng = rand::thread_rng();
    
    // Generate a complete procedural map (like FTL)
    // Create 5-7 layers with 2-4 nodes per layer
    let num_layers = rng.gen_range(5..=7);
    let mut next_id = 0u32;
    let mut layer_nodes: Vec<Vec<u32>> = Vec::new();
    
    // Generate first layer (starting sector)
    let starting_sector = generate_sector(
        next_id,
        SectorType::Station,
        &mut rng,
        0,
    );
    sectors.insert(next_id, starting_sector);
    layer_nodes.push(vec![next_id]);
    next_id += 1;
    
    // Generate remaining layers
    for layer in 1..num_layers {
        let nodes_in_layer = rng.gen_range(2..=4);
        let mut current_layer = Vec::new();
        
        for _ in 0..nodes_in_layer {
            let distance = layer as u32;
            let sector_type = generate_random_sector_type(&mut rng, distance);
            let sector = generate_sector(next_id, sector_type, &mut rng, distance);
            sectors.insert(next_id, sector);
            current_layer.push(next_id);
            next_id += 1;
        }
        
        // Connect to previous layer
        let prev_layer = &layer_nodes[layer - 1];
        for &current_id in &current_layer {
            // Each node connects to 1-2 nodes from previous layer
            let num_connections = rng.gen_range(1..=2.min(prev_layer.len()));
            let mut connected = std::collections::HashSet::new();
            let mut connections_to_add = Vec::new();
            
            for _ in 0..num_connections {
                let target_id = prev_layer[rng.gen_range(0..prev_layer.len())];
                if !connected.contains(&target_id) {
                    connected.insert(target_id);
                    connections_to_add.push(target_id);
                }
            }
            
            // Add forward connections
            if let Some(sector) = sectors.get_mut(&current_id) {
                sector.connections.extend(connections_to_add.iter().copied());
            }
            
            // Add reverse connections
            for &target_id in &connections_to_add {
                if let Some(target_sector) = sectors.get_mut(&target_id) {
                    target_sector.connections.push(current_id);
                }
            }
        }
        
        layer_nodes.push(current_layer);
    }
    
    commands.insert_resource(SectorMap {
        current_sector_id: 0,
        sectors,
        distance_traveled: 0,
    });
}

fn generate_sector(
    id: u32,
    sector_type: SectorType,
    rng: &mut rand::rngs::ThreadRng,
    distance: u32,
) -> Sector {
    let name = generate_sector_name(&sector_type, id);
    let description = sector_type.description().to_string();
    let events = generate_sector_events(&sector_type, rng);
    let danger_level = calculate_danger_level(distance, &sector_type);
    
    // Generate 1-3 connections to other sectors (will be created when needed)
    let num_connections = rng.gen_range(1..=3);
    let connections = Vec::new();
    
    // For now, we'll generate connection IDs that will be created on-demand
    // This creates a web-like structure
    for _ in 0..num_connections {
        // Generate a new sector ID that doesn't exist yet
        // We'll use a simple approach: next available IDs
        // In practice, these will be generated when player travels
    }
    
    Sector {
        _id: id,
        sector_type,
        name,
        description,
        connections,
        visited: false,
        events,
        danger_level,
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
    active_event: ResMut<crate::events::ActiveEvent>,
    input_consumed: Res<crate::events::InputConsumed>,
) {
    // Don't allow navigation if an event is currently active
    // Numbers should only be used for event choices when an event is active
    if active_event.event.is_some() {
        return;
    }
    
    if let Some(current_sector) = sector_map.sectors.get(&sector_map.current_sector_id) {
        let connections = current_sector.connections.clone();
        
        // Handle navigation to connected sectors using number keys 1-9
        for (i, &target_id) in connections.iter().enumerate() {
            let key = match i {
                0 => KeyCode::Digit1,
                1 => KeyCode::Digit2,
                2 => KeyCode::Digit3,
                3 => KeyCode::Digit4,
                4 => KeyCode::Digit5,
                5 => KeyCode::Digit6,
                6 => KeyCode::Digit7,
                7 => KeyCode::Digit8,
                8 => KeyCode::Digit9,
                _ => continue,
            };
            
            // Skip if this key was already consumed by event system
            if input_consumed.keys.contains(&key) {
                continue;
            }
            
            if keyboard.just_pressed(key) {
                try_travel_to_sector(
                    &mut sector_map,
                    &mut game_data,
                    target_id,
                    &mut event_writer,
                    active_event,
                );
                break;
            }
        }
    }
}

fn try_travel_to_sector(
    sector_map: &mut SectorMap,
    game_data: &mut crate::game::GameData,
    target_sector_id: u32,
    event_writer: &mut MessageWriter<events::GameEvent>,
    mut active_event: ResMut<events::ActiveEvent>,
) {
    // Check fuel
    if game_data.fuel < 1.0 {
        return;
    }
    
    // Check if target sector exists and is connected
    if let Some(current_sector) = sector_map.sectors.get(&sector_map.current_sector_id) {
        if !current_sector.connections.contains(&target_sector_id) {
            return; // Not connected
        }
    }
    
    if !sector_map.sectors.contains_key(&target_sector_id) {
        return; // Sector doesn't exist
    }
    
    // Travel to sector
    sector_map.current_sector_id = target_sector_id;
    sector_map.distance_traveled += 1;
    game_data.fuel -= 1.0;
    game_data.current_sector = target_sector_id;
    
    // Mark as visited
    if let Some(sector) = sector_map.sectors.get_mut(&target_sector_id) {
        sector.visited = true;
    }
    
    // Automatically trigger event for the new sector
    events::trigger_event_for_sector(sector_map, target_sector_id, event_writer, &mut *active_event);
}


// Helper function to get current sector (for UI)

// Visual map system
fn setup_map_visual(mut commands: Commands) {
    commands.insert_resource(MapVisual {
        node_entities: HashMap::new(),
        connection_entities: Vec::new(),
    });
}

fn update_map_visual(
    mut commands: Commands,
    sector_map: Res<SectorMap>,
    mut map_visual: ResMut<MapVisual>,
    node_query: Query<(Entity, &MapNode)>,
    connection_query: Query<Entity, (With<ConnectionLine>, Without<MapNode>)>,
    label_query: Query<Entity, With<NodeLabel>>,
) {
    // Calculate positions for all sectors (procedural layout)
    let mut positions = HashMap::new();
    calculate_sector_positions(&sector_map, &mut positions);
    
    // Create/update nodes
    for (sector_id, sector) in sector_map.sectors.iter() {
        if !map_visual.node_entities.contains_key(sector_id) {
            if let Some(&pos) = positions.get(sector_id) {
                let is_current = *sector_id == sector_map.current_sector_id;
                let color = if is_current {
                    Color::srgb(0.0, 1.0, 0.0) // Green for current
                } else if sector.visited {
                    Color::srgb(0.5, 0.5, 0.5) // Gray for visited
                } else {
                    Color::srgb(0.8, 0.8, 0.8) // White for unvisited
                };
                
                let size = if is_current { 15.0 } else { 10.0 };
                
                let node_entity = commands.spawn((
                    MapNode {
                        _sector_id: *sector_id,
                    },
                    Sprite {
                        color,
                        custom_size: Some(Vec2::new(size, size)),
                        ..default()
                    },
                    Transform::from_translation(Vec3::new(pos.x, pos.y, 1.0)),
                )).id();
                
                
                map_visual.node_entities.insert(*sector_id, node_entity);
            }
        } else {
            // Update existing node position and color
            if let Some(&pos) = positions.get(sector_id) {
                let is_current = *sector_id == sector_map.current_sector_id;
                let color = if is_current {
                    Color::srgb(0.0, 1.0, 0.0)
                } else if sector.visited {
                    Color::srgb(0.5, 0.5, 0.5)
                } else {
                    Color::srgb(0.8, 0.8, 0.8)
                };
                
                if let Ok((entity, _)) = node_query.get(*map_visual.node_entities.get(sector_id).unwrap()) {
                    commands.entity(entity).insert((
                        Sprite {
                            color,
                            custom_size: Some(Vec2::new(if is_current { 15.0 } else { 10.0 }, if is_current { 15.0 } else { 10.0 })),
                            ..default()
                        },
                        Transform::from_translation(Vec3::new(pos.x, pos.y, 1.0)),
                    ));
                }
            }
        }
    }
    
    // Update labels for connected nodes
    for entity in label_query.iter() {
        commands.entity(entity).despawn();
    }
    
    // Recreate labels for nodes connected to current sector (show numbers)
    if let Some(current_sector) = sector_map.sectors.get(&sector_map.current_sector_id) {
        for (index, &connected_id) in current_sector.connections.iter().enumerate() {
            if let Some(&pos) = positions.get(&connected_id) {
                commands.spawn((
                    NodeLabel { _sector_id: connected_id },
                    Text2d::new(format!("{}", index + 1)),
                    TextFont {
                        font_size: 20.0,
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 1.0, 0.0)),
                    Transform::from_translation(Vec3::new(pos.x, pos.y - 25.0, 2.0)),
                ));
            }
        }
    }
    
    // Create connection lines
    let mut existing_connections = std::collections::HashSet::new();
    for entity in connection_query.iter() {
        commands.entity(entity).despawn();
    }
    map_visual.connection_entities.clear();
    
    for (sector_id, sector) in sector_map.sectors.iter() {
        if let Some(&from_pos) = positions.get(sector_id) {
            for &connected_id in &sector.connections {
                if let Some(&to_pos) = positions.get(&connected_id) {
                    // Avoid duplicate connections
                    let connection_key = if sector_id < &connected_id {
                        (*sector_id, connected_id)
                    } else {
                        (connected_id, *sector_id)
                    };
                    
                    if !existing_connections.contains(&connection_key) {
                        existing_connections.insert(connection_key);
                        
                        // Create line between nodes
                        let mid_point = (from_pos + to_pos) / 2.0;
                        let direction = to_pos - from_pos;
                        let length = direction.length();
                        let angle = direction.y.atan2(direction.x);
                        
                        let line_entity = commands.spawn((
                            ConnectionLine {
                            },
                            Sprite {
                                color: Color::srgb(0.3, 0.3, 0.3),
                                custom_size: Some(Vec2::new(length, 2.0)),
                                ..default()
                            },
                            Transform {
                                translation: Vec3::new(mid_point.x, mid_point.y, 0.0),
                                rotation: Quat::from_rotation_z(angle),
                                ..default()
                            },
                        )).id();
                        
                        map_visual.connection_entities.push(line_entity);
                    }
                }
            }
        }
    }
}

fn calculate_sector_positions(
    sector_map: &SectorMap,
    positions: &mut HashMap<u32, Vec2>,
) {
    // Simple layout: sectors arranged in layers based on distance
    // Each layer is a row, sectors spread horizontally
    let mut layer_map: HashMap<u32, Vec<u32>> = HashMap::new();
    
    // BFS to assign layers
    let mut queue = std::collections::VecDeque::new();
    let mut visited = std::collections::HashSet::new();
    queue.push_back((0, 0)); // (sector_id, layer)
    visited.insert(0);
    
    while let Some((sector_id, layer)) = queue.pop_front() {
        layer_map.entry(layer).or_insert_with(Vec::new).push(sector_id);
        
        if let Some(sector) = sector_map.sectors.get(&sector_id) {
            for &connected_id in &sector.connections {
                if !visited.contains(&connected_id) {
                    visited.insert(connected_id);
                    queue.push_back((connected_id, layer + 1));
                }
            }
        }
    }
    
    // Position sectors in layers (horizontal layout like FTL)
    let layer_spacing = 200.0; // Horizontal spacing between layers
    let node_spacing = 100.0;  // Vertical spacing between nodes in same layer
    let start_x = -500.0;      // Start from left
    let start_y = 150.0;       // Center vertically
    
    for (layer, sector_ids) in layer_map.iter() {
        let layer_x = start_x + (*layer as f32 * layer_spacing);
        let count = sector_ids.len() as f32;
        let total_height = (count - 1.0) * node_spacing;
        let start_y_offset = start_y - (total_height / 2.0);
        
        for (i, &sector_id) in sector_ids.iter().enumerate() {
            let y = start_y_offset + (i as f32 * node_spacing);
            positions.insert(sector_id, Vec2::new(layer_x, y));
        }
    }
}

fn handle_node_clicks(
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    node_query: Query<(Entity, &MapNode, &Transform)>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut sector_map: ResMut<SectorMap>,
    mut game_data: ResMut<crate::game::GameData>,
    mut event_writer: MessageWriter<events::GameEvent>,
    active_event: ResMut<events::ActiveEvent>,
) {
    // Don't allow clicking nodes if an event is currently active
    if active_event.event.is_some() {
        return;
    }
    
    if mouse_button.just_pressed(MouseButton::Left) {
        if let Ok(window) = windows.single() {
            if let Some(cursor_pos) = window.cursor_position() {
                if let Ok((_camera, camera_transform)) = camera_query.single() {
                    // Convert screen position to world position for 2D camera
                    let window_size = Vec2::new(window.width(), window.height());
                    
                    // Get camera position
                    let camera_pos = camera_transform.translation();
                    
                    // For 2D camera with default settings, convert cursor to world coordinates
                    // Bevy 2D uses a coordinate system where (0,0) is at the center
                    let cursor_world_x = (cursor_pos.x - window_size.x / 2.0) + camera_pos.x;
                    let cursor_world_y = (window_size.y / 2.0 - cursor_pos.y) + camera_pos.y;
                    let cursor_world = Vec2::new(cursor_world_x, cursor_world_y);
                    
                    // Check if click is on a node
                    for (_entity, map_node, node_transform) in node_query.iter() {
                        let node_pos = Vec2::new(node_transform.translation.x, node_transform.translation.y);
                        let distance = (cursor_world - node_pos).length();
                        
                        // Click radius (node size + some padding)
                        if distance < 30.0 {
                            // Check if this node is connected to current sector
                            if let Some(current_sector) = sector_map.sectors.get(&sector_map.current_sector_id) {
                                if current_sector.connections.contains(&map_node._sector_id) {
                                    // Travel to this sector
                                    try_travel_to_sector(
                                        &mut sector_map,
                                        &mut game_data,
                                        map_node._sector_id,
                                        &mut event_writer,
                                        active_event,
                                    );
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}