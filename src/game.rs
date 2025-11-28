use bevy::prelude::*;
use crate::{factions::FactionsPlugin, ship::ShipPlugin, sector::SectorPlugin, events::EventsPlugin, ui::UIPlugin};

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_state::<GameState>()
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
    MainMenu,
    #[default]
    Playing,
    Paused,
    GameOver,
}

#[derive(Resource)]
pub struct GameData {
    pub current_sector: u32,
    pub fuel: f32,
    pub scrap: u32,
    pub crew: Vec<CrewMember>,
    pub difficulty: u32,
}

#[derive(Component, Clone)]
pub struct CrewMember {
    pub name: String,
    pub faction: crate::factions::Faction,
    pub skills: CrewSkills,
    pub health: f32,
}

#[derive(Clone)]
pub struct CrewSkills {
    pub piloting: u32,
    pub engines: u32,
    pub weapons: u32,
    pub shields: u32,
}

fn setup_game(mut commands: Commands) {
    // Initialize game data
    commands.insert_resource(GameData {
        current_sector: 0,
        fuel: 50.0,
        scrap: 15,
        crew: vec![
            CrewMember {
                name: "Captain Nova".to_string(),
                faction: crate::factions::Faction::Cosmicons,
                skills: CrewSkills {
                    piloting: 2,
                    engines: 1,
                    weapons: 2,
                    shields: 1,
                },
                health: 100.0,
            }
        ],
        difficulty: 1,
    });

    // Spawn camera
    commands.spawn(Camera2dBundle::default());
}

fn handle_input(
    keyboard: Res<Input<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    current_state: Res<State<GameState>>,
) {
    match current_state.get() {
        GameState::MainMenu => {
            if keyboard.just_pressed(KeyCode::Space) {
                next_state.set(GameState::Playing);
            }
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
        _ => {}
    }
}

fn update_game_state(
    _game_data: ResMut<GameData>,
    _time: Res<Time>,
) {
    // Update game logic here
    // For now, just a placeholder
}
