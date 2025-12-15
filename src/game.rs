use bevy::prelude::*;
use crate::{factions::{FactionsPlugin, PlayerFaction}, ship::ShipPlugin, sector::SectorPlugin, map::MapPlugin, camera::CameraPlugin, events::EventsPlugin, ui::UIPlugin, llm::LlmPlugin};

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app
            .init_state::<GameState>()
            .add_plugins((
                FactionsPlugin,
                ShipPlugin,
                SectorPlugin,
                MapPlugin,
                CameraPlugin,
                EventsPlugin,
                UIPlugin,
                LlmPlugin,
            ))
            .add_systems(Startup, setup_game)
            .add_systems(Update, (
                handle_input,
                update_game_state,
            ))
            .add_systems(OnEnter(GameState::FactionSelection), reset_game_on_restart);
    }
}

/// Game State Management
/// 
/// STATES:
/// - FactionSelection: Initial state - player selects their faction
///   - Only UI systems run (faction selection)
///   - All game systems disabled
/// 
/// - Playing: Normal gameplay state
///   - All game systems active (navigation, events, map, ship, etc.)
///   - Camera controls active
///   - Input handling active
/// 
/// - Paused: Game paused (ESC key)
///   - Game systems paused (no navigation, events, etc.)
///   - Camera animation continues (so you can look around)
///   - UI systems active (pause menu)
///   - ESC to resume
/// 
/// CONTROL SYSTEMS BY STATE:
/// - Map Navigation (1-9 keys): Playing only
/// - Event Choices (1-4 keys): Playing only
/// - Node Clicks: Playing only
/// - Camera Pan/Zoom: Playing and Paused
/// - Pause Menu: Paused only
/// - Faction Selection: FactionSelection only
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    FactionSelection,
    Playing,
    Paused,
}

#[derive(Resource)]
pub struct GameData {
    pub current_sector: u32,
    pub fuel: f32,
    pub scrap: u32,
}


fn setup_game(mut commands: Commands) {
    // Initialize game data
    commands.insert_resource(GameData {
        current_sector: 0,
        fuel: 50.0,
        scrap: 15,
    });

    // Camera is now handled by CameraPlugin
}

fn handle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    current_state: Res<State<GameState>>,
) {
    match current_state.get() {
        GameState::FactionSelection => {
            // Faction selection handled by UI (to be added)
        }
        GameState::Playing => {
            if keyboard.just_pressed(KeyCode::Escape) {
                next_state.set(GameState::Paused);
            }
        }
        GameState::Paused => {
            if keyboard.just_pressed(KeyCode::Escape) {
                next_state.set(GameState::Playing);
            }
        }
    }
}

fn update_game_state(
    _game_data: ResMut<GameData>,
    _time: Res<Time>,
) {
    // Update game logic here
    // For now, just a placeholder
}

fn reset_game_on_restart(
    game_data: Option<ResMut<GameData>>,
    player_faction: Option<ResMut<PlayerFaction>>,
) {
    // Reset game data to initial values
    if let Some(mut data) = game_data {
        data.current_sector = 0;
        data.fuel = 50.0;
        data.scrap = 15;
    }

    // Reset player faction selection
    if let Some(mut faction) = player_faction {
        faction.faction = None;
    }
}
