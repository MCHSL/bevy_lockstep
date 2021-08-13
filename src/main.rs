use bevy::prelude::*;
use rand::distributions::{Distribution, Uniform};
use rand::Rng;

mod mouse_tracking;
use mouse_tracking::*;

mod lockstep;
use lockstep::*;

#[derive(Debug, Clone)]
enum PlayerActions {
    NoAction,
    MoveTo(f32, f32),
    Build(f32, f32),
}

impl Default for PlayerActions {
    fn default() -> Self {
        return PlayerActions::NoAction;
    }
}

type InputQueue = lockstep::InputQueue<PlayerActions>;
type CurrentInputs = lockstep::CurrentInputs<PlayerActions>;

pub struct InputTimeout(Timer);

fn take_player_input(
    current_tick: Res<Tick>,
    mut inputs: ResMut<InputQueue>,
    mouse_pos: Res<MousePos>,
    mouse: Res<Input<MouseButton>>,
    mut input_timeout: ResMut<InputTimeout>,
    time: Res<Time>,
) {
    if mouse.just_pressed(MouseButton::Left) {
        println!("Adding input to tick {}", current_tick.0 + 1);
        let pos = mouse_pos.0;
        inputs.insert(
            Tick(current_tick.0 + 1),
            0,
            PlayerActions::MoveTo(pos.x, pos.y),
        );
        input_timeout.0.reset();
    } else if input_timeout.0.tick(time.delta()).just_finished() {
        return inputs.insert(*current_tick, 0, PlayerActions::default());
    }
}

fn test_lockstep_system() {}

fn setup(mut commands: Commands, mut materials: ResMut<Assets<ColorMaterial>>) {
    commands
        .spawn_bundle(OrthographicCameraBundle::new_2d())
        .insert(MainCamera);
    commands.insert_resource(Materials {
        square_material: materials.add(Color::rgb(0.1, 0.1, 0.1).into()),
    });
}

fn spawn_boye(mut commands: Commands, materials: Res<Materials>) {
    commands
        .spawn_bundle(SpriteBundle {
            material: materials.square_material.clone(),
            sprite: Sprite::new(Vec2::new(10.0, 10.0)),
            ..Default::default()
        })
        .insert(Player(0));

    commands
        .spawn_bundle(SpriteBundle {
            material: materials.square_material.clone(),
            sprite: Sprite::new(Vec2::new(20.0, 20.0)),
            transform: Transform::from_xyz(20.0, 20.0, 0.0),
            ..Default::default()
        })
        .insert(Player(1));
}

struct MockTimer(Timer);
fn mock_inputs(
    current_tick: Res<Tick>,
    mut inputs: ResMut<InputQueue>,
    time: Res<Time>,
    mut mock_timer: ResMut<MockTimer>,
) {
    if !mock_timer.0.tick(time.delta()).just_finished() {
        return;
    }

    inputs.insert(*current_tick, 1, PlayerActions::default());

    let range = Uniform::new(-200.0, 200.0);
    let mut rng = rand::thread_rng();

    let x = range.sample(&mut rng);
    let y = range.sample(&mut rng);

    inputs.insert(Tick(current_tick.0 + 1), 1, PlayerActions::MoveTo(x, y));
}

fn handle_movement(inputs: Res<CurrentInputs>, mut q: Query<(&mut Transform, &Player)>) {
    for (mut transform, player) in q.iter_mut() {
        for input in inputs.0.get(&player.0).unwrap() {
            if let PlayerActions::MoveTo(x, y) = input {
                *transform = Transform::from_xyz(*x, *y, 0.0);
            }
        }
    }
}

struct Player(PlayerID);
struct Materials {
    square_material: Handle<ColorMaterial>,
}

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .insert_resource(lockstep::LockstepTimer(Timer::from_seconds(0.1, true)))
        .insert_resource(lockstep::NumPlayers(2))
        .add_plugin(LockstepPlugin::<PlayerActions>::default())
        .add_plugin(MouseTrackingPlugin)
        .add_startup_system(setup.system())
        .add_startup_stage("game_setup", SystemStage::single(spawn_boye.system()))
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(LOCKSTEP)
                .after(LOCKSTEP_START)
                .before(LOCKSTEP_END)
                .with_system(test_lockstep_system.system())
                .with_system(handle_movement.system()),
        )
        .insert_resource(MockTimer(Timer::from_seconds(0.05, true)))
        .insert_resource(InputTimeout(Timer::from_seconds(0.05, true)))
        .add_system(take_player_input.system())
        .add_system(mock_inputs.system())
        .run();
}
