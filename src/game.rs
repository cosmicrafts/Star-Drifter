use bevy::prelude::*;
use crate::{factions::FactionsPlugin, ship::ShipPlugin, sector::SectorPlugin, events::EventsPlugin, ui::UIPlugin};

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app
            .init_state::<GameState>()
            .add_plugins((
                FactionsPlugin,
                ShipPlugin,
                SectorPlugin,
                EventsPlugin,
                UIPlugin,
            ))
            .add_systems(Startup, setup_game)
            .add_systems(Update, (
                handle_input,
                update_game_state,
            ));
    }
}

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
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

    // Spawn camera
    commands.spawn(Camera2d);
}

fn handle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    current_state: Res<State<GameState>>,
) {
    match current_state.get() {
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
