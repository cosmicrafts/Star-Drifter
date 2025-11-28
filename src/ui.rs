use bevy::prelude::*;
use crate::game::{GameState, GameData};
use crate::events::ActiveEvent;

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, setup_ui)
            .add_systems(Update, (
                update_hud,
                update_event_ui.run_if(in_state(GameState::Playing)),
                update_sector_info.run_if(in_state(GameState::Playing)),
            ));
    }
}

#[derive(Component)]
struct HudText;

#[derive(Component)]
struct EventText;

#[derive(Component)]
struct SectorText;

fn setup_ui(mut commands: Commands) {
    // HUD Elements
    commands.spawn((
        HudText,
        Text::new("Fuel: 16 | Scrap: 15 | Sector: 1/30"),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: px(10.0),
            left: px(10.0),
            ..default()
        },
    ));

    // Sector Info
    commands.spawn((
        SectorText,
        Text::new("Current Sector: Loading..."),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::srgb(0.8, 0.8, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(80.0),
            left: px(10.0),
            ..default()
        },
    ));

    // Controls
    commands.spawn((
        Text::new("Controls: 1-9 - Travel to Exit | Click Node - Travel | 1-3 - Event Choices | ESC - Pause"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(0.7, 0.7, 0.7)),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(10.0),
            left: px(10.0),
            ..default()
        },
    ));

    // Event UI - abajo a la derecha
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
