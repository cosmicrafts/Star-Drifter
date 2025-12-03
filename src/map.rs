use bevy::prelude::*;
use std::collections::HashMap;
use crate::sector::{SectorMap, SectorType};

// ============================================
// MAP PLUGIN - COSMIC STAR MAP VISUALIZATION
// ============================================

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, setup_map)
            .add_systems(Update, (
                draw_connections,  // Draw lines first (at z=0)
                update_map_nodes,   // Then nodes (at z=10)
                handle_node_clicks,
                animate_current_node,
            ));
    }
}

// ============================================
// COMPONENTS
// ============================================

#[derive(Component)]
pub struct MapNode {
    pub sector_id: u32,
}

#[derive(Component)]
pub struct NodeLabel {
    pub _sector_id: u32,
}

#[derive(Component)]
pub struct SectorTypeIcon {
    pub _sector_id: u32,
}

#[derive(Component)]
pub struct CurrentNodePulse {
    pub time: f32,
}

#[derive(Component)]
pub struct ConnectionLine {
    pub _from_id: u32,
    pub _to_id: u32,
}

// ============================================
// RESOURCES
// ============================================

#[derive(Resource)]
pub struct MapVisual {
    pub node_entities: HashMap<u32, Entity>,
    pub sector_positions: HashMap<u32, Vec2>,
}

// ============================================
// COLORS - Space Theme
// ============================================

const NODE_CURRENT: Color = Color::srgb(0.2, 1.0, 0.4);      // Bright green
const NODE_VISITED: Color = Color::srgb(0.3, 0.5, 0.8);      // Blue
const NODE_UNVISITED: Color = Color::srgb(0.7, 0.75, 0.85);  // Light gray-blue
const NODE_STATION: Color = Color::srgb(1.0, 0.85, 0.2);     // Gold
const NODE_COMBAT: Color = Color::srgb(0.9, 0.3, 0.2);       // Red
const NODE_ANOMALY: Color = Color::srgb(0.7, 0.3, 0.9);      // Purple
const NODE_DANGER: Color = Color::srgb(0.8, 0.5, 0.2);       // Orange
const CONNECTION_ACTIVE: Color = Color::srgb(0.3, 0.9, 0.5); // Bright cyan-green
const CONNECTION_DIM: Color = Color::srgba(0.3, 0.4, 0.5, 0.4); // Dim gray

// ============================================
// SETUP
// ============================================

fn setup_map(mut commands: Commands) {
    commands.insert_resource(MapVisual {
        node_entities: HashMap::new(),
        sector_positions: HashMap::new(),
    });
}

// ============================================
// UPDATE SYSTEMS
// ============================================

fn update_map_nodes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    sector_map: Res<SectorMap>,
    mut map_visual: ResMut<MapVisual>,
    mut node_query: Query<(Entity, &MapNode, &mut Transform, &mut Mesh2d, &MeshMaterial2d<ColorMaterial>)>,
    label_query: Query<Entity, With<NodeLabel>>,
    icon_query: Query<Entity, With<SectorTypeIcon>>,
) {
    // Update position cache
    for (sector_id, sector) in sector_map.sectors.iter() {
        map_visual.sector_positions.insert(*sector_id, sector.position);
    }

    // Despawn old labels and icons
    for entity in label_query.iter() {
        commands.entity(entity).despawn();
    }
    for entity in icon_query.iter() {
        commands.entity(entity).despawn();
    }

    // Create or update nodes
    for (sector_id, sector) in sector_map.sectors.iter() {
        let pos = sector.position;
        let is_current = *sector_id == sector_map.current_sector_id;
        
        // Determine node color based on type and state
        let base_color = if is_current {
            NODE_CURRENT
        } else {
            match sector.sector_type {
                SectorType::Station => NODE_STATION,
                SectorType::Combat => NODE_COMBAT,
                SectorType::Anomaly => NODE_ANOMALY,
                SectorType::DarkRift | SectorType::Distress => NODE_DANGER,
                _ => if sector.visited { NODE_VISITED } else { NODE_UNVISITED },
            }
        };
        
        // Node size based on importance
        let size = if is_current { 18.0 } else { 12.0 };

        if let Some(&entity) = map_visual.node_entities.get(sector_id) {
            // Update existing node
            if let Ok((_, _, mut transform, _, mat_handle)) = node_query.get_mut(entity) {
                transform.translation = Vec3::new(pos.x, pos.y, 10.0);
                transform.scale = Vec3::splat(1.0);
                
                // Update material color
                if let Some(mat) = materials.get_mut(&mat_handle.0) {
                    mat.color = base_color;
                }
            }
        } else {
            // Create new node with hexagon shape for visual interest
            let mesh = meshes.add(RegularPolygon::new(size, 6));
            let material = materials.add(ColorMaterial::from(base_color));
            
            let mut entity_cmds = commands.spawn((
                MapNode { sector_id: *sector_id },
                Mesh2d(mesh),
                MeshMaterial2d(material),
                Transform::from_translation(Vec3::new(pos.x, pos.y, 10.0)),
            ));
            
            // Add pulse animation to current node
            if is_current {
                entity_cmds.insert(CurrentNodePulse { time: 0.0 });
            }
            
            map_visual.node_entities.insert(*sector_id, entity_cmds.id());
        }
    }

    // Create labels for connected nodes (showing exit numbers)
    if let Some(current_sector) = sector_map.sectors.get(&sector_map.current_sector_id) {
        let mut seen = std::collections::HashSet::new();
        
        for (idx, &connected_id) in current_sector.connections.iter().enumerate() {
            if seen.contains(&connected_id) {
                continue;
            }
            seen.insert(connected_id);
            
            let pos = sector_map.sectors.get(&connected_id)
                .map(|s| s.position)
                .unwrap_or_else(|| {
                    // Calculate temporary position for ungenerated node
                    let angle = hash_to_float(hash_sector_id(connected_id)) * std::f32::consts::TAU;
                    current_sector.position + Vec2::new(angle.cos(), angle.sin()) * 300.0
                });
            
            // Spawn number label below node
            commands.spawn((
                NodeLabel { _sector_id: connected_id },
                Text2d::new(format!("{}", idx + 1)),
                TextFont { font_size: 18.0, ..default() },
                TextColor(Color::srgb(1.0, 0.9, 0.3)),
                Transform::from_translation(Vec3::new(pos.x, pos.y - 28.0, 12.0)),
            ));
            
            // Spawn sector type icon above node using colored shapes
            if let Some(sector) = sector_map.sectors.get(&connected_id) {
                let (mesh, material) = match sector.sector_type {
                    SectorType::Station => create_station_icon(&mut meshes, &mut materials),
                    SectorType::Combat => create_combat_icon(&mut meshes, &mut materials),
                    SectorType::Anomaly => create_anomaly_icon(&mut meshes, &mut materials),
                    SectorType::Distress => create_distress_icon(&mut meshes, &mut materials),
                    SectorType::Empty => create_empty_icon(&mut meshes, &mut materials),
                    SectorType::Nebula => create_nebula_icon(&mut meshes, &mut materials),
                    SectorType::AsteroidField => create_asteroid_icon(&mut meshes, &mut materials),
                    SectorType::DarkRift => create_darkrift_icon(&mut meshes, &mut materials),
                    SectorType::CelestialSite => create_celestial_icon(&mut meshes, &mut materials),
                    SectorType::AetheriumField => create_aetherium_icon(&mut meshes, &mut materials),
                };
                
                commands.spawn((
                    SectorTypeIcon { _sector_id: connected_id },
                    mesh,
                    material,
                    Transform::from_translation(Vec3::new(pos.x, pos.y + 24.0, 12.0)),
                ));
            }
        }
    }
}

fn draw_connections(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    sector_map: Res<SectorMap>,
    map_visual: Res<MapVisual>,
    line_query: Query<Entity, With<ConnectionLine>>,
) {
    // Despawn old lines
    for entity in line_query.iter() {
        commands.entity(entity).despawn();
    }
    
    let mut drawn = std::collections::HashSet::new();
    
    for (sector_id, sector) in sector_map.sectors.iter() {
        let from_pos = sector.position;
        let is_current = *sector_id == sector_map.current_sector_id;
        
        for &connected_id in &sector.connections {
            // Avoid duplicate lines
            let key = if *sector_id < connected_id {
                (*sector_id, connected_id)
            } else {
                (connected_id, *sector_id)
            };
            
            if drawn.contains(&key) {
                continue;
            }
            drawn.insert(key);
            
            // Get destination position
            let to_pos = map_visual.sector_positions.get(&connected_id)
                .copied()
                .unwrap_or_else(|| {
                    let angle = hash_to_float(hash_sector_id(connected_id)) * std::f32::consts::TAU;
                    from_pos + Vec2::new(angle.cos(), angle.sin()) * 300.0
                });
            
            // Determine if this is a connection from current node
            let is_active = is_current || connected_id == sector_map.current_sector_id;
            
            let color = if is_active {
                CONNECTION_ACTIVE
            } else {
                CONNECTION_DIM
            };
            
            // Create line as a mesh (rectangle)
            let diff = to_pos - from_pos;
            let length = diff.length();
            
            // Skip if length is too small
            if length < 0.1 {
                continue;
            }
            
            let direction = diff / length; // Normalize safely
            let midpoint = (from_pos + to_pos) * 0.5;
            let thickness = if is_active { 2.0 } else { 1.0 };
            
            // Create a thin rectangle for the line
            let line_mesh = meshes.add(Rectangle::new(length, thickness));
            let line_material = materials.add(ColorMaterial::from(color));
            
            // Calculate rotation angle
            let angle = direction.y.atan2(direction.x);
            
            commands.spawn((
                ConnectionLine {
                    _from_id: *sector_id,
                    _to_id: connected_id,
                },
                Mesh2d(line_mesh),
                MeshMaterial2d(line_material),
                Transform {
                    translation: Vec3::new(midpoint.x, midpoint.y, 0.0), // z=0, below nodes
                    rotation: Quat::from_rotation_z(angle),
                    ..default()
                },
            ));
        }
    }
}

fn animate_current_node(
    time: Res<Time>,
    sector_map: Res<SectorMap>,
    mut query: Query<(&MapNode, &mut Transform, &mut CurrentNodePulse)>,
    mut commands: Commands,
    map_visual: Res<MapVisual>,
) {
    // Ensure current node has pulse component
    if let Some(&current_entity) = map_visual.node_entities.get(&sector_map.current_sector_id) {
        if query.get(current_entity).is_err() {
            commands.entity(current_entity).insert(CurrentNodePulse { time: 0.0 });
        }
    }
    
    // Remove pulse from non-current nodes
    for (node, _, _) in query.iter() {
        if node.sector_id != sector_map.current_sector_id {
            if let Some(&entity) = map_visual.node_entities.get(&node.sector_id) {
                commands.entity(entity).remove::<CurrentNodePulse>();
            }
        }
    }
    
    // Animate pulse
    for (node, mut transform, mut pulse) in query.iter_mut() {
        if node.sector_id == sector_map.current_sector_id {
            pulse.time += time.delta_secs() * 3.0;
            let scale = 1.0 + (pulse.time.sin() * 0.15);
            transform.scale = Vec3::splat(scale);
        }
    }
}

fn handle_node_clicks(
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    node_query: Query<(&MapNode, &Transform)>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut sector_map: ResMut<SectorMap>,
    mut game_data: ResMut<crate::game::GameData>,
    mut event_writer: MessageWriter<crate::events::GameEvent>,
    mut active_event: ResMut<crate::events::ActiveEvent>,
) {
    // Don't process clicks during events
    if active_event.event.is_some() {
        return;
    }
    
    if !mouse_button.just_pressed(MouseButton::Left) {
        return;
    }
    
    let Ok(window) = windows.single() else { return };
    let Some(cursor_pos) = window.cursor_position() else { return };
    let Ok((camera, camera_transform)) = camera_query.single() else { return };
    
    // Convert screen to world coordinates
    let window_size = Vec2::new(window.width(), window.height());
    let ndc = Vec2::new(
        (cursor_pos.x / window_size.x) * 2.0 - 1.0,
        1.0 - (cursor_pos.y / window_size.y) * 2.0,
    );
    
    let viewport_size = camera.logical_viewport_size().unwrap_or(window_size);
    let camera_pos = camera_transform.translation().truncate();
    let world_pos = camera_pos + ndc * viewport_size * 0.5;
    
    // Find closest clickable node
    let mut closest: Option<(u32, f32)> = None;
    
    for (node, transform) in node_query.iter() {
        let node_pos = transform.translation.truncate();
        let dist = (world_pos - node_pos).length();
        
        // Click radius
        if dist < 35.0 {
            if closest.is_none() || dist < closest.unwrap().1 {
                closest = Some((node.sector_id, dist));
            }
        }
    }
    
    // Try to travel to clicked node
    if let Some((target_id, _)) = closest {
        if let Some(current) = sector_map.sectors.get(&sector_map.current_sector_id) {
            if current.connections.contains(&target_id) {
                crate::sector::try_travel_to_sector(
                    &mut sector_map,
                    &mut game_data,
                    target_id,
                    &mut event_writer,
                    &mut active_event,
                );
            }
        }
    }
}

// ============================================
// ICON CREATION HELPERS
// ============================================

fn create_station_icon(meshes: &mut ResMut<Assets<Mesh>>, materials: &mut ResMut<Assets<ColorMaterial>>) -> (Mesh2d, MeshMaterial2d<ColorMaterial>) {
    let mesh = meshes.add(Rectangle::new(8.0, 8.0));
    let material = materials.add(ColorMaterial::from(NODE_STATION));
    (Mesh2d(mesh), MeshMaterial2d(material))
}

fn create_combat_icon(meshes: &mut ResMut<Assets<Mesh>>, materials: &mut ResMut<Assets<ColorMaterial>>) -> (Mesh2d, MeshMaterial2d<ColorMaterial>) {
    let mesh = meshes.add(RegularPolygon::new(6.0, 4)); // Diamond shape
    let material = materials.add(ColorMaterial::from(NODE_COMBAT));
    (Mesh2d(mesh), MeshMaterial2d(material))
}

fn create_anomaly_icon(meshes: &mut ResMut<Assets<Mesh>>, materials: &mut ResMut<Assets<ColorMaterial>>) -> (Mesh2d, MeshMaterial2d<ColorMaterial>) {
    let mesh = meshes.add(Circle::new(6.0));
    let material = materials.add(ColorMaterial::from(NODE_ANOMALY));
    (Mesh2d(mesh), MeshMaterial2d(material))
}

fn create_distress_icon(meshes: &mut ResMut<Assets<Mesh>>, materials: &mut ResMut<Assets<ColorMaterial>>) -> (Mesh2d, MeshMaterial2d<ColorMaterial>) {
    let mesh = meshes.add(RegularPolygon::new(6.0, 6)); // Hexagon
    let material = materials.add(ColorMaterial::from(NODE_DANGER));
    (Mesh2d(mesh), MeshMaterial2d(material))
}

fn create_empty_icon(meshes: &mut ResMut<Assets<Mesh>>, materials: &mut ResMut<Assets<ColorMaterial>>) -> (Mesh2d, MeshMaterial2d<ColorMaterial>) {
    let mesh = meshes.add(Circle::new(3.0));
    let material = materials.add(ColorMaterial::from(Color::srgb(0.5, 0.5, 0.5)));
    (Mesh2d(mesh), MeshMaterial2d(material))
}

fn create_nebula_icon(meshes: &mut ResMut<Assets<Mesh>>, materials: &mut ResMut<Assets<ColorMaterial>>) -> (Mesh2d, MeshMaterial2d<ColorMaterial>) {
    let mesh = meshes.add(Ellipse::new(6.0, 4.0));
    let material = materials.add(ColorMaterial::from(Color::srgb(0.5, 0.6, 0.9)));
    (Mesh2d(mesh), MeshMaterial2d(material))
}

fn create_asteroid_icon(meshes: &mut ResMut<Assets<Mesh>>, materials: &mut ResMut<Assets<ColorMaterial>>) -> (Mesh2d, MeshMaterial2d<ColorMaterial>) {
    let mesh = meshes.add(RegularPolygon::new(5.0, 8)); // Octagon
    let material = materials.add(ColorMaterial::from(Color::srgb(0.6, 0.5, 0.4)));
    (Mesh2d(mesh), MeshMaterial2d(material))
}

fn create_darkrift_icon(meshes: &mut ResMut<Assets<Mesh>>, materials: &mut ResMut<Assets<ColorMaterial>>) -> (Mesh2d, MeshMaterial2d<ColorMaterial>) {
    let mesh = meshes.add(Circle::new(6.0));
    let material = materials.add(ColorMaterial::from(Color::srgb(0.2, 0.0, 0.3)));
    (Mesh2d(mesh), MeshMaterial2d(material))
}

fn create_celestial_icon(meshes: &mut ResMut<Assets<Mesh>>, materials: &mut ResMut<Assets<ColorMaterial>>) -> (Mesh2d, MeshMaterial2d<ColorMaterial>) {
    let mesh = meshes.add(RegularPolygon::new(6.0, 5)); // Star shape (pentagon)
    let material = materials.add(ColorMaterial::from(NODE_STATION));
    (Mesh2d(mesh), MeshMaterial2d(material))
}

fn create_aetherium_icon(meshes: &mut ResMut<Assets<Mesh>>, materials: &mut ResMut<Assets<ColorMaterial>>) -> (Mesh2d, MeshMaterial2d<ColorMaterial>) {
    let mesh = meshes.add(RegularPolygon::new(6.0, 6)); // Hexagon
    let material = materials.add(ColorMaterial::from(Color::srgb(0.2, 0.8, 0.9)));
    (Mesh2d(mesh), MeshMaterial2d(material))
}

// ============================================
// UTILITIES
// ============================================

fn hash_sector_id(id: u32) -> u32 {
    let mut h = id;
    h ^= h >> 16;
    h = h.wrapping_mul(0x85ebca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2ae35);
    h ^= h >> 16;
    h
}

fn hash_to_float(hash: u32) -> f32 {
    (hash as f32) / (u32::MAX as f32)
}

/// Public function to sync sector positions to a cache (for UI compatibility)
pub fn calculate_sector_positions(
    sector_map: &SectorMap,
    positions: &mut HashMap<u32, Vec2>,
    _window_width: f32,
    _window_height: f32,
) {
    for (id, sector) in sector_map.sectors.iter() {
        positions.insert(*id, sector.position);
    }
}
