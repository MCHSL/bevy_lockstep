use bevy::ecs::schedule::ShouldRun;
use bevy::prelude::*;
use std::collections::HashMap;
use std::marker::PhantomData;

pub const LOCKSTEP: &'static str = "lockstep";
pub const LOCKSTEP_START: &'static str = "lockstep_start";
pub const LOCKSTEP_END: &'static str = "lockstep_end";

pub type PlayerID = usize;

pub struct NumPlayers(pub usize);

pub struct LockstepTimer(pub Timer);

#[derive(Default, PartialEq, Eq, Hash, Clone, Copy)]
pub struct Tick(pub u64);

#[derive(Default)]
pub struct Step<I: Default> {
    inputs: HashMap<PlayerID, Vec<I>>,
}

#[derive(Default)]
pub struct CurrentInputs<I: Default>(pub HashMap<PlayerID, Vec<I>>);

#[derive(Default)]
pub struct InputQueue<I: Default>(HashMap<Tick, Step<I>>);

impl<I: Default> InputQueue<I> {
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

pub fn can_step<I: 'static + Send + Sync + Default>(
    current_tick: Res<Tick>,
    inputs: Res<InputQueue<I>>,
    time: Res<Time>,
    mut timer: ResMut<LockstepTimer>,
    num_players: Res<NumPlayers>,
) -> ShouldRun {
    if !timer.0.tick(time.delta()).just_finished() {
        return ShouldRun::No;
    }

    let entry = inputs.0.get(&*current_tick);
    if let Some(q) = entry {
        if q.inputs.len() == num_players.0 {
            ShouldRun::Yes
        } else {
            ShouldRun::No
        }
    } else {
        ShouldRun::No
    }
}

fn prepare_inputs<I: 'static + Send + Sync + Default + Clone>(
    current_tick: ResMut<Tick>,
    inputs: ResMut<InputQueue<I>>,
    mut current_inputs: ResMut<CurrentInputs<I>>,
) {
    *current_inputs = CurrentInputs(inputs.0.get(&*current_tick).unwrap().inputs.clone());
}

fn finish_step<I: 'static + Send + Sync + Default + Clone + std::fmt::Debug>(
    mut current_tick: ResMut<Tick>,
    mut inputs: ResMut<InputQueue<I>>,
) {
    println!(
        "Tick {}: {:?}",
        current_tick.0,
        inputs.0.get(&*current_tick).unwrap().inputs
    );
    inputs.0.remove(&*current_tick);
    current_tick.0 += 1;
}

pub struct LockstepPlugin<PlayerActions: 'static + Send + Sync + Default + Clone>(
    pub PhantomData<PlayerActions>,
);

impl<T: 'static + Send + Sync + Default + Clone> Default for LockstepPlugin<T> {
    fn default() -> Self {
        LockstepPlugin(PhantomData::<T>)
    }
}

impl<PlayerActions: 'static + Send + Sync + Default + Clone + std::fmt::Debug> Plugin
    for LockstepPlugin<PlayerActions>
{
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(Tick::default())
            .insert_resource(InputQueue::<PlayerActions>::default())
            .insert_resource(CurrentInputs::<PlayerActions>::default())
            .add_system_set(
                SystemSet::new()
                    .with_run_criteria(can_step::<PlayerActions>.system().label(LOCKSTEP))
                    .with_system(
                        prepare_inputs::<PlayerActions>
                            .system()
                            .label(LOCKSTEP_START),
                    )
                    .with_system(finish_step::<PlayerActions>.system().label(LOCKSTEP_END)),
            );
    }
}
