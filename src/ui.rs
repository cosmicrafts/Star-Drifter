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
                // Interactive buttons (process first)
                handle_all_buttons,
                // Informative text elements (update after button processing)
                update_hud,
                update_event_ui.run_if(in_state(GameState::Playing)).after(handle_all_buttons),
                update_sector_info.run_if(in_state(GameState::Playing)),
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
struct SectorText;

// Event panel components
#[derive(Component)]
struct EventPanel;

#[derive(Component)]
struct EventTitle;

#[derive(Component)]
struct EventDescription;

#[derive(Component)]
pub struct EventChoiceButton {
    pub choice_index: usize,
}

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
    // Main event panel (FTL style - centered at bottom)
    commands.spawn((
        EventPanel,
        Node {
            position_type: PositionType::Absolute,
            bottom: px(50.0),
            left: px(50.0),
            right: px(50.0),
            height: px(400.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(px(20.0)),
            row_gap: px(15.0),
            ..default()
        },
        BackgroundColor(Color::srgb(0.1, 0.1, 0.15)),
        Visibility::Hidden, // Hidden by default, shown when event is active
    )).with_children(|parent| {
        // Event title
        parent.spawn((
            EventTitle,
            Text::new(""),
            TextFont {
                font_size: 28.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.9, 0.7)),
            Node {
                margin: UiRect::bottom(px(10.0)),
                ..default()
            },
        ));

        // Event description
        parent.spawn((
            EventDescription,
            Text::new(""),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 0.9)),
            Node {
                margin: UiRect::bottom(px(20.0)),
                ..default()
            },
        ));

        // Container for choice buttons
        parent.spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(10.0),
                ..default()
            },
        )).with_children(|parent| {
            // Create buttons for choices (max 4 choices)
            for i in 0..4 {
                parent.spawn((
                    EventChoiceButton { choice_index: i },
                    Button,
                    Node {
                        width: percent(100.0),
                        height: px(50.0),
                        padding: UiRect::all(px(10.0)),
                        justify_content: JustifyContent::FlexStart,
                        align_items: AlignItems::Center,
                        border: UiRect::all(px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.2, 0.3)),
                    Visibility::Hidden, // Hidden by default
                )).with_children(|parent| {
                    // Button number label
                    parent.spawn((
                        Text::new(format!("{}", i + 1)),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.8, 0.0)),
                        Node {
                            width: px(30.0),
                            margin: UiRect::right(px(15.0)),
                            ..default()
                        },
                    ));
                    // Button text
                    parent.spawn((
                        Text::new(""),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
            }
        });
    });
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
    // Event choice buttons
    mut event_choice_buttons: Query<(&Interaction, &EventChoiceButton), (Changed<Interaction>, With<EventChoiceButton>)>,
    // Camera resources and queries
    mut camera_query: Query<&mut Transform, (With<Camera2d>, Without<Button>)>,
    pan_cam_query: Query<&bevy_pancam::PanCam>,
    sector_map: Res<crate::sector::SectorMap>,
    mut camera_animation: ResMut<crate::camera::CameraAnimation>,
    windows: Query<&Window>,
    // Event resources
    mut active_event: ResMut<crate::events::ActiveEvent>,
    mut game_data: ResMut<GameData>,
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

    // Handle event choice buttons
    for (interaction, choice_button) in event_choice_buttons.iter_mut() {
        if *interaction == Interaction::Pressed {
            crate::events::process_event_choice(
                choice_button.choice_index,
                &mut active_event,
                &mut game_data,
            );
        }
    }
}

// ============================================
// UPDATE FUNCTIONS - Informative text
// ============================================

fn update_hud(
    mut hud_query: Query<&mut Text, With<HudText>>,
    game_data: Res<GameData>,
    sector_map: Res<crate::sector::SectorMap>,
) {
    if let Ok(mut text) = hud_query.single_mut() {
        *text = Text::new(format!(
            "Fuel: {:.0} | Scrap: {} | Distance: {}",
            game_data.fuel,
            game_data.scrap,
            sector_map.distance_traveled
        ));
    }
}

fn update_event_ui(
    active_event: Res<ActiveEvent>,
    game_data: Res<GameData>,
    mut visibility_params: ParamSet<(
        Query<&mut Visibility, With<EventPanel>>,
        Query<&mut Visibility, With<EventChoiceButton>>,
    )>,
    mut choice_button_query: Query<(Entity, &EventChoiceButton, &Children), Without<EventPanel>>,
    mut text_params: ParamSet<(
        Query<&mut Text, With<EventTitle>>,
        Query<&mut Text, With<EventDescription>>,
        Query<&mut Text>,
    )>,
) {
    // Show/hide panel based on active event
    if let Ok(mut panel_visibility) = visibility_params.p0().single_mut() {
        if active_event.event.is_some() {
            *panel_visibility = Visibility::Visible;
        } else {
            // Hide panel and all buttons when no event is active
            *panel_visibility = Visibility::Hidden;
            for (button_entity, _, _) in choice_button_query.iter() {
                if let Ok(mut button_visibility) = visibility_params.p1().get_mut(button_entity) {
                    *button_visibility = Visibility::Hidden;
                }
            }
            return;
        }
    }

    if let Some(event) = &active_event.event {
        // Update title (using ParamSet to avoid query conflicts)
        if let Ok(mut title) = text_params.p0().single_mut() {
            *title = Text::new(event.title.clone());
        }

        // Update description
        if let Ok(mut description) = text_params.p1().single_mut() {
            *description = Text::new(event.description.clone());
        }

        // Update choice buttons
        for (button_entity, choice_button, children) in choice_button_query.iter_mut() {
            if choice_button.choice_index < event.choices.len() {
                let choice = &event.choices[choice_button.choice_index];
                
                // Show button (using ParamSet to avoid query conflicts)
                if let Ok(mut button_visibility) = visibility_params.p1().get_mut(button_entity) {
                    *button_visibility = Visibility::Visible;
                }

                // Check if choice is available (requirements met)
                let can_choose = crate::events::check_requirements(&choice.requirements, &game_data);

                // Update button text (second child is the text)
                for (i, child) in children.iter().enumerate() {
                    if i == 1 {
                        if let Ok(mut text) = text_params.p2().get_mut(child) {
                            let mut choice_text = choice.text.clone();
                            if !can_choose {
                                choice_text.push_str(" (Requirements not met)");
                            }
                            *text = Text::new(choice_text);
                        }
                    }
                }
            } else {
                // Hide button if no choice at this index
                if let Ok(mut button_visibility) = visibility_params.p1().get_mut(button_entity) {
                    *button_visibility = Visibility::Hidden;
                }
            }
        }
    } else {
        // Hide all buttons when no event is active
        for (button_entity, _, _) in choice_button_query.iter() {
            if let Ok(mut button_visibility) = visibility_params.p1().get_mut(button_entity) {
                *button_visibility = Visibility::Hidden;
            }
        }
    }
}

fn update_sector_info(
    mut sector_query: Query<&mut Text, (With<SectorText>, Without<HudText>)>,
    sector_map: Res<crate::sector::SectorMap>,
) {
    if let Ok(mut text) = sector_query.single_mut() {
        let sector_text = if let Some(current_sector) = sector_map.sectors.get(&sector_map.current_sector_id) {
            let mut text = format!(
                "Current Sector: {}\nType: {:?}\n{}\n\nExits: ",
                current_sector.name,
                current_sector.sector_type,
                current_sector.description
            );
            
            // Show available exits
            if current_sector.connections.is_empty() {
                text.push_str("Generating...");
            } else {
                for (i, exit_id) in current_sector.connections.iter().enumerate() {
                    if let Some(exit_sector) = sector_map.sectors.get(exit_id) {
                        text.push_str(&format!("\n{}: {} ({:?})", i + 1, exit_sector.name, exit_sector.sector_type));
                    } else {
                        text.push_str(&format!("\n{}: Unknown Sector", i + 1));
                    }
                }
            }
            
            text
        } else {
            "Loading sector...".to_string()
        };
        
        *text = Text::new(sector_text);
    }
}
