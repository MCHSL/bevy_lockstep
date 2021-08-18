use bevy::ecs::schedule::ShouldRun;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::marker::PhantomData;

pub const LOCKSTEP: &'static str = "lockstep";
pub const LOCKSTEP_START: &'static str = "lockstep_start";
pub const LOCKSTEP_END: &'static str = "lockstep_end";

#[macro_export]
macro_rules! set_types {
    ($player_id:ty, $player_actions:ty) => {
        type InputQueue = $crate::InputQueue<$player_id, $player_actions>;
        type CurrentInputs = $crate::CurrentInputs<$player_id, $player_actions>;
        type LocalAction = $crate::LocalAction<$player_id, $player_actions>;
        type RemoteAction = $crate::RemoteAction<$player_id, $player_actions>;
    };
}

pub struct Config {
    pub num_players: usize,
    pub ticks_per_second: usize,
    pub paused: bool,
}

struct LockstepTimer(pub Timer);

#[derive(Default, PartialEq, Eq, Hash, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Tick(pub u64);

#[derive(Default)]
pub struct Step<PlayerID: Default, Action: Default> {
    pub inputs: HashMap<PlayerID, Vec<Action>>,
}

#[derive(Default)]
pub struct CurrentInputs<PlayerID: Default, Action: Default>(pub HashMap<PlayerID, Vec<Action>>);

#[derive(Default)]
pub struct InputQueue<PlayerID: Default, Action: Default>(HashMap<Tick, Step<PlayerID, Action>>);

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct ActionAtTick<PlayerID: Default + Clone, Action: Default + Clone> {
    pub player: PlayerID,
    pub action: Action,
    pub tick: Tick,
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct LocalAction<PlayerID: Default + Clone, Action: Default + Clone>(
    pub ActionAtTick<PlayerID, Action>,
);

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct RemoteAction<PlayerID: Default + Clone, Action: Default + Clone>(
    pub ActionAtTick<PlayerID, Action>,
);

impl<PlayerID: Default + Eq + std::hash::Hash, Action: Default> InputQueue<PlayerID, Action> {
    pub fn insert(&mut self, tick: Tick, player: PlayerID, command: Action) {
        self.0
            .entry(tick)
            .or_default()
            .inputs
            .entry(player)
            .or_default()
            .push(command);
    }

    pub fn get(&self, tick: Tick) -> Option<&Step<PlayerID, Action>> {
        self.0.get(&tick)
    }
}

fn can_step<PlayerID: 'static + Send + Sync + Default, Action: 'static + Send + Sync + Default>(
    current_tick: Res<Tick>,
    inputs: Res<InputQueue<PlayerID, Action>>,
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
    Action: 'static + Send + Sync + Default + Clone,
>(
    current_tick: ResMut<Tick>,
    inputs: ResMut<InputQueue<PlayerID, Action>>,
    mut current_inputs: ResMut<CurrentInputs<PlayerID, Action>>,
) {
    *current_inputs = CurrentInputs(inputs.0.get(&*current_tick).unwrap().inputs.clone());
}

fn finish_step<
    PlayerID: 'static + Send + Sync + Default + std::fmt::Debug,
    Action: 'static + Send + Sync + Default + Clone + std::fmt::Debug,
>(
    mut current_tick: ResMut<Tick>,
    mut inputs: ResMut<InputQueue<PlayerID, Action>>,
) {
    /*println!(
        "Tick {}: {:?}",
        current_tick.0,
        inputs.0.get(&*current_tick).unwrap().inputs
    );*/
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

fn insert_local_actions<
    PlayerID: 'static + Send + Sync + Default + Clone + Eq + std::hash::Hash + std::fmt::Debug,
    Action: 'static + Send + Sync + Default + Clone + std::fmt::Debug,
>(
    mut events: EventReader<LocalAction<PlayerID, Action>>,
    mut inputs: ResMut<InputQueue<PlayerID, Action>>,
) {
    for ev in events.iter() {
        inputs.insert(ev.0.tick, ev.0.player.clone(), ev.0.action.clone());
    }
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
            .add_event::<LocalAction<PlayerID, PlayerActions>>()
            .add_event::<RemoteAction<PlayerID, PlayerActions>>()
            .add_startup_system(insert_timer.system())
            .add_system(insert_local_actions::<PlayerID, PlayerActions>.system())
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
