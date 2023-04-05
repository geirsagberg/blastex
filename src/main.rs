#![allow(unused_parens)]

use std::f32::consts::PI;

use rand::{thread_rng, Rng};

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

const BULLET_SPEED: f32 = 3.0;

fn main() {
    App::new()
        .insert_resource(EntityCount::default())
        .insert_resource(Score(0))
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
                (
                    update_player_movement,
                    update_movement,
                    clamp_inside_world,
                    shoot,
                    check_collisions,
                )
                    .chain(),
                spawn_enemies,
                update_lifetimes,
                despawn_outside_world,
                spawn_mirrors,
            ),
        )
        .run();
}

fn spawn_mirrors(
    mut commands: Commands,
    time: Res<FixedTime>,
    mut query: Query<(&mut MirrorSpawner, &GlobalTransform)>,
) {
    for (mut mirror_spawner, transform) in &mut query {
        if mirror_spawner.timer.tick(time.period).finished() {
            commands.spawn(MirrorBundle {
                aabb: AABB {
                    half_size: Vec2::new(8.0, 1.0),
                },
                lifetime: Lifetime::from_seconds(10.0),
                movement: Movement {
                    velocity: Vec2::new(0.0, 1.0),
                    max_speed: 1.0,
                    ..default()
                },
                sprite: SpriteBundle {
                    transform: //Transform::default(),
                    transform
                        .compute_transform()
                        .with_rotation(Quat::from_rotation_z(mirror_spawner.angle)),
                    sprite: Sprite {
                        color: Color::WHITE,
                        custom_size: Some(Vec2::new(16.0, 2.0)),
                        ..default()
                    },
                    ..default()
                },
                ..default()
            });
        }
    }
}

#[derive(Component, Default)]
struct Mirror;

#[derive(Bundle, Default)]
struct MirrorBundle {
    sprite: SpriteBundle,
    movement: Movement,
    aabb: AABB,
    mirror: Mirror,
    lifetime: Lifetime,
}

#[derive(Component, Default)]
struct Enemy;

#[derive(Bundle)]
struct EnemyBundle {
    sprite: SpriteBundle,
    movement: Movement,
    aabb: AABB,
    enemy: Enemy,
    lifetime: Lifetime,
}

#[derive(Component)]
struct MirrorSpawner {
    timer: Timer,
    angle: f32,
}

#[derive(Bundle)]
struct MirrorSpawnerBundle {
    mirror_spawner: MirrorSpawner,
    #[bundle]
    transform_bundle: TransformBundle,
}

impl Default for EnemyBundle {
    fn default() -> Self {
        Self {
            sprite: SpriteBundle::default(),
            movement: Movement::default(),
            aabb: AABB::default(),
            enemy: Enemy::default(),
            lifetime: Lifetime::from_seconds(5.0),
        }
    }
}

#[derive(Component)]
struct EnemySpawner {
    timer: Timer,
    texture: Handle<Image>,
    movement: Movement,
    aabb: AABB,
}

#[derive(Component, Default)]
struct Lifetime {
    timer: Timer,
}

impl Lifetime {
    fn from_seconds(seconds: f32) -> Self {
        Self {
            timer: Timer::from_seconds(seconds, TimerMode::Once),
        }
    }
}

fn spawn_enemies(
    time: Res<FixedTime>,
    mut commands: Commands,
    mut query: Query<(&mut EnemySpawner)>,
) {
    for (mut enemy_spawner) in &mut query {
        if enemy_spawner.timer.tick(time.period).finished() {
            let mut rng = thread_rng();
            let x = (rng.gen::<f32>() * GAME_WIDTH - GAME_WIDTH / 2.0) * 0.95;
            let y = GAME_HEIGHT / 2.0;

            commands.spawn((EnemyBundle {
                sprite: SpriteBundle {
                    texture: enemy_spawner.texture.clone(),
                    transform: Transform::from_xyz(x, y + 16., 1.0),
                    ..default()
                },
                movement: enemy_spawner.movement,
                aabb: enemy_spawner.aabb,
                ..default()
            },));
        }
    }
}

fn check_obb_overlap(
    transform1: &Transform,
    obb1_half_extents: &Vec2,
    transform2: &Transform,
    obb2_half_extents: &Vec2,
) -> bool {
    // Convert the transforms to 4x4 matrices
    let mat1 = transform1.compute_matrix();
    let mat2 = transform2.compute_matrix();

    // Compute the orientation matrices of each OBB
    let orient1 = Mat4::from_quat(transform1.rotation);
    let orient2 = Mat4::from_quat(transform2.rotation);

    // Compute the axes to be used in the Separating Axis Theorem
    let axes = [
        orient1.x_axis.truncate().truncate(),
        orient1.y_axis.truncate().truncate(),
        orient2.x_axis.truncate().truncate(),
        orient2.y_axis.truncate().truncate(),
    ];

    for axis in axes.iter() {
        // Project the half extents of both OBBs onto the axis
        let mut projection1 = Vec2::new(0.0, 0.0);
        projection1.x = obb1_half_extents.x * axis.dot(orient1.x_axis.truncate().truncate());
        projection1.y = obb1_half_extents.y * axis.dot(orient1.y_axis.truncate().truncate());

        let mut projection2 = Vec2::new(0.0, 0.0);
        projection2.x = obb2_half_extents.x * axis.dot(orient2.x_axis.truncate().truncate());
        projection2.y = obb2_half_extents.y * axis.dot(orient2.y_axis.truncate().truncate());

        // Project the centers of both OBBs onto the axis
        let center1 = mat1.transform_point3(Vec3::ZERO).truncate();
        let center2 = mat2.transform_point3(Vec3::ZERO).truncate();

        let center_projection = center2 - center1;
        let center_distance = center_projection.dot(*axis);

        // Check if the projections of the OBBs onto the axis overlap
        let overlap =
            (projection1.x.abs() + projection1.y.abs() + projection2.x.abs() + projection2.y.abs())
                - center_distance.abs()
                < 0.0001;
        if !overlap {
            return false;
        }
    }

    true
}

fn check_collisions(
    mut commands: Commands,
    query_enemy: Query<(Entity, &AABB, &Transform), With<Enemy>>,
    query_mirror: Query<(Entity, &AABB, &Transform), With<Mirror>>,
    query_bullet: Query<(Entity, &AABB, &Transform), With<Bullet>>,
) {
    for (entity, aabb, transform) in &query_bullet {
        let position = transform.translation;
        let half_size = aabb.half_size;

        for (entity_mirror, aabb_mirror, transform_mirror) in &query_mirror {
            if check_obb_overlap(
                transform,
                &half_size,
                transform_mirror,
                &aabb_mirror.half_size,
            ) {
                commands.entity(entity).despawn_recursive();
                commands.entity(entity_mirror).despawn_recursive();
            }
        }

        for (entity_enemy, aabb_enemy, transform_enemy) in &query_enemy {
            let position_enemy = transform_enemy.translation;
            let half_size_enemy = aabb_enemy.half_size;

            if position.x + half_size.x > position_enemy.x - half_size_enemy.x
                && position.x - half_size.x < position_enemy.x + half_size_enemy.x
                && position.y + half_size.y > position_enemy.y - half_size_enemy.y
                && position.y - half_size.y < position_enemy.y + half_size_enemy.y
            {
                commands.entity(entity).despawn_recursive();
                commands.entity(entity_enemy).despawn_recursive();
            }
        }
    }
}

fn update_lifetimes(
    mut commands: Commands,
    time: Res<FixedTime>,
    mut query: Query<(Entity, &mut Lifetime)>,
) {
    for (entity, mut lifetime) in &mut query {
        if lifetime.timer.tick(time.period).finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

#[derive(Component)]
struct AutoDespawn;

fn despawn_outside_world(
    mut commands: Commands,
    query: Query<(Entity, &Transform, &AABB), With<AutoDespawn>>,
) {
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

    commands.spawn((
        Bullet,
        AutoDespawn,
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
            max_speed: 10.0,
        },
        Lifetime::from_seconds(5.0),
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

#[derive(Component, Default, Clone, Copy)]
struct Movement {
    acceleration: Vec2,
    velocity: Vec2,
    damping: f32,
    max_speed: f32,
}

#[derive(Component, Default, Clone, Copy)]
struct AABB {
    half_size: Vec2,
}

#[derive(Resource)]
struct Score(usize);

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
            max_speed: 2.,
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
                "\nScore: 0",
                TextStyle {
                    font: font.clone(),
                    font_size: 24.0,
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

    commands.spawn(EnemySpawner {
        timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        texture: asset_server.load("enemy_01.png"),
        movement: Movement {
            acceleration: Vec2::new(0.0, -0.1),
            velocity: Vec2::new(0.0, -1.0),
            damping: 0.0,
            max_speed: 10.0,
        },
        aabb: AABB {
            half_size: Vec2::splat(16.0),
        },
    });

    commands.spawn(EnemySpawner {
        timer: Timer::from_seconds(1.5, TimerMode::Repeating),
        texture: asset_server.load("enemy_02.png"),
        movement: Movement {
            acceleration: Vec2::new(0.0, -0.1),
            velocity: Vec2::new(0.0, -0.5),
            damping: 0.0,
            max_speed: 10.0,
        },
        aabb: AABB {
            half_size: Vec2::splat(16.0),
        },
    });

    spawn_mirror_spawner(&mut commands, Direction::Left);
    spawn_mirror_spawner(&mut commands, Direction::Right);
}

fn spawn_mirror_spawner(commands: &mut Commands, direction: Direction) {
    let angle = match direction {
        Direction::Left => -PI / 4.,
        Direction::Right => PI / 4.,
    };
    let x = match direction {
        Direction::Left => -GAME_WIDTH / 2. + 10.,
        Direction::Right => GAME_WIDTH / 2. - 10.,
    };
    commands.spawn(MirrorSpawnerBundle {
        mirror_spawner: MirrorSpawner {
            timer: Timer::from_seconds(1., TimerMode::Repeating),
            angle,
        },
        transform_bundle: TransformBundle::from_transform(Transform::from_xyz(
            x,
            -GAME_HEIGHT / 2. - 10.,
            1.0,
        )),
    });
}

fn update_debug_text(
    time: Res<Time>,
    diagnostics: Res<Diagnostics>,
    mut query: Query<(&mut Text, &mut DebugText)>,
    score: Res<Score>,
) {
    for (mut text, mut debug_text) in &mut query {
        debug_text.timer.tick(time.delta());
        if debug_text.timer.just_finished() {
            if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
                if let Some(fps) = fps.smoothed() {
                    text.sections[0].value = format!("FPS: {fps:.1}");
                }
            }
            let score = score.0;
            text.sections[1].value = format!("\nScore: {score}");
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

        let velocity = movement.velocity;
        let velocity_length = velocity.length();
        if velocity_length > movement.max_speed {
            movement.velocity = velocity / velocity_length * movement.max_speed;
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
