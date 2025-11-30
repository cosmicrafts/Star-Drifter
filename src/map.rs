use bevy::prelude::*;
use std::collections::HashMap;
use crate::sector::SectorMap;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, setup_map_visual)
            .add_systems(Update, (
                update_map_visual,
                handle_node_clicks,
            ));
    }
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
    pub sector_positions: HashMap<u32, Vec2>, // Cache positions to prevent recalculation
}

fn setup_map_visual(mut commands: Commands) {
    commands.insert_resource(MapVisual {
        node_entities: HashMap::new(),
        connection_entities: Vec::new(),
        sector_positions: HashMap::new(),
    });
}

fn update_map_visual(
    mut commands: Commands,
    windows: Query<&Window>,
    sector_map: Res<SectorMap>,
    mut map_visual: ResMut<MapVisual>,
    node_query: Query<(Entity, &MapNode)>,
    connection_query: Query<Entity, (With<ConnectionLine>, Without<MapNode>)>,
    label_query: Query<Entity, With<NodeLabel>>,
) {
    // Get window size to adapt the map
    let Ok(window) = windows.single() else { return; };
    let window_width = window.width();
    let window_height = window.height();
    
    // Use positions directly from sectors (they're stored in Sector.position)
    // Update cache for compatibility with existing code
    for (sector_id, sector) in sector_map.sectors.iter() {
        map_visual.sector_positions.insert(*sector_id, sector.position);
    }
    
    // Create/update nodes
    for (sector_id, sector) in sector_map.sectors.iter() {
        let pos = sector.position;
        if !map_visual.node_entities.contains_key(sector_id) {
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
        } else {
            // Update existing node position and color
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
    
    // Update labels for connected nodes
    for entity in label_query.iter() {
        commands.entity(entity).despawn();
    }
    
    // Recreate labels for nodes connected to current sector (show numbers)
    if let Some(current_sector) = sector_map.sectors.get(&sector_map.current_sector_id) {
        let mut seen = std::collections::HashSet::new();
        let mut label_index = 0;
        
        for &connected_id in &current_sector.connections {
            // Skip if already seen
            if seen.contains(&connected_id) {
                continue;
            }
            seen.insert(connected_id);
            
            // Show label for all connected nodes
            // Use position from sector if it exists, otherwise calculate temporary position
            let pos = if let Some(connected_sector) = sector_map.sectors.get(&connected_id) {
                connected_sector.position
            } else {
                // Calculate temporary position for non-existent node (will be generated on travel)
                let current_pos = current_sector.position;
                let angle = (hash_to_float(hash_sector_id(connected_id)) * 2.0 * std::f32::consts::PI) 
                    + (seen.len() as f32 * 0.5); // Spread around
                let distance = 300.0;
                current_pos + Vec2::new(angle.cos() * distance, angle.sin() * distance)
            };
            
            label_index += 1;
            commands.spawn((
                NodeLabel { _sector_id: connected_id },
                Text2d::new(format!("{}", label_index)),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 0.0)),
                Transform::from_translation(Vec3::new(pos.x, pos.y - 25.0, 2.0)),
            ));
        }
    }
    
    // Create connection lines
    let mut existing_connections = std::collections::HashSet::new();
    for entity in connection_query.iter() {
        commands.entity(entity).despawn();
    }
    map_visual.connection_entities.clear();
    
    for (sector_id, sector) in sector_map.sectors.iter() {
        let from_pos = sector.position;
        for &connected_id in &sector.connections {
            // Use position from sector if it exists, otherwise calculate temporary position
            let to_pos = if let Some(connected_sector) = sector_map.sectors.get(&connected_id) {
                connected_sector.position
            } else {
                // Calculate temporary position for non-existent node
                let angle = hash_to_float(hash_sector_id(connected_id)) * 2.0 * std::f32::consts::PI;
                let distance = 300.0;
                from_pos + Vec2::new(angle.cos() * distance, angle.sin() * distance)
            };
            
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

// Deterministic hash function for consistent random values based on sector ID
fn hash_sector_id(id: u32) -> u32 {
    let mut hash = id;
    hash ^= hash >> 16;
    hash = hash.wrapping_mul(0x85ebca6b);
    hash ^= hash >> 13;
    hash = hash.wrapping_mul(0xc2b2ae35);
    hash ^= hash >> 16;
    hash
}

// Convert hash to float in range [0.0, 1.0]
fn hash_to_float(hash: u32) -> f32 {
    (hash as f32) / (u32::MAX as f32)
}

// Positions are now stored directly in Sector.position, so this function
// just syncs them to the cache for compatibility
pub fn calculate_sector_positions(
    sector_map: &SectorMap,
    positions: &mut HashMap<u32, Vec2>,
    _window_width: f32,  // No longer used, but kept for compatibility
    _window_height: f32, // No longer used, but kept for compatibility
) {
    // Simply copy positions from sectors to the cache
    for (sector_id, sector) in sector_map.sectors.iter() {
        positions.insert(*sector_id, sector.position);
    }
}

fn handle_node_clicks(
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    node_query: Query<(Entity, &MapNode, &Transform)>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut sector_map: ResMut<SectorMap>,
    mut game_data: ResMut<crate::game::GameData>,
    mut event_writer: MessageWriter<crate::events::GameEvent>,
    mut active_event: ResMut<crate::events::ActiveEvent>,
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
                                    crate::sector::try_travel_to_sector(
                                        &mut sector_map,
                                        &mut game_data,
                                        map_node._sector_id,
                                        &mut event_writer,
                                        &mut active_event,
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



