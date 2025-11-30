use bevy::prelude::*;
use bevy_pancam::{PanCam, PanCamPlugin};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(PanCamPlugin::default())
            .insert_resource(CameraAnimation::default())
            .add_systems(Startup, setup_map_camera)
            .add_systems(Update, update_camera_animation);
    }
}

#[derive(Resource, Default)]
pub struct CameraAnimation {
    pub is_animating: bool,
    pub start_pos: Vec2,
    pub target_pos: Vec2,
    pub progress: f32,
    pub duration: f32,
    pub elapsed: f32,
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


fn update_camera_animation(
    mut camera_query: Query<&mut Transform, (With<Camera2d>, Without<Button>)>,
    mut camera_animation: ResMut<CameraAnimation>,
    time: Res<Time>,
) {
    if !camera_animation.is_animating {
        return;
    }

    camera_animation.elapsed += time.delta_secs();
    camera_animation.progress = (camera_animation.elapsed / camera_animation.duration).min(1.0);

    // Ease-out cubic for smooth deceleration (like Google Maps)
    let eased_progress = 1.0 - (1.0 - camera_animation.progress).powi(3);

    if let Ok(mut camera_transform) = camera_query.single_mut() {
        let current_pos = camera_animation.start_pos.lerp(
            camera_animation.target_pos,
            eased_progress,
        );

        camera_transform.translation.x = current_pos.x;
        camera_transform.translation.y = current_pos.y;

        // Stop animation when complete
        if camera_animation.progress >= 1.0 {
            camera_animation.is_animating = false;
            camera_transform.translation.x = camera_animation.target_pos.x;
            camera_transform.translation.y = camera_animation.target_pos.y;
        }
    }
}

