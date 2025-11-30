use bevy::prelude::*;
use bevy_pancam::{PanCam, PanCamPlugin};
use std::collections::HashMap;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(PanCamPlugin::default())
            .insert_resource(CameraAnimation::default())
            .add_systems(Startup, (setup_map_camera, setup_camera_controls))
            .add_systems(Update, (
                handle_camera_buttons,
                update_camera_animation,
            ));
    }
}

#[derive(Resource, Default)]
struct CameraAnimation {
    is_animating: bool,
    start_pos: Vec2,
    target_pos: Vec2,
    progress: f32,
    duration: f32,
    elapsed: f32,
}

#[derive(Component)]
struct CenterButton;

#[derive(Component)]
struct ZoomInButton;

#[derive(Component)]
struct ZoomOutButton;

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

fn setup_camera_controls(mut commands: Commands) {
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
        parent.spawn((
            CenterButton,
            Button,
            Node {
                width: px(50.0),
                height: px(50.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
        )).with_children(|parent| {
            parent.spawn((
                Text::new("📍"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });

        // Zoom in button
        parent.spawn((
            ZoomInButton,
            Button,
            Node {
                width: px(50.0),
                height: px(50.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
        )).with_children(|parent| {
            parent.spawn((
                Text::new("+"),
                TextFont {
                    font_size: 32.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });

        // Zoom out button
        parent.spawn((
            ZoomOutButton,
            Button,
            Node {
                width: px(50.0),
                height: px(50.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
        )).with_children(|parent| {
            parent.spawn((
                Text::new("−"),
                TextFont {
                    font_size: 32.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
    });
}

fn handle_camera_buttons(
    mut center_button_query: Query<&Interaction, (Changed<Interaction>, With<CenterButton>)>,
    mut zoom_in_button_query: Query<&Interaction, (Changed<Interaction>, With<ZoomInButton>)>,
    mut zoom_out_button_query: Query<&Interaction, (Changed<Interaction>, With<ZoomOutButton>)>,
    mut camera_query: Query<&mut Transform, (With<Camera2d>, Without<Button>)>,
    pan_cam_query: Query<&PanCam>,
    sector_map: Res<crate::sector::SectorMap>,
    mut camera_animation: ResMut<CameraAnimation>,
    windows: Query<&Window>,
) {
    // Handle center button
    for interaction in center_button_query.iter_mut() {
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
                    if let Ok(mut camera_transform) = camera_query.single_mut() {
                        // Start smooth animation
                        camera_animation.is_animating = true;
                        camera_animation.start_pos = Vec2::new(
                            camera_transform.translation.x,
                            camera_transform.translation.y,
                        );
                        camera_animation.target_pos = current_pos;
                        camera_animation.progress = 0.0;
                        camera_animation.duration = 0.8; // Animation duration in seconds
                        camera_animation.elapsed = 0.0;
                    }
                }
            }
        }
    }

    // Handle zoom in button
    for interaction in zoom_in_button_query.iter_mut() {
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
    for interaction in zoom_out_button_query.iter_mut() {
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

