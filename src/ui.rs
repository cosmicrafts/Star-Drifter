use bevy::prelude::*;
use crate::game::{GameState, GameData};
use crate::events::ActiveEvent;
use std::collections::HashMap;

// ============================================
// UI PLUGIN - ALL UI RELATED CODE
// ============================================

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, setup_all_ui)
            .add_systems(Update, (
                // Informative text elements
                update_hud,
                update_event_ui.run_if(in_state(GameState::Playing)),
                update_sector_info.run_if(in_state(GameState::Playing)),
                // Interactive buttons
                handle_all_buttons,
            ));
    }
}

// ============================================
// MARKER COMPONENTS
// ============================================

// Informative text elements
#[derive(Component)]
struct HudText;

#[derive(Component)]
struct EventText;

#[derive(Component)]
struct SectorText;

// Buttons - organized by category
#[derive(Component)]
pub struct CameraCenterButton;

#[derive(Component)]
pub struct CameraZoomInButton;

#[derive(Component)]
pub struct CameraZoomOutButton;

// ============================================
// UI CREATION HELPERS (reusable)
// ============================================

/// Helper to create buttons with consistent style
pub fn create_button(
    parent: &mut ChildSpawnerCommands,
    marker: impl Component,
    text: &str,
    font_size: f32,
    size: (f32, f32), // (width, height)
) -> Entity {
    parent.spawn((
        marker,
        Button,
        Node {
            width: px(size.0),
            height: px(size.1),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(px(2.0)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
    )).with_children(|parent| {
        parent.spawn((
            Text::new(text),
            TextFont {
                font_size,
                ..default()
            },
            TextColor(Color::WHITE),
        ));
    }).id()
}

/// Helper to create text elements with consistent style
pub fn create_text_element<M: Component>(
    commands: &mut Commands,
    marker: Option<M>,
    text: &str,
    font_size: f32,
    color: Color,
    position: (f32, f32), // (top/bottom, left/right)
    position_type: PositionType,
) -> Entity {
    let node = match position_type {
        PositionType::Absolute => Node {
            position_type: PositionType::Absolute,
            top: px(position.0),
            left: px(position.1),
            ..default()
        },
        _ => Node {
            position_type: PositionType::Absolute,
            bottom: px(position.0),
            left: px(position.1),
            ..default()
        },
    };

    let mut entity_commands = commands.spawn((
        Text::new(text),
        TextFont {
            font_size,
            ..default()
        },
        TextColor(color),
        node,
    ));

    if let Some(marker) = marker {
        entity_commands.insert(marker);
    }

    entity_commands.id()
}

/// Helper to create text elements without marker
pub fn create_text_element_no_marker(
    commands: &mut Commands,
    text: &str,
    font_size: f32,
    color: Color,
    position: (f32, f32), // (top/bottom, left/right)
    position_type: PositionType,
) -> Entity {
    create_text_element::<HudText>(commands, None, text, font_size, color, position, position_type)
}

// ============================================
// SETUP - ALL IN ONE PLACE
// ============================================

fn setup_all_ui(mut commands: Commands) {
    // === HUD (top left) ===
    setup_hud(&mut commands);
    
    // === Camera Buttons (top right) ===
    setup_camera_buttons(&mut commands);
    
    // === Sector Info (bottom left) ===
    setup_sector_info(&mut commands);
    
    // === Events (bottom right) ===
    setup_event_ui(&mut commands);
    
    // === Controls Text (bottom left) ===
    setup_controls_text(&mut commands);
}

fn setup_hud(commands: &mut Commands) {
    create_text_element(
        commands,
        Some(HudText),
        "Fuel: 16 | Scrap: 15 | Sector: 1/30",
        24.0,
        Color::WHITE,
        (10.0, 10.0),
        PositionType::Absolute,
    );
}

fn setup_camera_buttons(commands: &mut Commands) {
    // Container for buttons in top right corner
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(10.0),
            right: px(10.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexEnd,
            row_gap: px(5.0),
            ..default()
        },
    )).with_children(|parent| {
        // Center button (GPS icon)
        create_button(parent, CameraCenterButton, "📍", 24.0, (50.0, 50.0));
        
        // Zoom in button
        create_button(parent, CameraZoomInButton, "+", 32.0, (50.0, 50.0));
        
        // Zoom out button
        create_button(parent, CameraZoomOutButton, "−", 32.0, (50.0, 50.0));
    });
}

fn setup_sector_info(commands: &mut Commands) {
    create_text_element(
        commands,
        Some(SectorText),
        "Current Sector: Loading...",
        20.0,
        Color::srgb(0.8, 0.8, 1.0),
        (80.0, 10.0),
        PositionType::Relative, // bottom
    );
}

fn setup_event_ui(commands: &mut Commands) {
    commands.spawn((
        EventText,
        Text::new(""),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 1.0, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(100.0),
            right: px(10.0),
            width: px(400.0),
            ..default()
        },
    ));
}

fn setup_controls_text(commands: &mut Commands) {
    create_text_element_no_marker(
        commands,
        "Controls: 1-9 - Travel to Exit | Click Node - Travel | 1-3 - Event Choices | ESC - Pause",
        16.0,
        Color::srgb(0.7, 0.7, 0.7),
        (10.0, 10.0),
        PositionType::Relative, // bottom
    );
}

// ============================================
// HANDLERS - ALL BUTTONS HERE
// ============================================

fn handle_all_buttons(
    // Camera buttons
    mut center_btn: Query<&Interaction, (Changed<Interaction>, With<CameraCenterButton>)>,
    mut zoom_in_btn: Query<&Interaction, (Changed<Interaction>, With<CameraZoomInButton>)>,
    mut zoom_out_btn: Query<&Interaction, (Changed<Interaction>, With<CameraZoomOutButton>)>,
    // Camera resources and queries
    mut camera_query: Query<&mut Transform, (With<Camera2d>, Without<Button>)>,
    pan_cam_query: Query<&bevy_pancam::PanCam>,
    sector_map: Res<crate::sector::SectorMap>,
    mut camera_animation: ResMut<crate::camera::CameraAnimation>,
    windows: Query<&Window>,
) {
    // Handle center button (GPS)
    for interaction in center_btn.iter_mut() {
        if *interaction == Interaction::Pressed {
            if let Ok(window) = windows.single() {
                let mut positions = HashMap::new();
                crate::map::calculate_sector_positions(
                    &sector_map,
                    &mut positions,
                    window.width(),
                    window.height(),
                );
                
                if let Some(&current_pos) = positions.get(&sector_map.current_sector_id) {
                    if let Ok(camera_transform) = camera_query.single() {
                        // Start smooth animation
                        camera_animation.is_animating = true;
                        camera_animation.start_pos = Vec2::new(
                            camera_transform.translation.x,
                            camera_transform.translation.y,
                        );
                        camera_animation.target_pos = current_pos;
                        camera_animation.progress = 0.0;
                        camera_animation.duration = 0.8;
                        camera_animation.elapsed = 0.0;
                    }
                }
            }
        }
    }

    // Handle zoom in button
    for interaction in zoom_in_btn.iter_mut() {
        if *interaction == Interaction::Pressed {
            if let Ok(pan_cam) = pan_cam_query.single() {
                if let Ok(mut camera_transform) = camera_query.single_mut() {
                    let current_scale = camera_transform.scale.x;
                    let new_scale = (current_scale * 1.2).min(pan_cam.max_scale);
                    camera_transform.scale = Vec3::splat(new_scale);
                }
            }
        }
    }

    // Handle zoom out button
    for interaction in zoom_out_btn.iter_mut() {
        if *interaction == Interaction::Pressed {
            if let Ok(pan_cam) = pan_cam_query.single() {
                if let Ok(mut camera_transform) = camera_query.single_mut() {
                    let current_scale = camera_transform.scale.x;
                    let new_scale = (current_scale / 1.2).max(pan_cam.min_scale);
                    camera_transform.scale = Vec3::splat(new_scale);
                }
            }
        }
    }
}

// ============================================
// UPDATE FUNCTIONS - Informative text
// ============================================

fn update_hud(
    mut hud_query: Query<&mut TextSpan, With<HudText>>,
    game_data: Res<GameData>,
    sector_map: Res<crate::sector::SectorMap>,
) {
    if let Ok(mut span) = hud_query.single_mut() {
        **span = format!(
            "Fuel: {:.1} | Scrap: {} | Distance: {}",
            game_data.fuel,
            game_data.scrap,
            sector_map.distance_traveled
        );
    }
}

fn update_event_ui(
    mut event_query: Query<&mut Text, With<EventText>>,
    active_event: Res<ActiveEvent>,
) {
    if let Ok(mut text) = event_query.single_mut() {
        if let Some(event) = &active_event.event {
            let mut event_text = format!("{}\n{}\n\nChoices:\n", event.title, event.description);
            
            for (i, choice) in event.choices.iter().enumerate() {
                event_text.push_str(&format!("{}. {}\n", i + 1, choice.text));
            }
            
            *text = Text::new(event_text);
        } else {
            *text = Text::new("");
        }
    }
}

fn update_sector_info(
    mut sector_query: Query<&mut TextSpan, (With<SectorText>, Without<HudText>)>,
    sector_map: Res<crate::sector::SectorMap>,
) {
    if let Ok(mut span) = sector_query.single_mut() {
        if let Some(current_sector) = sector_map.sectors.get(&sector_map.current_sector_id) {
            let mut sector_text = format!(
                "Current Sector: {}\nType: {:?}\n{}\n\nExits: ",
                current_sector.name,
                current_sector.sector_type,
                current_sector.description
            );
            
            // Show available exits
            if current_sector.connections.is_empty() {
                sector_text.push_str("Generating...");
            } else {
                for (i, exit_id) in current_sector.connections.iter().enumerate() {
                    if let Some(exit_sector) = sector_map.sectors.get(exit_id) {
                        sector_text.push_str(&format!("\n{}: {} ({:?})", i + 1, exit_sector.name, exit_sector.sector_type));
                    } else {
                        sector_text.push_str(&format!("\n{}: Unknown Sector", i + 1));
                    }
                }
            }
            
            **span = sector_text;
        } else {
            **span = "Loading sector...".to_string();
        }
    }
}
