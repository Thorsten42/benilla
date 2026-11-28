//! Parameter sets whose members may conflict with one another, each member's access gathered once.
//! Bevy's `ParamSet` gathers each member's access twice, the first time on a clone of the access
//! the system has built so far, so a set nested `d` deep gathers its leaves `2^d` times; the call
//! host's appliers are one flat set of these ([`disjoint_set!`]), and a system that holds the host
//! beside state of its own pairs them in a [`Pair`].

use bevy::ecs::change_detection::Tick;
use bevy::ecs::component::ComponentId;
use bevy::ecs::system::{
    SystemAccess, SystemMeta, SystemParam, SystemParamItem, SystemParamValidationError,
};
use bevy::ecs::world::unsafe_world_cell::UnsafeWorldCell;
use bevy::ecs::world::{DeferredWorld, World};

/// Gather the access of `P`, one member of a set, alone, check it against what the system holds
/// outside the set, as `ParamSet` does, and add it to `gathered`. Members may conflict with one
/// another: the set lends one at a time.
pub(crate) fn gather<P: SystemParam>(
    state: &P::State,
    meta: &mut SystemMeta,
    outside: &SystemAccess,
    gathered: &mut SystemAccess,
    world: &mut World,
    member: &str,
) {
    let mut alone = SystemAccess::default();
    P::init_access(state, meta, &mut alone, world);
    assert!(
        outside.is_compatible(&alone),
        "{member} in {} conflicts with a parameter outside its set: {:?}",
        meta.name(),
        outside.get_conflicts(&alone),
    );
    gathered.extend(alone);
}

/// A set's verdict on whether its members can be fetched, kept between runs. A member's fetch
/// fails only when a resource or non-send value it declares is absent (no member is a
/// `Single`, `Populated`, `If` or `When`), so the verdict is a function of which declared ones
/// exist: each run tests their existence, an O(1) lookup apiece, and fetches every member again
/// only when that changed. Bevy 0.20 has no cheap `validate_param`; fetching each member every
/// run cost the main thread about 4 ms a frame in motion across the call host's sets.
/// The resources and non-send values a set's members declare, each as `(id, is non-send)`.
type Declared = Box<[(ComponentId, bool)]>;

#[doc(hidden)]
#[derive(Default)]
pub(crate) struct Validity {
    /// What the members declare, as `(id, is non-send)`, gathered once; `None` for an access
    /// unbounded by name, which fetches every run.
    declared: std::sync::OnceLock<Option<Declared>>,
    /// Which of `declared` existed when `verdict` was taken.
    present: Vec<bool>,
    verdict: Option<Result<(), SystemParamValidationError>>,
}

impl Validity {
    /// Record what the members declare, from the access gathered for them.
    pub(crate) fn declare(&self, gathered: &SystemAccess, world: &World) {
        let declared = match gathered {
            SystemAccess::Exclusive => None,
            SystemAccess::None => Some(Box::default()),
            SystemAccess::Shared(set) => {
                let access = crate::resource_access::of_set(world, set);
                match (
                    access.reads().as_finite_set(),
                    access.writes().as_finite_set(),
                ) {
                    (Some(reads), Some(writes)) => {
                        let mut ids: Vec<ComponentId> = reads.iter().chain(writes.iter()).collect();
                        ids.sort_unstable();
                        ids.dedup();
                        Some(
                            ids.into_iter()
                                .map(|id| {
                                    let non_send = world
                                        .components()
                                        .get_info(id)
                                        .is_some_and(|info| !info.is_send_and_sync());
                                    (id, non_send)
                                })
                                .collect(),
                        )
                    }
                    _ => None,
                }
            }
        };
        let _ = self.declared.set(declared);
    }

    /// The verdict, from the last one while the declared resources' existence is unchanged,
    /// else from `fetch_all`, which fetches every member.
    ///
    /// # Safety
    /// `world` may read the existence of every resource the members declare.
    pub(crate) unsafe fn check(
        &mut self,
        world: UnsafeWorldCell,
        mut fetch_all: impl FnMut() -> Result<(), SystemParamValidationError>,
    ) -> Result<(), SystemParamValidationError> {
        let Some(Some(declared)) = self.declared.get() else {
            return fetch_all();
        };
        self.present.resize(declared.len(), false);
        let mut changed = self.verdict.is_none();
        for ((id, non_send), was) in declared.iter().zip(self.present.iter_mut()) {
            // SAFETY: existence only, of a resource the caller may read.
            let now = unsafe { exists(world, *id, *non_send) };
            changed |= now != *was;
            *was = now;
        }
        if changed {
            self.verdict = Some(fetch_all());
        }
        let verdict = self.verdict.clone().expect("taken above");
        // The tests hold the kept verdict to a fresh fetch, so a member that fails on more than a
        // declared resource's existence is caught by a test that exercises that failure.
        #[cfg(test)]
        if !changed {
            assert_eq!(
                verdict.is_ok(),
                fetch_all().is_ok(),
                "a set's kept verdict differs from fetching its members"
            );
        }
        verdict
    }
}

/// Whether the resource or non-send value `id` exists, as a fetch of it would find it.
///
/// # Safety
/// `world` may read whether `id` exists.
unsafe fn exists(world: UnsafeWorldCell, id: ComponentId, non_send: bool) -> bool {
    if non_send {
        // SAFETY: the existence of a value the caller may read.
        unsafe { world.storages() }
            .non_sends
            .get(id)
            .is_some_and(|data| data.is_present())
    } else {
        // SAFETY: as above.
        unsafe { world.resource_entities() }
            .get(id)
            .and_then(|entity| world.get_entity(entity).ok())
            .is_some_and(|entity| entity.contains_id(id))
    }
}

/// A set of named members that may conflict with one another, lent one at a time through a
/// method of the member's name, each member's access gathered once ([`gather`]). Each member's
/// type is written with `'static` lifetimes; its method lends it for the borrow of the set. A
/// member may carry one `cfg` after its docs.
macro_rules! disjoint_set {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident, $state:ident $(, $watched:ident)? {
            $($(#[doc = $doc:literal])* $(#[cfg($cfg:meta)])? $field:ident: $ty:ty,)*
        }
    ) => {
        $(#[$meta])*
        $vis struct $name<'w, 's> {
            states: &'s mut $state,
            meta: bevy::ecs::system::SystemMeta,
            world: bevy::ecs::world::unsafe_world_cell::UnsafeWorldCell<'w>,
            change_tick: bevy::ecs::change_detection::Tick,
            /// The tick and member of the lend running now, while the census watches writes.
            #[cfg(debug_assertions)]
            lend: Option<(bevy::ecs::change_detection::Tick, &'static str)>,
        }

        #[doc(hidden)]
        $vis struct $state {
            $($(#[cfg($cfg)])? $field: crate::script_calls::disjoint::Lent<<$ty as bevy::ecs::system::SystemParam>::State>,)*
            validity: crate::script_calls::disjoint::Validity,
        }

        // SAFETY: the set registers every member's access, each checked against what the system
        // holds outside the set ([`crate::script_calls::disjoint::gather`]), and lends one member
        // at a time, through `&mut self`.
        unsafe impl bevy::ecs::system::SystemParam for $name<'_, '_> {
            // Boxed, as a system holds its params' state inline from the moment it is added.
            type State = Box<$state>;
            type Item<'w, 's> = $name<'w, 's>;

            fn init_state(world: &mut bevy::ecs::world::World) -> Self::State {
                Box::new($state {
                    $(
                        $(#[cfg($cfg)])?
                        $field: crate::script_calls::disjoint::Lent {
                            state: <$ty as bevy::ecs::system::SystemParam>::init_state(world),
                            lent: false,
                        },
                    )*
                    validity: Default::default(),
                })
            }

            fn init_access(
                state: &Self::State,
                meta: &mut bevy::ecs::system::SystemMeta,
                access: &mut bevy::ecs::system::SystemAccess,
                world: &mut bevy::ecs::world::World,
            ) {
                let mut gathered = bevy::ecs::system::SystemAccess::default();
                $(
                    $(#[cfg($cfg)])?
                    crate::script_calls::disjoint::gather::<$ty>(
                        &state.$field.state,
                        meta,
                        access,
                        &mut gathered,
                        world,
                        stringify!($field),
                    );
                )*
                state.validity.declare(&gathered, world);
                access.extend(gathered);
            }

            fn apply(
                state: &mut Self::State,
                meta: &bevy::ecs::system::SystemMeta,
                world: &mut bevy::ecs::world::World,
            ) {
                $(
                    $(#[cfg($cfg)])?
                    if std::mem::take(&mut state.$field.lent) {
                        <$ty as bevy::ecs::system::SystemParam>::apply(&mut state.$field.state, meta, world);
                    }
                )*
            }

            fn queue(
                state: &mut Self::State,
                meta: &bevy::ecs::system::SystemMeta,
                mut world: bevy::ecs::world::DeferredWorld,
            ) {
                $($(#[cfg($cfg)])? if std::mem::take(&mut state.$field.lent) {
                    <$ty as bevy::ecs::system::SystemParam>::queue(
                        &mut state.$field.state,
                        meta,
                        world.reborrow(),
                    );
                })*
            }

            unsafe fn get_param<'w, 's>(
                state: &'s mut Self::State,
                meta: &bevy::ecs::system::SystemMeta,
                world: bevy::ecs::world::unsafe_world_cell::UnsafeWorldCell<'w>,
                change_tick: bevy::ecs::change_detection::Tick,
            ) -> Result<Self::Item<'w, 's>, bevy::ecs::system::SystemParamValidationError> {
                // Every member validated up front, so an unavailable member skips the system
                // rather than failing its lend: fetched and dropped when the declared resources'
                // existence changed, the kept verdict otherwise
                // ([`crate::script_calls::disjoint::Validity`]).
                let $state { validity, $($(#[cfg($cfg)])? $field,)* } = &mut **state;
                // SAFETY: upheld by the caller, for every member's access as for the set's.
                unsafe {
                    validity.check(world, || {
                        $(
                            $(#[cfg($cfg)])?
                            let _ = <$ty as bevy::ecs::system::SystemParam>::get_param(
                                &mut $field.state,
                                meta,
                                world,
                                change_tick,
                            )?;
                        )*
                        Ok(())
                    })
                }?;
                Ok($name {
                    states: &mut **state,
                    meta: meta.clone(),
                    world,
                    change_tick,
                    #[cfg(debug_assertions)]
                    lend: None,
                })
            }
        }

        impl $name<'_, '_> {
            /// Whether the census watches this set's lends: one marked `watched`, the state a call
            /// writes, and not a set of reads.
            #[cfg(debug_assertions)]
            const WATCHED: bool = false $(|| { let _ = stringify!($watched); true })?;

            /// The lend that ended with the last member's item has written or not, for the census
            /// ([`crate::script_calls::disjoint::end_lend`]): the host calls this where its
            /// members are all unlent, such as the end of an apply and ahead of a signal point.
            #[cfg(debug_assertions)]
            #[allow(dead_code)]
            pub(crate) fn end_lend(&mut self) {
                crate::script_calls::disjoint::end_lend(self.world, &mut self.lend);
            }

            $(
                $(#[doc = $doc])*
                $(#[cfg($cfg)])?
                pub(crate) fn $field(
                    &mut self,
                ) -> bevy::ecs::system::SystemParamItem<'_, '_, $ty> {
                    #[cfg(debug_assertions)]
                    let change_tick = if Self::WATCHED {
                        crate::script_calls::disjoint::begin_lend(
                            self.world,
                            &mut self.lend,
                            self.change_tick,
                            stringify!($field),
                        )
                    } else {
                        self.change_tick
                    };
                    #[cfg(not(debug_assertions))]
                    let change_tick = self.change_tick;
                    self.states.$field.lent = true;
                    // SAFETY: the set registered this member's access, and `&mut self` keeps
                    // every other member of the set unlent while this one lives.
                    unsafe {
                        <$ty as bevy::ecs::system::SystemParam>::get_param(
                            &mut self.states.$field.state,
                            &self.meta,
                            self.world,
                            change_tick,
                        )
                    }
                    .unwrap_or_else(|err| {
                        panic!("{} validated at fetch, then failed: {err}", stringify!($field))
                    })
                }
            )*
        }
    };
}
pub(crate) use disjoint_set;

/// Two params that may conflict, lent one at a time, as a two-member `ParamSet` with each
/// member's access gathered once ([`gather`]).
pub(crate) struct Pair<'w, 's, A: SystemParam, B: SystemParam> {
    states: &'s mut PairState<A::State, B::State>,
    meta: SystemMeta,
    world: UnsafeWorldCell<'w>,
    change_tick: Tick,
    /// Whether the second member validated, once [`Pair::p1_valid`] has asked this run.
    valid: Option<bool>,
    /// The tick and name of the first member's lend running now, while the census watches writes.
    #[cfg(debug_assertions)]
    lend: Option<(Tick, &'static str)>,
}

/// A set member's state, and whether it was lent since its deferred work last went out: a member
/// not lent wrote nothing to its buffers, so its apply and queue are skipped.
#[doc(hidden)]
pub(crate) struct Lent<S> {
    pub(crate) state: S,
    pub(crate) lent: bool,
}

/// A [`Pair`]'s members' states, and whether the second was lent since its deferred work last went
/// out: a member not lent wrote nothing to its buffers, so its apply and queue are skipped.
#[doc(hidden)]
pub(crate) struct PairState<A, B> {
    first: A,
    second: B,
    second_lent: bool,
    validity: Validity,
}

// SAFETY: as [`disjoint_set!`]'s: both members' access registered and checked against what the
// system holds outside the pair, one member lent at a time.
unsafe impl<A: SystemParam, B: SystemParam> SystemParam for Pair<'_, '_, A, B> {
    type State = PairState<A::State, B::State>;
    type Item<'w, 's> = Pair<'w, 's, A, B>;

    fn init_state(world: &mut World) -> Self::State {
        PairState {
            first: A::init_state(world),
            second: B::init_state(world),
            second_lent: false,
            validity: Validity::default(),
        }
    }

    fn init_access(
        state: &Self::State,
        meta: &mut SystemMeta,
        access: &mut SystemAccess,
        world: &mut World,
    ) {
        let mut gathered = SystemAccess::default();
        gather::<A>(&state.first, meta, access, &mut gathered, world, "p0");
        gather::<B>(&state.second, meta, access, &mut gathered, world, "p1");
        state.validity.declare(&gathered, world);
        access.extend(gathered);
    }

    fn apply(state: &mut Self::State, meta: &SystemMeta, world: &mut World) {
        A::apply(&mut state.first, meta, world);
        if std::mem::take(&mut state.second_lent) {
            B::apply(&mut state.second, meta, world);
        }
    }

    fn queue(state: &mut Self::State, meta: &SystemMeta, mut world: DeferredWorld) {
        A::queue(&mut state.first, meta, world.reborrow());
        if std::mem::take(&mut state.second_lent) {
            B::queue(&mut state.second, meta, world.reborrow());
        }
    }

    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Result<Self::Item<'w, 's>, bevy::ecs::system::SystemParamValidationError> {
        // Both members validated up front, as the set's are ([`disjoint_set!`]).
        let PairState {
            first,
            second,
            validity,
            ..
        } = &mut *state;
        // SAFETY: upheld by the caller, for each member's access as for the pair's.
        unsafe {
            validity.check(world, || {
                let _ = A::get_param(first, meta, world, change_tick)?;
                let _ = B::get_param(second, meta, world, change_tick)?;
                Ok(())
            })
        }?;
        Ok(Pair {
            states: state,
            meta: meta.clone(),
            world,
            change_tick,
            valid: None,
            #[cfg(debug_assertions)]
            lend: None,
        })
    }
}

impl<A: SystemParam, B: SystemParam> Pair<'_, '_, A, B> {
    /// The first member, the second unlent while it lives.
    pub(crate) fn p0(&mut self) -> SystemParamItem<'_, '_, A> {
        #[cfg(debug_assertions)]
        let change_tick = begin_lend(self.world, &mut self.lend, self.change_tick, "p0");
        #[cfg(not(debug_assertions))]
        let change_tick = self.change_tick;
        // SAFETY: the pair registered `A`'s access, and `&mut self` keeps `B` unlent.
        unsafe { A::get_param(&mut self.states.first, &self.meta, self.world, change_tick) }
            .unwrap_or_else(|err| panic!("p0 validated at fetch, then failed: {err}"))
    }

    /// The first member's lend has ended, for the census ([`end_lend`]): its item has dropped.
    #[cfg(debug_assertions)]
    pub(crate) fn end_lend(&mut self) {
        end_lend(self.world, &mut self.lend);
    }

    /// The second member, the first unlent while it lives.
    pub(crate) fn p1(&mut self) -> SystemParamItem<'_, '_, B> {
        self.states.second_lent = true;
        // SAFETY: the pair registered `B`'s access, and `&mut self` keeps `A` unlent.
        unsafe {
            B::get_param(
                &mut self.states.second,
                &self.meta,
                self.world,
                self.change_tick,
            )
        }
        .unwrap_or_else(|err| panic!("p1 validated at fetch, then failed: {err}"))
    }
}

impl<A: SystemParam, T: SystemParam> Pair<'_, '_, A, Option<T>> {
    /// The second member as `p1` lends it, validated once a run: nothing adds or removes a resource
    /// or an entity mid-run.
    pub(crate) fn p1_valid(&mut self) -> Option<SystemParamItem<'_, '_, T>> {
        let (states, meta, world, tick) = (
            &mut self.states.second,
            &self.meta,
            self.world,
            self.change_tick,
        );
        // SAFETY: the pair registered `T`'s access, and `&mut self` keeps `A` unlent. The fetch
        // is the validation, as Bevy has no separate one; its item drops at once.
        let valid = *self
            .valid
            .get_or_insert_with(|| unsafe { T::get_param(states, meta, world, tick) }.is_ok());
        self.states.second_lent |= valid;
        if !valid {
            return None;
        }
        // SAFETY: as above, and `T` validated in this run.
        unsafe { T::get_param(states, meta, world, tick) }.ok()
    }
}

/// Begin a lend of a member, for the census: the one before it is ended, and while the census
/// watches writes ([`benilla_ui::script::census::watching_writes`]) the lend takes a change tick
/// of its own, which only its writes carry, to be found by [`end_lend`]. `system` is the tick the
/// system runs at, which every lend carries otherwise.
#[cfg(debug_assertions)]
pub(crate) fn begin_lend(
    world: UnsafeWorldCell,
    lend: &mut Option<(Tick, &'static str)>,
    system: Tick,
    member: &'static str,
) -> Tick {
    use benilla_ui::script::census;
    end_lend(world, lend);
    if !census::watching_writes() {
        return system;
    }
    census::lend_begins();
    let tick = world.increment_change_tick();
    *lend = Some((tick, member));
    tick
}

/// End the lend begun last, for the census: when a handler is held, it reports whether the lend
/// wrote, and a write that follows the handler's signal panics, unless the member only publishes
/// ([`super::census::EPILOGUE`]). Every item the lend gave has dropped: the lends are `&mut`
/// borrows of the set, and the next begins, or the host ends this one, only once they have ended.
#[cfg(debug_assertions)]
pub(crate) fn end_lend(world: UnsafeWorldCell, lend: &mut Option<(Tick, &'static str)>) {
    use benilla_ui::script::census;
    let Some((tick, member)) = lend.take() else {
        return;
    };
    let after = census::lend_ends(census::window_open() && written_at(world, tick), member);
    if after && !super::census::EPILOGUE.contains(&member) && !std::thread::panicking() {
        panic!(
            "census: `{member}` written after a handler its apply signalled was held, which the \
             reference runs before the write: `SignalEvent` (0x703e50) runs every listener before \
             the verb goes on"
        );
    }
}

/// Whether anything in the world carries `tick` as its change tick: a resource, non-send data, or
/// a table component of any entity. A write through a `ResMut` or a `Mut` stamps the tick its lend was
/// given, so a lend with a tick of its own wrote exactly if something carries it. Commands
/// queued by the lend and sparse-set components are not seen.
#[cfg(debug_assertions)]
fn written_at(world: UnsafeWorldCell, tick: Tick) -> bool {
    // SAFETY: only the change ticks are read, as plain values, with no member of the set lent
    // (the caller's contract above) and no other thread running.
    let storages = unsafe { world.storages() };
    // A resource is a sparse-set component on an entity of its own, found through the world's
    // resource-entity map; non-send data keeps its own storage.
    // SAFETY: as above: the map is read, no resource it names is lent.
    let resource_entities = unsafe { world.resource_entities() };
    let resources = resource_entities
        .iter()
        .filter_map(|(id, entity)| storages.sparse_sets.get(id)?.get_ticks(entity))
        .chain(storages.non_sends.iter().filter_map(|(_, r)| r.get_ticks()));
    for ticks in resources {
        if ticks.changed.get() == tick.get() {
            return true;
        }
    }
    let components = world.components();
    storages.tables.iter().any(|table| {
        components.iter_registered().any(|(id, _)| {
            table
                .get_changed_ticks_slice_for(id)
                // SAFETY: as above.
                .is_some_and(|ticks| {
                    ticks
                        .iter()
                        .any(|t| unsafe { *t.get() }.get() == tick.get())
                })
        })
    })
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use super::Pair;

    #[derive(Resource, Default)]
    struct Gate;

    #[derive(Resource, Default)]
    struct Spare;

    /// Non-send data a set may require.
    #[derive(Default)]
    struct Local;

    #[derive(Resource, Default)]
    struct Runs(u32);

    disjoint_set! {
        /// A set that requires `Gate` and the non-send `Local`, and takes `Spare` if present.
        struct Gated, GatedState {
            gate: Res<'static, Gate>,
            local: NonSend<'static, Local>,
            spare: Option<Res<'static, Spare>>,
            runs: ResMut<'static, Runs>,
        }
    }

    fn count_set(mut set: Gated) {
        let _ = set.gate();
        let _ = set.local();
        let _ = set.spare();
        set.runs().0 += 1;
    }

    fn count_pair(mut pair: Pair<Res<'static, Gate>, ResMut<'static, Runs>>) {
        let _ = pair.p0();
        pair.p1().0 += 1;
    }

    /// Lets a failed validation skip its system, as the client's error handler does, and fails
    /// the test on anything else: a lend that fails after validating is a panic Bevy reports
    /// through the same handler.
    fn skip_only(error: bevy::ecs::error::BevyError, _: bevy::ecs::error::ErrorContext) {
        let error = error.to_string();
        assert!(
            error.contains("failed validation"),
            "a system failed other than by skipping: {error}"
        );
    }

    /// A world where a member's fetch failing skips its system, with one counting system.
    fn gated(system: fn(&mut App)) -> App {
        let mut app = App::new();
        app.set_error_handler(skip_only)
            .init_resource::<Runs>()
            .insert_non_send(Local);
        system(&mut app);
        app
    }

    fn runs(app: &App) -> u32 {
        app.world().resource::<Runs>().0
    }

    /// A set skips while a member's resource is missing and runs while every one exists, whatever
    /// its verdict was the run before: a removal after a run and a re-insert are each seen on the
    /// next run. An optional member's absence skips nothing.
    #[test]
    fn a_set_skips_while_a_member_resource_is_missing_and_runs_once_it_exists() {
        for system in [
            |app: &mut App| {
                app.add_systems(Update, count_set);
            },
            |app: &mut App| {
                app.add_systems(Update, count_pair);
            },
        ] {
            let mut app = gated(system);
            app.update();
            assert_eq!(runs(&app), 0, "no Gate: skipped");
            app.init_resource::<Gate>();
            app.update();
            app.update();
            assert_eq!(runs(&app), 2, "Gate present, Spare absent: runs");
            // Removed by a system after it in the same frame, as a teardown would.
            app.add_systems(Last, |mut commands: Commands, runs: Res<Runs>| {
                if runs.0 == 3 {
                    commands.remove_resource::<Gate>();
                }
            });
            app.update();
            assert_eq!(runs(&app), 3, "the run before the removal");
            app.update();
            app.update();
            assert_eq!(runs(&app), 3, "Gate removed: skipped on the next runs");
            app.init_resource::<Gate>();
            app.update();
            assert_eq!(runs(&app), 4, "Gate back: runs again");
        }
    }

    /// The same for a member's non-send value.
    #[test]
    fn a_set_skips_while_a_member_non_send_value_is_missing() {
        let mut app = gated(|app: &mut App| {
            app.init_resource::<Gate>().add_systems(Update, count_set);
        });
        app.update();
        assert_eq!(runs(&app), 1);
        app.world_mut().remove_non_send::<Local>();
        app.update();
        assert_eq!(runs(&app), 1, "Local removed: skipped");
        app.insert_non_send(Local);
        app.update();
        assert_eq!(runs(&app), 2, "Local back: runs");
    }

    /// A system holds the host's state inline from the moment it is added.
    #[test]
    fn the_hosts_state_is_a_few_words() {
        use crate::script_calls::ScriptCallHost;
        use bevy::ecs::system::SystemParam;
        let size = size_of::<<ScriptCallHost<'static, 'static> as SystemParam>::State>();
        assert!(size <= 64, "{size}");
    }

    #[derive(Resource, Default)]
    struct Cell(u32);

    /// The members of a pair may conflict with one another, as the pair lends one at a time.
    #[test]
    fn a_pairs_members_may_conflict_with_each_other() {
        let mut app = App::new();
        app.init_resource::<Cell>().add_systems(
            Update,
            |mut pair: Pair<ResMut<'static, Cell>, Res<'static, Cell>>| {
                pair.p0().0 += 1;
                assert_eq!(pair.p1().0, 1);
            },
        );
        app.update();
    }

    /// A member that conflicts with a parameter outside its set is refused as the system's state
    /// is built, as `ParamSet` refuses it.
    #[test]
    #[should_panic(expected = "conflicts with a parameter outside its set")]
    fn a_member_conflicting_with_a_parameter_outside_the_set_panics() {
        let mut app = App::new();
        app.init_resource::<Cell>().add_systems(
            Update,
            |_outside: ResMut<Cell>, _pair: Pair<Res<'static, Cell>, Res<'static, Cell>>| {},
        );
        app.update();
    }

    /// Counts its applies.
    #[derive(Default)]
    struct Applies;

    static APPLIES: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

    impl bevy::ecs::system::SystemBuffer for Applies {
        fn apply(&mut self, _: &bevy::ecs::system::SystemMeta, _: &mut World) {
            APPLIES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        fn queue(&mut self, _: &bevy::ecs::system::SystemMeta, _: bevy::ecs::world::DeferredWorld) {
        }
    }

    /// The second member's deferred work goes out after a run that lent it, and not after one that
    /// did not.
    #[test]
    fn the_second_members_buffers_apply_only_after_a_run_that_lent_it() {
        let mut app = App::new();
        app.init_resource::<Cell>().add_systems(
            Update,
            |mut pair: Pair<
                ResMut<'static, Cell>,
                bevy::ecs::system::Deferred<'static, Applies>,
            >| {
                pair.p0().0 += 1;
                if pair.p0().0 == 2 {
                    pair.p1();
                }
            },
        );
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(APPLIES.load(std::sync::atomic::Ordering::Relaxed), 1);
    }

    static SET_APPLIES: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

    /// Counts its applies, for the set.
    #[derive(Default)]
    struct SetApplies;

    impl bevy::ecs::system::SystemBuffer for SetApplies {
        fn apply(&mut self, _: &bevy::ecs::system::SystemMeta, _: &mut World) {
            SET_APPLIES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        fn queue(&mut self, _: &bevy::ecs::system::SystemMeta, _: bevy::ecs::world::DeferredWorld) {
        }
    }

    disjoint_set! {
        /// A set whose second member counts its applies.
        struct Counted, CountedState {
            cell: ResMut<'static, Cell>,
            applies: bevy::ecs::system::Deferred<'static, SetApplies>,
        }
    }

    /// A set member's deferred work goes out after a run that lent it, and not after one that did
    /// not, whatever the set's other members did.
    #[test]
    fn a_set_members_buffers_apply_only_after_a_run_that_lent_it() {
        let mut app = App::new();
        app.init_resource::<Cell>()
            .add_systems(Update, |mut set: Counted| {
                set.cell().0 += 1;
                if set.cell().0 == 2 {
                    set.applies();
                }
            });
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(SET_APPLIES.load(std::sync::atomic::Ordering::Relaxed), 1);
    }

    /// Counts the runs whose deferred work went out by queue, as a one-shot run sends it.
    #[derive(Default)]
    struct Queues;

    #[derive(Resource, Default)]
    struct Queued(u32);

    impl bevy::ecs::system::SystemBuffer for Queues {
        fn apply(&mut self, _: &bevy::ecs::system::SystemMeta, _: &mut World) {}

        fn queue(
            &mut self,
            _: &bevy::ecs::system::SystemMeta,
            mut world: bevy::ecs::world::DeferredWorld,
        ) {
            world.resource_mut::<Queued>().0 += 1;
        }
    }

    disjoint_set! {
        /// A set whose second member counts its queues.
        struct QueuedSet, QueuedSetState {
            cell: ResMut<'static, Cell>,
            queues: bevy::ecs::system::Deferred<'static, Queues>,
        }
    }

    /// A one-shot run queues a pair's second member and a set's member after a run that lent it,
    /// and not after one that did not.
    #[test]
    fn a_one_shot_run_queues_only_the_members_it_lent() {
        let mut world = World::new();
        world.init_resource::<Cell>();
        world.init_resource::<Queued>();
        let pair = world.register_system(
            |mut pair: Pair<
                ResMut<'static, Cell>,
                bevy::ecs::system::Deferred<'static, Queues>,
            >| {
                pair.p0().0 += 1;
                if pair.p0().0 == 2 {
                    pair.p1();
                }
            },
        );
        for _ in 0..3 {
            world.run_system(pair).unwrap();
        }
        assert_eq!(world.resource::<Queued>().0, 1);
        world.resource_mut::<Cell>().0 = 0;
        let set = world.register_system(|mut set: QueuedSet| {
            set.cell().0 += 1;
            if set.cell().0 == 2 {
                set.queues();
            }
        });
        for _ in 0..3 {
            world.run_system(set).unwrap();
        }
        assert_eq!(world.resource::<Queued>().0, 2);
    }

    #[derive(Resource, Default)]
    struct Seen(Vec<bool>);

    /// `p1_valid` lends `None` while its resource is missing, every lend of the run, and the next
    /// run validates again, so it lends the resource once it is there.
    #[test]
    fn p1_valid_validates_once_a_run() {
        let mut app = App::new();
        app.init_resource::<Seen>().add_systems(
            Update,
            |mut pair: Pair<ResMut<'static, Seen>, Option<Res<'static, Cell>>>| {
                let lent = [pair.p1_valid().is_some(), pair.p1_valid().is_some()];
                pair.p0().0.extend(lent);
            },
        );
        app.update();
        app.init_resource::<Cell>();
        app.update();
        assert_eq!(app.world().resource::<Seen>().0, [false, false, true, true]);
    }
}
