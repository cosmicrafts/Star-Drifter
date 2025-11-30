use bevy::prelude::*;
use bevy_pancam::{PanCam, PanCamPlugin};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(PanCamPlugin::default())
            .add_systems(Startup, setup_map_camera);
    }
}

fn setup_map_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        PanCam {
            grab_buttons: vec![MouseButton::Left],
            zoom_to_cursor: true,
            min_scale: 0.2,
            max_scale: 3.0,
            // No limits on camera position - allow full map exploration
            min_x: f32::NEG_INFINITY,
            max_x: f32::INFINITY,
            min_y: f32::NEG_INFINITY,
            max_y: f32::INFINITY,
            ..default()
        },
    ));
}

