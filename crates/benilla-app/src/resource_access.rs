//! A system's access to resources and non-send data, apart from its access to components.
//!
//! Bevy keeps a resource as a component on an entity of its own and non-send data under a
//! component id of its own, so one [`Access`] holds all three. The guards and the pass's order
//! read resources alone: [`resource_access`] narrows a system's access to them, and a query that
//! excludes resource entities (`Without<IsResource>`) reaches none.

use bevy::ecs::component::ComponentId;
use bevy::ecs::query::{Access, FilteredAccess, FilteredAccessSet};
use bevy::ecs::resource::IS_RESOURCE;
use bevy::ecs::system::SystemAccess;
use bevy::prelude::World;

/// Whether `id` names a resource (a component that requires `IsResource`) or non-send data.
pub(crate) fn is_resource(world: &World, id: ComponentId) -> bool {
    world.components().get_info(id).is_some_and(|info| {
        !info.is_send_and_sync()
            || info
                .required_components()
                .iter_ids()
                .any(|r| r == IS_RESOURCE)
    })
}

/// The resources and non-send data `access` declares. An access that reads or writes every
/// component reaches them all; an exclusive system declares none, as with [`declared`], so a
/// caller that cares asks [`SystemAccess::is_exclusive`].
pub(crate) fn resource_access(world: &World, access: &SystemAccess) -> Access {
    match access {
        SystemAccess::None | SystemAccess::Exclusive => Access::default(),
        SystemAccess::Shared(set) => of_set(world, set),
    }
}

/// [`resource_access`] of a set of filtered accesses.
pub(crate) fn of_set(world: &World, set: &FilteredAccessSet) -> Access {
    let mut out = Access::default();
    for filtered in set.filtered_accesses() {
        out.extend(&of_filtered(world, filtered));
    }
    out
}

/// [`resource_access`] of one filtered access.
pub(crate) fn of_filtered(world: &World, filtered: &FilteredAccess) -> Access {
    let mut out = Access::default();
    if filtered.without_filters().any(|id| id == IS_RESOURCE) {
        return out;
    }
    let access = filtered.access();
    // An access bounded by what it leaves out (every component, or all but some) reaches every
    // resource.
    match access.writes().as_finite_set() {
        None => out.write_all(),
        Some(writes) => {
            for id in writes.iter().filter(|id| is_resource(world, *id)) {
                out.add_write(id);
            }
        }
    }
    match access.reads().as_finite_set() {
        None => out.read_all(),
        Some(reads) => {
            for id in reads.iter().filter(|id| is_resource(world, *id)) {
                out.add_read(id);
            }
        }
    }
    out
}

/// The components `access` reaches, of any kind, resources and the resource marker left out.
#[cfg(test)]
pub(crate) fn components<'w>(
    world: &'w World,
    access: &Access,
) -> impl Iterator<Item = bevy::ecs::query::ComponentAccessKind> + 'w {
    use bevy::ecs::query::ComponentAccessKind;
    let kinds: Vec<ComponentAccessKind> = access
        .try_iter_access()
        .map(Iterator::collect)
        .unwrap_or_default();
    kinds.into_iter().filter(move |k| {
        let (ComponentAccessKind::Archetypal(id)
        | ComponentAccessKind::Shared(id)
        | ComponentAccessKind::Exclusive(id)) = *k;
        id != IS_RESOURCE && !is_resource(world, id)
    })
}

/// The resources `access` writes; none when it writes them all.
#[cfg(test)]
pub(crate) fn writes(access: &Access) -> Vec<ComponentId> {
    access
        .writes()
        .as_finite_set()
        .map(|w| w.iter().collect())
        .unwrap_or_default()
}

/// The resources `access` reads and does not write; none when it reads them all.
#[cfg(test)]
pub(crate) fn reads(access: &Access) -> Vec<ComponentId> {
    reads_and_writes(access)
        .into_iter()
        .filter(|id| !access.has_write(*id))
        .collect()
}

/// The resources `access` reads or writes; none when it reads them all.
#[cfg(test)]
pub(crate) fn reads_and_writes(access: &Access) -> Vec<ComponentId> {
    access
        .reads()
        .as_finite_set()
        .map(|r| r.iter().collect())
        .unwrap_or_default()
}

/// The filtered accesses a system declares; an exclusive system declares none, its access being
/// the whole world's, which no comparison reads.
pub(crate) fn declared(access: SystemAccess) -> FilteredAccessSet {
    match access {
        SystemAccess::Shared(set) => set,
        SystemAccess::None | SystemAccess::Exclusive => FilteredAccessSet::new(),
    }
}

/// [`declared`] by reference: an exclusive system's set is empty, never Bevy's `write_all`
/// stand-in (`SystemAccess::to_filtered_access_set`).
#[cfg(test)]
pub(crate) fn declared_set(access: &SystemAccess) -> std::borrow::Cow<'_, FilteredAccessSet> {
    match access {
        SystemAccess::Shared(set) => std::borrow::Cow::Borrowed(set),
        SystemAccess::None | SystemAccess::Exclusive => {
            std::borrow::Cow::Owned(FilteredAccessSet::new())
        }
    }
}

/// The filtered accesses `access` declares, as [`declared`] counts them.
#[cfg(test)]
pub(crate) fn filtered(access: &SystemAccess) -> &[bevy::ecs::query::FilteredAccess] {
    match access {
        SystemAccess::Shared(set) => set.filtered_accesses(),
        SystemAccess::None | SystemAccess::Exclusive => &[],
    }
}
