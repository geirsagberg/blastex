#![allow(unused_parens)]
use std::time::Duration;

use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bevy_pixel_camera::{PixelCameraBundle, PixelCameraPlugin};

const WINDOW_WIDTH: f32 = 1024.0;
const WINDOW_HEIGHT: f32 = 768.0;

const SCALE: i32 = 2;

const GAME_WIDTH: f32 = WINDOW_WIDTH / SCALE as f32;
const GAME_HEIGHT: f32 = WINDOW_HEIGHT / SCALE as f32;

fn main() {
    App::new()
        .insert_resource(EntityCount::default())
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        position: WindowPosition::At(IVec2::new(-600, 600)),
                        resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
                        resizable: false,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugin(FrameTimeDiagnosticsPlugin)
        .add_plugin(PixelCameraPlugin)
        .add_systems(Startup, (setup))
        .add_systems(Update, (update_debug_text))
        .add_systems(
            FixedUpdate,
            (
                update_player_movement,
                update_movement,
                clamp_inside_world,
                shoot,
            )
                .chain(),
        )
        .add_systems(PostUpdate, (despawn_outside_world).chain())
        .run();
}

#[derive(Component)]
struct Lifetime(Duration);

fn update_lifetimes(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Lifetime)>,
) {
    for (entity, mut lifetime) in &mut query {
        lifetime.0 = lifetime.0.saturating_sub(time.delta());
        if lifetime.0 <= Duration::from_secs(0) {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn despawn_outside_world(mut commands: Commands, query: Query<(Entity, &Transform, &AABB)>) {
    for (entity, transform, aabb) in &query {
        let position = transform.translation;
        let half_size = aabb.half_size;

        if position.x - half_size.x > GAME_WIDTH
            || position.x + half_size.x < -GAME_WIDTH
            || position.y - half_size.y > GAME_HEIGHT
            || position.y + half_size.y < -GAME_HEIGHT
        {
            commands.entity(entity).despawn_recursive();
        }
    }
}

#[derive(Component)]
struct Bullet;

fn shoot(
    keys: Res<Input<KeyCode>>,
    mut commands: Commands,
    mut query: Query<(&Transform, &AABB), With<Player>>,
) {
    if keys.pressed(KeyCode::Space) {
        for (transform, aabb) in &mut query {
            spawn_bullet(transform, aabb, &mut commands, Direction::Left);
            spawn_bullet(transform, aabb, &mut commands, Direction::Right);
        }
    }
}

enum Direction {
    Left,
    Right,
}

fn spawn_bullet(transform: &Transform, aabb: &AABB, commands: &mut Commands, direction: Direction) {
    let position = transform.translation;
    let half_size = aabb.half_size;

    let direction_component = match direction {
        Direction::Left => -1.0,
        Direction::Right => 1.0,
    };

    const BULLET_SPEED: f32 = 3.0;

    commands.spawn((
        Bullet,
        SpriteBundle {
            sprite: Sprite {
                color: Color::WHITE,
                custom_size: Some(Vec2::splat(2.0)),
                ..default()
            },
            transform: Transform::from_xyz(
                position.x + half_size.x * direction_component,
                position.y,
                1.0,
            ),
            ..default()
        },
        AABB {
            half_size: Vec2::splat(1.0),
        },
        Movement {
            acceleration: Vec2::ZERO,
            velocity: Vec2::new(direction_component * BULLET_SPEED, 0.0),
            damping: 0.0,
        },
        Lifetime(Duration::from_secs(1)),
    ));
}

#[derive(Component)]
struct Camera;

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Background;

#[derive(Component)]
struct DebugText {
    timer: Timer,
}

#[derive(Resource, Default)]
struct EntityCount(usize);

impl DebugText {
    fn new() -> Self {
        Self {
            timer: Timer::from_seconds(0.1, TimerMode::Repeating),
        }
    }
}

#[derive(Component, Default)]
struct Movement {
    acceleration: Vec2,
    velocity: Vec2,
    damping: f32,
}

#[derive(Component)]
struct AABB {
    half_size: Vec2,
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let ship_handle = asset_server.load("ship.png");
    let ship_atlas = TextureAtlas::from_grid(ship_handle, Vec2::new(48.0, 32.0), 6, 1, None, None);
    let ship_atlas_handle = texture_atlases.add(ship_atlas);

    commands.spawn((Camera, PixelCameraBundle::from_zoom(2)));

    commands.spawn((
        Background,
        SpriteBundle {
            texture: asset_server.load("bg_01.png"),
            transform: Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::splat(2.0)),
            ..default()
        },
    ));

    commands.spawn((
        Player,
        SpriteSheetBundle {
            texture_atlas: ship_atlas_handle,
            sprite: TextureAtlasSprite::new(0),
            transform: Transform::from_xyz(0.0, 32. - GAME_HEIGHT / 2., 1.0),
            ..default()
        },
        AABB {
            half_size: Vec2::splat(16.),
        },
        Movement {
            acceleration: Vec2::ZERO,
            velocity: Vec2::ZERO,
            damping: 0.1,
        },
    ));

    commands.spawn((
        DebugText::new(),
        TextBundle::from_sections([
            TextSection::new(
                "FPS: 0",
                TextStyle {
                    font: font.clone(),
                    font_size: 12.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::new(
                "\nEntities: 0",
                TextStyle {
                    font: font.clone(),
                    font_size: 12.0,
                    color: Color::WHITE,
                },
            ),
        ])
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        }),
    ));
}

fn update_debug_text(
    time: Res<Time>,
    diagnostics: Res<Diagnostics>,
    mut query: Query<(&mut Text, &mut DebugText)>,
) {
    for (mut text, mut debug_text) in &mut query {
        debug_text.timer.tick(time.delta());
        if debug_text.timer.just_finished() {
            if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
                if let Some(fps) = fps.smoothed() {
                    text.sections[0].value = format!("FPS: {fps:.1}");
                }
            }
        }
    }
}

fn update_player_movement(
    keys: Res<Input<KeyCode>>,
    mut query: Query<(&mut Movement), With<Player>>,
) {
    for (mut movement) in &mut query {
        let acceleration_x = if keys.pressed(KeyCode::A) {
            -1.0
        } else if keys.pressed(KeyCode::D) {
            1.0
        } else {
            0.0
        };

        let acceleration_y = if keys.pressed(KeyCode::W) {
            1.0
        } else if keys.pressed(KeyCode::S) {
            -1.0
        } else {
            0.0
        };

        let acceleration = Vec2::new(acceleration_x, acceleration_y);
        movement.acceleration = acceleration;
    }
}

fn update_movement(mut query: Query<(&mut Movement, &mut Transform)>) {
    for (mut movement, mut transform) in &mut query {
        let acceleration = movement.acceleration;
        if acceleration.x != 0.0 || acceleration.y != 0.0 {
            movement.velocity += acceleration * 0.1;
        } else {
            let damping = movement.damping;
            movement.velocity *= 1. - damping;
        }
        transform.translation.x += movement.velocity.x;
        transform.translation.y += movement.velocity.y;
    }
}

fn clamp_inside_world(
    mut query: Query<(&mut Transform, &AABB, Option<&mut Movement>), With<Player>>,
) {
    for (mut transform, aabb, movement) in &mut query {
        let half_width = GAME_WIDTH / 2.;
        let half_height = GAME_HEIGHT / 2.;
        let x = transform.translation.x;
        let y = transform.translation.y;
        let half_size = aabb.half_size;
        transform.translation.x = x.clamp(-half_width + half_size.x, half_width - half_size.x);
        transform.translation.y = y.clamp(-half_height + half_size.y, half_height - half_size.y);

        if let Some(mut movement) = movement {
            let x = transform.translation.x;
            let y = transform.translation.y;
            let half_size = aabb.half_size;
            if x <= -half_width + half_size.x || x >= half_width - half_size.x {
                movement.velocity.x = 0.0;
            }
            if y <= -half_height + half_size.y || y >= half_height - half_size.y {
                movement.velocity.y = 0.0;
            }
        }
    }
}
