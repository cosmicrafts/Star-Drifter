use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use crate::game::{GameState, GameData};
use crate::events::ActiveEvent;
use std::collections::HashMap;

// ============================================
// UI PLUGIN - COSMICRAFTS STAR DRIFTER
// ============================================

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, setup_all_ui)
            .add_systems(Update, (
                update_button_styles,
                handle_all_buttons,
                update_hud,
                update_fps_overlay,
                update_event_ui.run_if(in_state(GameState::Playing)).after(handle_all_buttons),
                update_sector_info.run_if(in_state(GameState::Playing)),
            ));
    }
}

// ============================================
// MARKER COMPONENTS
// ============================================

#[derive(Component)]
struct HudText;

#[derive(Component)]
struct FuelValue;

#[derive(Component)]
struct ScrapValue;

#[derive(Component)]
struct DistanceValue;

#[derive(Component)]
struct SectorText;

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

#[derive(Component)]
pub struct CameraCenterButton;

#[derive(Component)]
pub struct CameraZoomInButton;

#[derive(Component)]
pub struct CameraZoomOutButton;

#[derive(Component)]
struct FpsText;

// ============================================
// THEME - Cosmic Space Aesthetic
// ============================================

// Deep space dark theme with cyan/blue accents
const PANEL_BG: Color = Color::srgba(0.04, 0.06, 0.12, 0.92);
const PANEL_BG_LIGHT: Color = Color::srgba(0.08, 0.10, 0.18, 0.92);
const ACCENT_CYAN: Color = Color::srgb(0.2, 0.8, 0.9);
const ACCENT_CYAN_DIM: Color = Color::srgba(0.2, 0.8, 0.9, 0.6);
const ACCENT_GOLD: Color = Color::srgb(1.0, 0.85, 0.3);
const TEXT_BRIGHT: Color = Color::srgb(0.95, 0.97, 1.0);
const TEXT_DIM: Color = Color::srgb(0.6, 0.65, 0.75);
const SUCCESS_GREEN: Color = Color::srgb(0.3, 0.9, 0.5);
const BTN_NORMAL: Color = Color::srgba(0.1, 0.12, 0.2, 0.9);
const BTN_HOVER: Color = Color::srgba(0.15, 0.25, 0.4, 0.95);
const BTN_PRESS: Color = Color::srgba(0.2, 0.4, 0.6, 1.0);

// ============================================
// HELPER - Create shadow style
// ============================================

fn shadow(alpha: f32, y: f32, blur: f32) -> BoxShadow {
    BoxShadow(vec![ShadowStyle {
        color: Color::BLACK.with_alpha(alpha),
        x_offset: px(0.0),
        y_offset: px(y),
        blur_radius: px(blur),
        spread_radius: px(0.0),
    }])
}

// ============================================
// SETUP - ALL UI ELEMENTS
// ============================================

fn setup_all_ui(mut commands: Commands) {
    setup_hud(&mut commands);
    setup_camera_buttons(&mut commands);
    setup_sector_info(&mut commands);
    setup_event_ui(&mut commands);
    setup_controls_text(&mut commands);
    setup_fps_overlay(&mut commands);
}

fn setup_hud(commands: &mut Commands) {
    commands.spawn((
        HudText,
        Node {
            position_type: PositionType::Absolute,
            top: px(16.0),
            left: px(16.0),
            padding: UiRect::axes(px(20.0), px(14.0)),
            border: UiRect::all(px(2.0)),
            ..default()
        },
        BorderRadius::all(px(8.0)),
        BackgroundGradient::from(LinearGradient {
            angle: std::f32::consts::FRAC_PI_2,
            stops: vec![PANEL_BG.into(), PANEL_BG_LIGHT.into()],
            ..default()
        }),
        BorderColor::all(ACCENT_CYAN_DIM),
        shadow(0.7, 4.0, 12.0),
    )).with_children(|p| {
        // Create a flex row for the HUD items
        p.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: px(16.0),
            align_items: AlignItems::Center,
            ..default()
        }).with_children(|row| {
            // Fuel icon + value
            row.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(8.0),
                align_items: AlignItems::Center,
                ..default()
            }).with_children(|fuel| {
                fuel.spawn((
                    Node {
                        width: px(50.0),
                        height: px(24.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(px(1.0)),
                        ..default()
                    },
                    BackgroundColor(ACCENT_GOLD.with_alpha(0.2)),
                    BorderColor::all(ACCENT_GOLD),
                    BorderRadius::all(px(4.0)),
                )).with_children(|icon| {
                    icon.spawn((
                        Text::new("Fuel"),
                        TextFont { font_size: 14.0, ..default() },
                        TextColor(ACCENT_GOLD),
                        TextShadow { color: Color::BLACK.with_alpha(0.8), offset: Vec2::splat(1.5) },
                    ));
                });
                fuel.spawn((
                    FuelValue,
                    Text::new("50"),
                    TextFont { font_size: 22.0, ..default() },
                    TextColor(TEXT_BRIGHT),
                    TextShadow { color: Color::BLACK.with_alpha(0.8), offset: Vec2::splat(1.5) },
                ));
            });
            
            // Separator
            row.spawn((
                Text::new("|"),
                TextFont { font_size: 18.0, ..default() },
                TextColor(TEXT_DIM),
            ));
            
            // Scrap icon + value
            row.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(8.0),
                align_items: AlignItems::Center,
                ..default()
            }).with_children(|scrap| {
                scrap.spawn((
                    Node {
                        width: px(55.0),
                        height: px(24.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(px(1.0)),
                        ..default()
                    },
                    BackgroundColor(ACCENT_CYAN.with_alpha(0.2)),
                    BorderColor::all(ACCENT_CYAN),
                    BorderRadius::all(px(4.0)),
                )).with_children(|icon| {
                    icon.spawn((
                        Text::new("Scrap"),
                        TextFont { font_size: 14.0, ..default() },
                        TextColor(ACCENT_CYAN),
                        TextShadow { color: Color::BLACK.with_alpha(0.8), offset: Vec2::splat(1.5) },
                    ));
                });
                scrap.spawn((
                    ScrapValue,
                    Text::new("15"),
                    TextFont { font_size: 22.0, ..default() },
                    TextColor(TEXT_BRIGHT),
                    TextShadow { color: Color::BLACK.with_alpha(0.8), offset: Vec2::splat(1.5) },
                ));
            });
            
            // Separator
            row.spawn((
                Text::new("|"),
                TextFont { font_size: 18.0, ..default() },
                TextColor(TEXT_DIM),
            ));
            
            // Distance icon + value
            row.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(8.0),
                align_items: AlignItems::Center,
                ..default()
            }).with_children(|dist| {
                dist.spawn((
                    Node {
                        width: px(45.0),
                        height: px(24.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(px(1.0)),
                        ..default()
                    },
                    BackgroundColor(SUCCESS_GREEN.with_alpha(0.2)),
                    BorderColor::all(SUCCESS_GREEN),
                    BorderRadius::all(px(4.0)),
                )).with_children(|icon| {
                    icon.spawn((
                        Text::new("Dist"),
                        TextFont { font_size: 14.0, ..default() },
                        TextColor(SUCCESS_GREEN),
                        TextShadow { color: Color::BLACK.with_alpha(0.8), offset: Vec2::splat(1.5) },
                    ));
                });
                dist.spawn((
                    DistanceValue,
                    Text::new("0"),
                    TextFont { font_size: 22.0, ..default() },
                    TextColor(TEXT_BRIGHT),
                    TextShadow { color: Color::BLACK.with_alpha(0.8), offset: Vec2::splat(1.5) },
                ));
            });
        });
    });
}

fn setup_camera_buttons(commands: &mut Commands) {
    commands.spawn(Node {
        position_type: PositionType::Absolute,
        top: px(16.0),
        right: px(16.0),
        flex_direction: FlexDirection::Column,
        row_gap: px(8.0),
        ..default()
    }).with_children(|p| {
        // Center button
        spawn_cam_button(p, CameraCenterButton, "C");
        // Zoom in button
        spawn_cam_button(p, CameraZoomInButton, "+");
        // Zoom out button
        spawn_cam_button(p, CameraZoomOutButton, "-");
    });
}

fn spawn_cam_button(p: &mut ChildSpawnerCommands, marker: impl Component, icon: &str) {
    p.spawn((
        marker,
        Button,
        Node {
            width: px(48.0),
            height: px(48.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(px(2.0)),
            ..default()
        },
        BorderRadius::all(px(10.0)),
        BackgroundColor(BTN_NORMAL),
        BorderColor::all(ACCENT_CYAN_DIM),
        shadow(0.5, 3.0, 8.0),
    )).with_children(|btn| {
        btn.spawn((
            Text::new(icon),
            TextFont { font_size: 22.0, ..default() },
            TextColor(ACCENT_CYAN),
            TextShadow::default(),
        ));
    });
}

fn setup_sector_info(commands: &mut Commands) {
    commands.spawn((
        SectorText,
        Node {
            position_type: PositionType::Absolute,
            bottom: px(60.0),
            left: px(16.0),
            max_width: px(400.0),
            padding: UiRect::all(px(16.0)),
            border: UiRect::all(px(2.0)),
            ..default()
        },
        GlobalZIndex(10),
        BorderRadius::all(px(10.0)),
        BackgroundGradient::from(LinearGradient {
            angle: 0.0,
            stops: vec![PANEL_BG.into(), PANEL_BG_LIGHT.into()],
            ..default()
        }),
        BorderColor::all(ACCENT_CYAN_DIM),
        shadow(0.6, 4.0, 10.0),
    )).with_children(|p| {
        p.spawn((
            Text::new("Scanning sector..."),
            TextFont { font_size: 16.0, ..default() },
            TextColor(TEXT_BRIGHT),
            TextShadow { color: Color::BLACK.with_alpha(0.7), offset: Vec2::splat(1.0) },
        ));
    });
}

fn setup_event_ui(commands: &mut Commands) {
    commands.spawn((
        EventPanel,
        Node {
            position_type: PositionType::Absolute,
            bottom: px(80.0),
            left: px(80.0),
            right: px(80.0),
            max_height: px(500.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(px(28.0)),
            row_gap: px(16.0),
            border: UiRect::all(px(3.0)),
            ..default()
        },
        GlobalZIndex(100),
        BorderRadius::all(px(16.0)),
        BackgroundGradient::from(LinearGradient {
            angle: std::f32::consts::FRAC_PI_4,
            stops: vec![
                PANEL_BG.into(),
                Color::srgba(0.06, 0.08, 0.14, 0.95).into(),
                PANEL_BG.into(),
            ],
            ..default()
        }),
        BorderColor::all(ACCENT_CYAN),
        shadow(0.8, 8.0, 24.0),
        Visibility::Hidden,
    )).with_children(|panel| {
        // Title
        panel.spawn((
            EventTitle,
            Text::new(""),
            TextFont { font_size: 28.0, ..default() },
            TextColor(ACCENT_GOLD),
            TextShadow { color: Color::BLACK.with_alpha(0.9), offset: Vec2::new(2.0, 2.0) },
            Node { margin: UiRect::bottom(px(8.0)), ..default() },
        ));

        // Description
        panel.spawn((
            EventDescription,
            Text::new(""),
            TextFont { font_size: 17.0, ..default() },
            TextColor(TEXT_DIM),
            TextShadow { color: Color::BLACK.with_alpha(0.6), offset: Vec2::splat(1.0) },
            Node { margin: UiRect::bottom(px(20.0)), ..default() },
        ));

        // Choice buttons container
        panel.spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(12.0),
            ..default()
        }).with_children(|choices| {
            for i in 0..4 {
                choices.spawn((
                    EventChoiceButton { choice_index: i },
                    Button,
                    Node {
                        width: percent(100.0),
                        min_height: px(56.0),
                        padding: UiRect::axes(px(18.0), px(14.0)),
                        align_items: AlignItems::Center,
                        border: UiRect::all(px(2.0)),
                        ..default()
                    },
                    BorderRadius::all(px(10.0)),
                    BackgroundColor(BTN_NORMAL),
                    BorderColor::all(Color::srgba(0.3, 0.4, 0.5, 0.8)),
                    shadow(0.4, 2.0, 6.0),
                    Visibility::Hidden,
                )).with_children(|btn| {
                    // Number badge
                    btn.spawn((
                        Text::new(format!("{}", i + 1)),
                        TextFont { font_size: 18.0, ..default() },
                        TextColor(ACCENT_GOLD),
                        TextShadow::default(),
                        Node { 
                            width: px(28.0),
                            margin: UiRect::right(px(16.0)),
                            ..default() 
                        },
                    ));
                    // Choice text
                    btn.spawn((
                        Text::new(""),
                        TextFont { font_size: 15.0, ..default() },
                        TextColor(TEXT_BRIGHT),
                        TextShadow { color: Color::BLACK.with_alpha(0.5), offset: Vec2::splat(1.0) },
                    ));
                });
            }
        });
    });
}

fn setup_controls_text(commands: &mut Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: px(16.0),
            left: percent(50.0),
            padding: UiRect::axes(px(16.0), px(8.0)),
            ..default()
        },
        BorderRadius::all(px(6.0)),
        BackgroundColor(Color::BLACK.with_alpha(0.5)),
    )).with_children(|p| {
        p.spawn((
            Text::new("1-9: Travel  •  Click: Select  •  ESC: Pause"),
            TextFont { font_size: 13.0, ..default() },
            TextColor(TEXT_DIM),
            TextShadow::default(),
        ));
    });
}

fn setup_fps_overlay(commands: &mut Commands) {
    commands.spawn((
        FpsText,
        Node {
            position_type: PositionType::Absolute,
            top: px(80.0),
            right: px(16.0),
            padding: UiRect::axes(px(12.0), px(6.0)),
            ..default()
        },
        BorderRadius::all(px(6.0)),
        BackgroundColor(Color::BLACK.with_alpha(0.6)),
    )).with_children(|p| {
        p.spawn((
            Text::new("FPS: --"),
            TextFont { font_size: 14.0, ..default() },
            TextColor(SUCCESS_GREEN),
            TextShadow::default(),
        ));
    });
}

// ============================================
// UPDATE SYSTEMS
// ============================================

fn update_fps_overlay(
    fps_query: Query<&Children, With<FpsText>>,
    mut text_query: Query<&mut Text>,
    diagnostics: Res<DiagnosticsStore>,
    time: Res<Time>,
) {
    if let Ok(children) = fps_query.single() {
        if let Some(&entity) = children.first() {
            if let Ok(mut text) = text_query.get_mut(entity) {
                let fps = diagnostics
                    .get(&FrameTimeDiagnosticsPlugin::FPS)
                    .and_then(|d| d.smoothed())
                    .unwrap_or((1.0 / time.delta_secs()) as f64);
                *text = Text::new(format!("FPS: {:.0}", fps));
            }
        }
    }
}

fn update_button_styles(
    mut buttons: ParamSet<(
        Query<(&Interaction, &mut BackgroundColor, &mut BorderColor),
              (Changed<Interaction>, Or<(With<CameraCenterButton>, With<CameraZoomInButton>, With<CameraZoomOutButton>)>)>,
        Query<(&Interaction, &mut BackgroundColor, &mut BorderColor),
              (Changed<Interaction>, With<EventChoiceButton>)>,
    )>,
) {
    // Camera buttons
    for (interaction, mut bg, mut border) in buttons.p0().iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *bg = BTN_PRESS.into();
                *border = BorderColor::all(ACCENT_CYAN);
            }
            Interaction::Hovered => {
                *bg = BTN_HOVER.into();
                *border = BorderColor::all(ACCENT_CYAN);
            }
            Interaction::None => {
                *bg = BTN_NORMAL.into();
                *border = BorderColor::all(ACCENT_CYAN_DIM);
            }
        }
    }

    // Event choice buttons
    for (interaction, mut bg, mut border) in buttons.p1().iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *bg = BTN_PRESS.into();
                *border = BorderColor::all(ACCENT_CYAN);
            }
            Interaction::Hovered => {
                *bg = BTN_HOVER.into();
                *border = BorderColor::all(ACCENT_CYAN_DIM);
            }
            Interaction::None => {
                *bg = BTN_NORMAL.into();
                *border = BorderColor::all(Color::srgba(0.3, 0.4, 0.5, 0.8));
            }
        }
    }
}

fn handle_all_buttons(
    center_btn: Query<&Interaction, (Changed<Interaction>, With<CameraCenterButton>)>,
    zoom_in_btn: Query<&Interaction, (Changed<Interaction>, With<CameraZoomInButton>)>,
    zoom_out_btn: Query<&Interaction, (Changed<Interaction>, With<CameraZoomOutButton>)>,
    event_btns: Query<(&Interaction, &EventChoiceButton), (Changed<Interaction>, With<EventChoiceButton>)>,
    mut camera_query: Query<&mut Transform, (With<Camera2d>, Without<Button>)>,
    pan_cam: Query<&bevy_pancam::PanCam>,
    sector_map: Res<crate::sector::SectorMap>,
    mut camera_anim: ResMut<crate::camera::CameraAnimation>,
    windows: Query<&Window>,
    mut active_event: ResMut<crate::events::ActiveEvent>,
    mut game_data: ResMut<GameData>,
) {
    // Center button
    for interaction in center_btn.iter() {
        if *interaction == Interaction::Pressed {
            if let Ok(window) = windows.single() {
                let mut positions = HashMap::new();
                crate::map::calculate_sector_positions(&sector_map, &mut positions, window.width(), window.height());
                if let Some(&pos) = positions.get(&sector_map.current_sector_id) {
                    if let Ok(transform) = camera_query.single() {
                        camera_anim.is_animating = true;
                        camera_anim.start_pos = Vec2::new(transform.translation.x, transform.translation.y);
                        camera_anim.target_pos = pos;
                        camera_anim.progress = 0.0;
                        camera_anim.duration = 0.6;
                        camera_anim.elapsed = 0.0;
                    }
                }
            }
        }
    }

    // Zoom buttons
    if let Ok(pancam) = pan_cam.single() {
        if let Ok(mut transform) = camera_query.single_mut() {
            for interaction in zoom_in_btn.iter() {
                if *interaction == Interaction::Pressed {
                    transform.scale = Vec3::splat((transform.scale.x * 1.25).min(pancam.max_scale));
                }
            }
            for interaction in zoom_out_btn.iter() {
                if *interaction == Interaction::Pressed {
                    transform.scale = Vec3::splat((transform.scale.x / 1.25).max(pancam.min_scale));
                }
            }
        }
    }

    // Event choice buttons
    for (interaction, choice) in event_btns.iter() {
        if *interaction == Interaction::Pressed {
            crate::events::process_event_choice(choice.choice_index, &mut active_event, &mut game_data);
        }
    }
}

fn update_hud(
    mut text_params: ParamSet<(
        Query<&mut Text, With<FuelValue>>,
        Query<&mut Text, With<ScrapValue>>,
        Query<&mut Text, With<DistanceValue>>,
    )>,
    game_data: Res<GameData>,
    sector_map: Res<crate::sector::SectorMap>,
) {
    if let Ok(mut text) = text_params.p0().single_mut() {
        *text = Text::new(format!("{:.0}", game_data.fuel));
    }
    if let Ok(mut text) = text_params.p1().single_mut() {
        *text = Text::new(format!("{}", game_data.scrap));
    }
    if let Ok(mut text) = text_params.p2().single_mut() {
        *text = Text::new(format!("{}", sector_map.distance_traveled));
    }
}

fn update_event_ui(
    active_event: Res<ActiveEvent>,
    game_data: Res<GameData>,
    mut vis_params: ParamSet<(
        Query<&mut Visibility, With<EventPanel>>,
        Query<&mut Visibility, With<EventChoiceButton>>,
    )>,
    choice_btns: Query<(Entity, &EventChoiceButton, &Children), Without<EventPanel>>,
    mut text_params: ParamSet<(
        Query<&mut Text, With<EventTitle>>,
        Query<&mut Text, With<EventDescription>>,
        Query<&mut Text>,
    )>,
) {
    // Toggle panel visibility
    if let Ok(mut vis) = vis_params.p0().single_mut() {
        if active_event.event.is_some() {
            *vis = Visibility::Visible;
        } else {
            *vis = Visibility::Hidden;
            for (entity, _, _) in choice_btns.iter() {
                if let Ok(mut v) = vis_params.p1().get_mut(entity) {
                    *v = Visibility::Hidden;
                }
            }
            return;
        }
    }

    if let Some(event) = &active_event.event {
        // Update title
        if let Ok(mut title) = text_params.p0().single_mut() {
            *title = Text::new(event.title.clone());
        }

        // Update description
        if let Ok(mut desc) = text_params.p1().single_mut() {
            *desc = Text::new(event.description.clone());
        }

        // Update choice buttons
        for (entity, choice, children) in choice_btns.iter() {
            if choice.choice_index < event.choices.len() {
                let c = &event.choices[choice.choice_index];
                if let Ok(mut v) = vis_params.p1().get_mut(entity) {
                    *v = Visibility::Visible;
                }
                let can_choose = crate::events::check_requirements(&c.requirements, &game_data);
                for (i, child) in children.iter().enumerate() {
                    if i == 1 {
                        if let Ok(mut text) = text_params.p2().get_mut(child) {
                            let mut s = c.text.clone();
                            if !can_choose {
                                s.push_str(" ⚠️");
                            }
                            *text = Text::new(s);
                        }
                    }
                }
            } else if let Ok(mut v) = vis_params.p1().get_mut(entity) {
                *v = Visibility::Hidden;
            }
        }
    }
}

fn update_sector_info(
    sector: Query<&Children, (With<SectorText>, Without<HudText>)>,
    mut text_query: Query<&mut Text>,
    sector_map: Res<crate::sector::SectorMap>,
) {
    if let Ok(children) = sector.single() {
        if let Some(&entity) = children.first() {
            if let Ok(mut text) = text_query.get_mut(entity) {
                let content = if let Some(s) = sector_map.sectors.get(&sector_map.current_sector_id) {
                    let mut t = format!("LOC: {}\nTYPE: {:?}\n{}\n\nEXITS:", s.name, s.sector_type, s.description);
                    if s.connections.is_empty() {
                        t.push_str(" Scanning...");
                    } else {
                        for (i, id) in s.connections.iter().enumerate() {
                            if let Some(exit) = sector_map.sectors.get(id) {
                                t.push_str(&format!("\n  {} → {} ({:?})", i + 1, exit.name, exit.sector_type));
                            }
                        }
                    }
                    t
                } else {
                    "Initializing...".to_string()
                };
                *text = Text::new(content);
            }
        }
    }
}
