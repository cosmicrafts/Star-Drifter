use bevy::prelude::*;
use bevy_pancam::{PanCam, PanCamPlugin};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(PanCamPlugin::default())
            .insert_resource(CameraAnimation::default())
            .insert_resource(TouchState::default())
            .add_systems(Startup, setup_map_camera)
            .add_systems(Update, (
                update_camera_animation,
                handle_touch_input,
                handle_trackpad_input,
            ));
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

#[derive(Resource, Default)]
struct TouchState {
    last_touch_pos: Option<Vec2>,
    last_pinch_distance: Option<f32>,
}

fn setup_map_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        PanCam {
            grab_buttons: vec![MouseButton::Left],
            zoom_to_cursor: true,
            min_scale: 0.1,
            max_scale: 20.0,
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

fn handle_touch_input(
    mut touch_state: ResMut<TouchState>,
    touches: Res<Touches>,
    mut camera_query: Query<&mut Transform, (With<Camera2d>, Without<Button>)>,
    camera_animation: Res<CameraAnimation>,
) {
    // Don't handle touch during camera animation
    if camera_animation.is_animating {
        return;
    }

    let Ok(mut camera_transform) = camera_query.single_mut() else { return };

    let active_touches: Vec<_> = touches.iter().collect();
    
    match active_touches.len() {
        1 => {
            // Single touch - pan the camera
            let touch = active_touches[0];
            let touch_pos = touch.position();
            
            if let Some(last_pos) = touch_state.last_touch_pos {
                // Calculate pan delta in world space
                let scale = camera_transform.scale.x;
                
                // Convert screen delta to world delta
                let screen_delta = touch_pos - last_pos;
                let world_delta = screen_delta / scale;
                
                // Pan camera in opposite direction of touch movement
                camera_transform.translation.x -= world_delta.x;
                camera_transform.translation.y += world_delta.y; // Y is inverted in screen space
            }
            
            touch_state.last_touch_pos = Some(touch_pos);
        }
        2 => {
            // Two touches - pinch to zoom
            let touch1 = active_touches[0];
            let touch2 = active_touches[1];
            
            let current_distance = touch1.position().distance(touch2.position());
            
            if let Some(last_distance) = touch_state.last_pinch_distance {
                let scale_factor = current_distance / last_distance;
                let new_scale = (camera_transform.scale.x * scale_factor)
                    .clamp(0.1, 20.0);
                camera_transform.scale = Vec3::splat(new_scale);
            }
            
            touch_state.last_pinch_distance = Some(current_distance);
            touch_state.last_touch_pos = None; // Reset pan state
        }
        _ => {
            // Reset state when no touches or too many touches
            touch_state.last_touch_pos = None;
            touch_state.last_pinch_distance = None;
        }
    }
    
    // Reset state when all touches end
    if active_touches.is_empty() {
        touch_state.last_touch_pos = None;
        touch_state.last_pinch_distance = None;
    }
}

fn handle_trackpad_input(
    mut camera_query: Query<&mut Transform, (With<Camera2d>, Without<Button>)>,
    mut scroll_events: MessageReader<bevy::input::mouse::MouseWheel>,
    camera_animation: Res<CameraAnimation>,
) {
    // Don't handle trackpad during camera animation
    if camera_animation.is_animating {
        return;
    }

    let Ok(mut camera_transform) = camera_query.single_mut() else { return };

    for event in scroll_events.read() {
        match event.unit {
            bevy::input::mouse::MouseScrollUnit::Line => {
                // Trackpad pan (two-finger scroll)
                let pan_speed = 50.0;
                let pan_delta = Vec2::new(event.x, -event.y) * pan_speed / camera_transform.scale.x;
                
                camera_transform.translation.x += pan_delta.x;
                camera_transform.translation.y += pan_delta.y;
            }
            bevy::input::mouse::MouseScrollUnit::Pixel => {
                // Trackpad pinch-to-zoom or precise pan
                // Check if it's primarily vertical (zoom) or horizontal (pan)
                if event.y.abs() > event.x.abs() {
                    // Zoom
                    let zoom_factor = 1.0 + (event.y * 0.01);
                    let new_scale = (camera_transform.scale.x * zoom_factor)
                        .clamp(0.1, 20.0);
                    camera_transform.scale = Vec3::splat(new_scale);
                } else {
                    // Pan
                    let pan_delta = Vec2::new(event.x, -event.y) / camera_transform.scale.x;
                    camera_transform.translation.x += pan_delta.x;
                    camera_transform.translation.y += pan_delta.y;
                }
            }
        }
    }
}

