use bevy::ecs::schedule::ShouldRun;
use bevy::prelude::*;
use std::collections::HashMap;
use std::marker::PhantomData;

pub const LOCKSTEP: &'static str = "lockstep";
pub const LOCKSTEP_START: &'static str = "lockstep_start";
pub const LOCKSTEP_END: &'static str = "lockstep_end";

pub struct Config {
    pub num_players: usize,
    pub ticks_per_second: usize,
    pub paused: bool,
}

struct LockstepTimer(pub Timer);

#[derive(Default, PartialEq, Eq, Hash, Clone, Copy)]
pub struct Tick(pub u64);

#[derive(Default)]
pub struct Step<PlayerID: Default, I: Default> {
    inputs: HashMap<PlayerID, Vec<I>>,
}

#[derive(Default)]
pub struct CurrentInputs<PlayerID: Default, I: Default>(pub HashMap<PlayerID, Vec<I>>);

#[derive(Default)]
pub struct InputQueue<PlayerID: Default, I: Default>(HashMap<Tick, Step<PlayerID, I>>);

impl<PlayerID: Default + Eq + std::hash::Hash, I: Default> InputQueue<PlayerID, I> {
    pub fn insert(&mut self, tick: Tick, player: PlayerID, command: I) {
        self.0
            .entry(tick)
            .or_default()
            .inputs
            .entry(player)
            .or_default()
            .push(command);
    }
}

fn can_step<PlayerID: 'static + Send + Sync + Default, I: 'static + Send + Sync + Default>(
    current_tick: Res<Tick>,
    inputs: Res<InputQueue<PlayerID, I>>,
    time: Res<Time>,
    mut timer: ResMut<LockstepTimer>,
    config: Res<Config>,
) -> ShouldRun {
    if config.paused {
        return ShouldRun::No;
    }

    if !timer.0.tick(time.delta()).just_finished() {
        return ShouldRun::No;
    }

    let entry = (*inputs).0.get(&*current_tick);
    if let Some(q) = entry {
        if q.inputs.len() == config.num_players {
            ShouldRun::Yes
        } else {
            ShouldRun::No
        }
    } else {
        ShouldRun::No
    }
}

fn prepare_inputs<
    PlayerID: 'static + Send + Sync + Default + Clone,
    I: 'static + Send + Sync + Default + Clone,
>(
    current_tick: ResMut<Tick>,
    inputs: ResMut<InputQueue<PlayerID, I>>,
    mut current_inputs: ResMut<CurrentInputs<PlayerID, I>>,
) {
    *current_inputs = CurrentInputs(inputs.0.get(&*current_tick).unwrap().inputs.clone());
}

fn finish_step<
    PlayerID: 'static + Send + Sync + Default + std::fmt::Debug,
    I: 'static + Send + Sync + Default + Clone + std::fmt::Debug,
>(
    mut current_tick: ResMut<Tick>,
    mut inputs: ResMut<InputQueue<PlayerID, I>>,
) {
    println!(
        "Tick {}: {:?}",
        current_tick.0,
        inputs.0.get(&*current_tick).unwrap().inputs
    );
    inputs.0.remove(&*current_tick);
    current_tick.0 += 1;
}

pub struct LockstepPlugin<PlayerID, PlayerActions: 'static + Send + Sync + Default + Clone>(
    pub PhantomData<(PlayerID, PlayerActions)>,
);

impl<PlayerID, T: 'static + Send + Sync + Default + Clone> Default for LockstepPlugin<PlayerID, T> {
    fn default() -> Self {
        LockstepPlugin(PhantomData::<(PlayerID, T)>)
    }
}

fn insert_timer(mut commands: Commands, config: Res<Config>) {
    commands.insert_resource(LockstepTimer(Timer::from_seconds(
        1.0 / config.ticks_per_second as f32,
        true,
    )))
}

impl<
        PlayerID: 'static + Send + Sync + Default + Clone + Eq + std::hash::Hash + std::fmt::Debug,
        PlayerActions: 'static + Send + Sync + Default + Clone + std::fmt::Debug,
    > Plugin for LockstepPlugin<PlayerID, PlayerActions>
{
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(Tick::default())
            .insert_resource(InputQueue::<PlayerID, PlayerActions>::default())
            .insert_resource(CurrentInputs::<PlayerID, PlayerActions>::default())
            .add_startup_system(insert_timer.system())
            .add_system_set(
                SystemSet::new()
                    .with_run_criteria(can_step::<PlayerID, PlayerActions>.system().label(LOCKSTEP))
                    .with_system(
                        prepare_inputs::<PlayerID, PlayerActions>
                            .system()
                            .label(LOCKSTEP_START),
                    )
                    .with_system(
                        finish_step::<PlayerID, PlayerActions>
                            .system()
                            .label(LOCKSTEP_END),
                    ),
            );
    }
}

//EXAMPLE

/*
fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .insert_resource(lockstep::Config {
            num_players: 2,
            ticks_per_second: 5,
            paused: false,
        })
        .add_plugin(LockstepPlugin::<usize, PlayerActions>::default())
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(LOCKSTEP)
                .after(LOCKSTEP_START)
                .before(LOCKSTEP_END)
                .with_system(do_thing_in_lockstep.system()),
        )
        .run();
}
*/
