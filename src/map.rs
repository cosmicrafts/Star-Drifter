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
}

fn setup_map_visual(mut commands: Commands) {
    commands.insert_resource(MapVisual {
        node_entities: HashMap::new(),
        connection_entities: Vec::new(),
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
    
    // Calculate positions for all sectors (procedural layout, adapted to window)
    let mut positions = HashMap::new();
    calculate_sector_positions(&sector_map, &mut positions, window_width, window_height);
    
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
        let mut seen = std::collections::HashSet::new();
        let mut label_index = 0;
        
        for &connected_id in &current_sector.connections {
            // Skip if already seen or doesn't exist
            if seen.contains(&connected_id) || !sector_map.sectors.contains_key(&connected_id) {
                continue;
            }
            seen.insert(connected_id);
            
            if let Some(&pos) = positions.get(&connected_id) {
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
    _window_width: f32,  // No longer used, but kept for compatibility
    _window_height: f32, // No longer used, but kept for compatibility
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
    
    // FIXED SPACING - no longer dependent on window size
    // Map can be larger than window, PanCam allows navigation
    const LAYER_SPACING: f32 = 300.0;  // Horizontal spacing between layers
    const NODE_SPACING: f32 = 200.0;   // Vertical spacing between nodes in a layer
    const START_X: f32 = 0.0;           // Starting X position (center of map)
    const START_Y: f32 = 0.0;           // Starting Y position (center of map)
    
    for (layer, sector_ids) in layer_map.iter() {
        let layer_x = START_X + (*layer as f32 * LAYER_SPACING);
        let count = sector_ids.len() as f32;
        let total_height = if count > 1.0 { (count - 1.0) * NODE_SPACING } else { 0.0 };
        let start_y_offset = START_Y - (total_height / 2.0);
        
        for (i, &sector_id) in sector_ids.iter().enumerate() {
            let y = start_y_offset + (i as f32 * NODE_SPACING);
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
    mut event_writer: MessageWriter<crate::events::GameEvent>,
    active_event: ResMut<crate::events::ActiveEvent>,
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



