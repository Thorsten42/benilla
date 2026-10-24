//! Which world the mover may resolve against. The reference's world triangle query `0x6aa8b0`
//! fails, and the mover's substep is skipped, on any of three legs: a map object not ready
//! (`0x6aa90c` → `0x6aaab0`, which `0x6aa913` tests before the one-object branch), a terrain
//! chunk under the swept box not loaded (`0x6aadc0`), or a doodad in a gathered chunk not ready
//! (`0x6ab4a1` → `0x6abc40`, 0 at `0x6abcda`). [`publish_terrain_residency`] writes each frame,
//! after the attaches, the tiles that pass the terrain leg, the boxes of the map objects that fail
//! theirs and the chunks whose doodads fail theirs; [`TerrainResidency::gather_ready`] answers all
//! three for a box and a query's class word.
//!
//! Deviation: a model whose file failed to load counts as ready with nothing to collide, and an
//! authored tile whose ADT failed to load counts as resident, where the reference never runs on
//! without them: its WMO root and ADT opens go through `CMap::SafeOpen` (`0x6c38b5`, `0x6c1e85`),
//! which retries ten times and then ends the process (`0x694130`, `0x64d0e0` at `0x694162`); its
//! M2 open's failure is not traced. It differs because a client must not exit over one bad file.

use bevy::platform::collections::HashSet;
use bevy::prelude::*;

use super::collider::PendingCollider;
use super::weld::HullWelds;
use super::{Placements, TerrainStreamer};

/// The tile grid's half extent, `0x7ffab4`: a world coordinate maps to `17066.666 − c`.
const GRID_ORIGIN: f32 = 17066.666;
/// The tile grid's extent, `0x7ffab0`: a mapped coordinate must lie below it.
const GRID_SPAN: f32 = 34133.332;
/// Cells per yard, `0x810ae4`: eight cells to a chunk, sixteen chunks to a tile.
const CELLS_PER_YARD: f32 = 0.24;

/// The class-word bits under which a query takes a map's own doodads: the doodad walks, a map
/// object's group doodads (`0x6aad37`) and a chunk's (`0x6abc4c`), run under 0xf0000f, and take a
/// def with no owner pair `+0x180` under 0xf (`0x6abccf`).
pub const MAP_DOODADS: u32 = 0xf;
/// The class-word bits under which a query takes a game object's doodad def, one with an owner
/// pair `+0x180` (`0x6abcc7`): [`UnloadedObjectDef`].
pub const OBJECT_DOODADS: u32 = 0xf00000;

/// A transport game object's doodad def whose model has not loaded. The reference builds a def
/// only for types 11 and 15 (`0x5f4970` → `0x5f8030`), a doodad def with the object's GUID as its
/// owner pair only for a model that is not a `.wmo` (`0x6707c0`, `0x6708ba`, `0x6708c6`; a `.wmo`
/// becomes a map object def), and the transport tick places it (`0x6709a0` → `0x698e20`):
/// until its model loads it has no flag 0x80 and is boxed at its position with radius 0
/// (`0x698ec1`), so it is filed in the chunk it stands in (`0x6a86d0`) and fails a query that
/// takes game objects there (`0x6abcda`). The bearer's `Transform` is its position; the app
/// removes the marker when the model is built.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct UnloadedObjectDef;

/// The cell a world coordinate falls in on one axis, as `0x6aa9b4` forms it: `17066.666 − v`
/// times 0.24, stored as a float, less 0.5 on the x87 stack and `fistp`'d to nearest-even.
fn cell_of(v: f32) -> i32 {
    let mapped = -(v - GRID_ORIGIN);
    (f64::from(mapped * CELLS_PER_YARD) - 0.5).round_ties_even() as i32
}

/// A chunk's `(column, row)` key on the 1024-chunk grid (`>> 3`, `0x6aaa23`), from the WoW-space
/// `(x, y)` of a point in it: columns run along y and rows along x, as the tile key does.
pub(crate) fn chunk_key(x: f32, y: f32) -> (i32, i32) {
    (cell_of(y) >> 3, cell_of(x) >> 3)
}

/// The tiles whose terrain the mover's world query may gather, as of the last stream stage.
#[derive(Resource, Clone, PartialEq, Debug)]
pub struct TerrainResidency {
    /// Bit `tile_x` of row `tile_y` (the [`benilla_formats::world_to_tile`] key): the tile is
    /// spawned and its terrain and fence colliders are attached, or its ADT failed to load.
    rows: [u64; 64],
    /// The map is one object (WDT `MPHD` bit 0, `[0xc9e384]`): after the map-object leg its query
    /// returns 1 without testing terrain (`0x6aa919`), so terrain gates nothing there.
    one_object: bool,
    /// The world boxes `(min, max)` of the registered map objects whose root is not yet
    /// collidable: not spawned, or a collider of its own still building.
    unready_roots: Vec<[[f32; 3]; 2]>,
    /// The world boxes of the map objects whose props are not yet collidable: not all spawned, a
    /// prop hull still welding, or its weld still building.
    unready_props: Vec<[[f32; 3]; 2]>,
    /// The [`chunk_key`]s of the chunks that list a map doodad not yet collidable.
    unready_doodad_chunks: HashSet<(i32, i32)>,
    /// The [`chunk_key`]s of the chunks an [`UnloadedObjectDef`] stands in.
    unready_object_chunks: HashSet<(i32, i32)>,
}

impl Default for TerrainResidency {
    /// Nothing resident: before the first stream stage, and after leaving the world.
    fn default() -> Self {
        Self {
            rows: [0; 64],
            one_object: false,
            unready_roots: Vec::new(),
            unready_props: Vec::new(),
            unready_doodad_chunks: HashSet::default(),
            unready_object_chunks: HashSet::default(),
        }
    }
}

/// Whether two WoW-space boxes overlap: `0x6acb40` (every min of `a` at most `b`'s max) and
/// `0x699330` (every max at least `b`'s min) on all three axes, a NaN failing it.
fn overlaps([lo, hi]: &[[f32; 3]; 2], min: [f32; 3], max: [f32; 3]) -> bool {
    (0..3).all(|i| lo[i] <= max[i] && hi[i] >= min[i])
}

impl TerrainResidency {
    /// A residency with exactly `tiles` resident, for tests and tools.
    pub fn with_tiles(tiles: impl IntoIterator<Item = (i32, i32)>) -> Self {
        let mut r = Self::default();
        for (tx, ty) in tiles {
            r.insert(tx, ty);
        }
        r
    }

    /// A one-object map's residency, whose terrain gates nothing.
    pub fn one_object() -> Self {
        Self {
            one_object: true,
            ..Self::default()
        }
    }

    /// A residency with every tile resident and the map objects in `boxes` not ready, root and
    /// props, for tests and tools.
    pub fn with_unready_map_objects(boxes: impl IntoIterator<Item = [[f32; 3]; 2]>) -> Self {
        let mut r = Self::with_tiles((0..64).flat_map(|tx| (0..64).map(move |ty| (tx, ty))));
        r.unready_roots = boxes.into_iter().collect();
        r.unready_props = r.unready_roots.clone();
        r
    }

    fn insert(&mut self, tx: i32, ty: i32) {
        if (0..64).contains(&tx) && (0..64).contains(&ty) {
            self.rows[ty as usize] |= 1 << tx;
        }
    }

    /// Whether the tile `(tile_x, tile_y)` is resident.
    pub fn tile(&self, tx: i32, ty: i32) -> bool {
        (0..64).contains(&tx) && (0..64).contains(&ty) && self.rows[ty as usize] & (1 << tx) != 0
    }

    /// Whether the world query `0x6aa8b0` over the WoW-space box `min..max` with the class word
    /// `mask` completes: its map-object leg ([`Self::map_objects_ready`]) and, unless the map is
    /// one object, every chunk under the box loaded ([`Self::box_resident`]) with its doodads
    /// ([`Self::doodads_ready`]).
    pub fn gather_ready(&self, min: [f32; 3], max: [f32; 3], mask: u32) -> bool {
        self.map_objects_ready(min, max, mask)
            && (self.one_object
                || (self.box_resident([min[0], min[1]], [max[0], max[1]])
                    && self.doodads_ready(min, max, mask)))
    }

    /// The map-object leg, `0x6aaab0`: the gather fails while a map object whose box overlaps the
    /// query's ([`overlaps`]) has no loaded root (`+0x1d4`, `0x6aabaa`) or no def-group list
    /// (`+0x138` tagged or null, `0x6aabb8`-`0x6aabc8`), and, under [`MAP_DOODADS`], while a
    /// doodad of one of its groups the query's box meets (`0x6aad0c`, `0x6aad1b`) is not loaded.
    /// benilla builds a building's groups and their defs with its root, so a spawned root has
    /// both, and gates its props as one set over the building's box.
    pub fn map_objects_ready(&self, min: [f32; 3], max: [f32; 3], mask: u32) -> bool {
        !self.unready_roots.iter().any(|b| overlaps(b, min, max))
            && (mask & MAP_DOODADS == 0
                || !self.unready_props.iter().any(|b| overlaps(b, min, max)))
    }

    /// The doodad leg of every chunk under the box (`0x6ab4a1` → `0x6abc40`): a chunk fails at
    /// the first doodad of its list that is not loaded (bit 0x80 of `+0xc`, `0x6abcda`), whether
    /// or not its box meets the query's, a map doodad under [`MAP_DOODADS`] and a game object's
    /// under [`OBJECT_DOODADS`].
    pub fn doodads_ready(&self, min: [f32; 3], max: [f32; 3], mask: u32) -> bool {
        let sets = [
            (MAP_DOODADS, &self.unready_doodad_chunks),
            (OBJECT_DOODADS, &self.unready_object_chunks),
        ];
        let (col0, row0) = chunk_key(max[0], max[1]);
        let (col1, row1) = chunk_key(min[0], min[1]);
        sets.into_iter()
            .filter(|(bits, set)| mask & bits != 0 && !set.is_empty())
            .all(|(_, set)| {
                !(row0..=row1).any(|row| (col0..=col1).any(|col| set.contains(&(col, row))))
            })
    }

    /// The terrain leg of `0x6aa8b0` for the WoW-space box `min..max` (x, y; z is not read): every
    /// chunk the box's cells touch must be loaded (`0x6aadc0` returns 0 for a null tile at
    /// `0x6aadfa` and a null chunk at `0x6aae1f`), and the box must lie on the grid (`0x6aa967`,
    /// `0x6aa98c`). benilla loads whole tiles, so a chunk is loaded when its tile is resident.
    pub fn box_resident(&self, min: [f32; 2], max: [f32; 2]) -> bool {
        if self.one_object {
            return true;
        }
        // `fsub 17066.666; fchs` per corner, so the max corner gives the low cell.
        let lo = [-(max[0] - GRID_ORIGIN), -(max[1] - GRID_ORIGIN)];
        let hi = [-(min[0] - GRID_ORIGIN), -(min[1] - GRID_ORIGIN)];
        // `0x6aa967`: off the grid's north or west edge. The reference passes a NaN on; no
        // finite position makes one, and it is refused here.
        if !(lo[0] >= 0.0 && lo[1] >= 0.0) {
            return false;
        }
        // `0x6aa98c`: off the south or east edge.
        if !(hi[0] < GRID_SPAN && hi[1] < GRID_SPAN) {
            return false;
        }
        // `>> 3` to the chunk (`0x6aaa23`) and `>> 4` to the tile (`0x6aaddc`).
        let tile = |v: f32| (cell_of(v) >> 3) >> 4;
        // Rows run along x and columns along y; the tile key is (column, row).
        let (row0, row1) = (tile(max[0]), tile(min[0]));
        let (col0, col1) = (tile(max[1]), tile(min[1]));
        (row0..=row1).all(|row| (col0..=col1).all(|col| self.tile(col, row)))
    }
}

/// Publishes [`TerrainResidency`] at the end of the stream stage, so the next frame's mover reads
/// the tiles whose colliders are attached by then: a tile counts once its root is spawned and
/// neither its terrain nor its fence collider is still building, or once its ADT's load failed
/// (the module's `Deviation:`). A tile the WDT does not author is never requested, so never
/// resident.
///
/// A placement counts as loaded once it can collide: a building's root once spawned with its own
/// colliders built, its props once all spawned with their welded hulls built; a map doodad once
/// spawned with its tile's hull welds built. Only placements not yet latched ready are looked at
/// ([`Placements::unsettled`]), each leaving the set once ready.
pub(super) fn publish_terrain_residency(
    state: Res<TerrainStreamer>,
    placements: ResMut<Placements>,
    welds: Res<HullWelds>,
    pending: Query<(), With<PendingCollider>>,
    unloaded_objects: Query<&Transform, With<UnloadedObjectDef>>,
    asset_server: Res<AssetServer>,
    mut residency: ResMut<TerrainResidency>,
) {
    let mut next = TerrainResidency {
        one_object: state.one_object,
        ..TerrainResidency::default()
    };
    let placements = placements.into_inner();
    let (by_id, unsettled) = (&placements.by_id, &mut placements.unsettled);
    let building = |es: &[Entity]| es.iter().any(|&e| pending.contains(e));
    unsettled.retain(|uid| {
        let Some(p) = by_id.get(uid) else {
            return false; // released
        };
        match p.map_box {
            Some(b) => {
                let root_n = p.root_entities.min(p.entities.len());
                let root = p.spawned && !building(&p.entities[..root_n]);
                let props = p.spawned
                    && p.doodads.iter().all(|d| d.spawned)
                    && !welds.welding_prop(*uid)
                    && !building(&p.entities[root_n..]);
                if !root {
                    next.unready_roots.push(b);
                }
                if !props {
                    next.unready_props.push(b);
                }
                !(root && props)
            }
            None => {
                let ready = p.spawned
                    && !welds.welding_tile(p.owner)
                    && state
                        .tiles
                        .get(&p.owner)
                        .is_none_or(|t| !building(&t.welds))
                    && !building(&p.entities);
                if !ready {
                    next.unready_doodad_chunks.extend(p.chunks.iter().copied());
                }
                !ready
            }
        }
    });
    for t in &unloaded_objects {
        let [x, y, _] = benilla_assets::coords::bevy_to_wow(t.translation);
        next.unready_object_chunks.insert(chunk_key(x, y));
    }
    for (&(tx, ty), t) in &state.tiles {
        let Some(root) = t.entity else {
            if matches!(
                asset_server.load_state(&t.handle),
                bevy::asset::LoadState::Failed(_)
            ) {
                next.insert(tx, ty);
            }
            continue;
        };
        if pending.contains(root) || t.wall.is_some_and(|w| pending.contains(w)) {
            continue;
        }
        next.insert(tx, ty);
    }
    if *residency != next {
        *residency = next;
    }
}

#[cfg(test)]
mod tests {
    use super::super::spawn::prop_light::{PropLight, WmoDoodadInst};
    use super::super::{ModelHandle, Placement};
    use super::*;
    use benilla_formats::{tile_to_world, world_to_tile, TILE_SIZE};

    /// The WoW-space box of half-width `half` around `(x, y)`.
    fn around(x: f32, y: f32, half: f32) -> ([f32; 2], [f32; 2]) {
        ([x - half, y - half], [x + half, y + half])
    }

    #[test]
    fn a_box_inside_one_resident_tile_is_resident_and_its_neighbour_is_not() {
        let r = TerrainResidency::with_tiles([(32, 48)]);
        let (x, y) = tile_to_world(32, 48);
        // `tile_to_world` gives the tile's north-west corner; step a quarter tile into it.
        let (cx, cy) = (x - TILE_SIZE * 0.25, y - TILE_SIZE * 0.25);
        assert_eq!(world_to_tile(cx, cy), (32, 48));
        let (lo, hi) = around(cx, cy, 2.0);
        assert!(r.box_resident(lo, hi));
        // One tile south (−x is the next row) is not.
        let (lo, hi) = around(cx - TILE_SIZE, cy, 2.0);
        assert!(!r.box_resident(lo, hi));
        // A box straddling the row line needs both tiles.
        let (lo, hi) = around(x - TILE_SIZE, cy, 2.0);
        assert!(!r.box_resident(lo, hi));
        let both = TerrainResidency::with_tiles([(32, 48), (32, 49)]);
        assert!(both.box_resident(lo, hi));
    }

    #[test]
    fn a_box_off_the_grid_is_never_resident() {
        let every =
            TerrainResidency::with_tiles((0..64).flat_map(|tx| (0..64).map(move |ty| (tx, ty))));
        assert!(every.box_resident([-10.0, -10.0], [10.0, 10.0]));
        assert!(
            !every.box_resident([17060.0, 0.0], [17070.0, 1.0]),
            "north edge"
        );
        assert!(
            !every.box_resident([-17070.0, 0.0], [-17060.0, 1.0]),
            "south edge"
        );
        assert!(
            !every.box_resident([0.0, 17060.0], [1.0, 17070.0]),
            "west edge"
        );
        assert!(
            !every.box_resident([0.0, -17070.0], [1.0, -17060.0]),
            "east edge"
        );
    }

    /// A spawned tile counts once neither its terrain nor its fence collider is still building;
    /// a tile still being read, or never authored, never does.
    #[test]
    fn a_tile_counts_once_its_colliders_are_attached() {
        use super::super::TileState;
        let mut app = test_app();
        let pending =
            || PendingCollider::ready(avian3d::prelude::Collider::sphere(1.0), None, true);
        let root = app.world_mut().spawn(pending()).id();
        let wall = app.world_mut().spawn(pending()).id();
        let mut state = TerrainStreamer::default();
        let mut spawned = TileState::requested(Handle::default());
        spawned.entity = Some(root);
        spawned.wall = Some(wall);
        state.tiles.insert((32, 48), spawned);
        state
            .tiles
            .insert((33, 48), TileState::requested(Handle::default()));
        app.insert_resource(state);

        app.update();
        assert!(
            !app.world().resource::<TerrainResidency>().tile(32, 48),
            "both still build"
        );
        app.world_mut().entity_mut(root).remove::<PendingCollider>();
        app.update();
        assert!(
            !app.world().resource::<TerrainResidency>().tile(32, 48),
            "the fence builds"
        );
        app.world_mut().entity_mut(wall).remove::<PendingCollider>();
        app.update();
        let r = app.world().resource::<TerrainResidency>();
        assert!(r.tile(32, 48), "spawned with both colliders attached");
        assert!(!r.tile(33, 48), "a tile still being read is not resident");
        assert!(!r.tile(34, 48), "nor is one never requested");
    }

    /// The publish under a headless asset server, so a tile's load state is real.
    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<benilla_assets::AdtTile>()
            .init_resource::<TerrainResidency>()
            .init_resource::<Placements>()
            .init_resource::<HullWelds>()
            .init_resource::<TerrainStreamer>()
            .add_systems(Update, publish_terrain_residency);
        app
    }

    /// An authored tile whose ADT fails to load is walked on, not walled off.
    #[test]
    fn a_tile_whose_adt_failed_to_load_counts_as_resident() {
        use super::super::TileState;
        let mut app = test_app();
        let handle = app
            .world()
            .resource::<AssetServer>()
            .load::<benilla_assets::AdtTile>("no_such_dir/no_such_tile.adt");
        let mut state = TerrainStreamer::default();
        state
            .tiles
            .insert((32, 48), TileState::requested(handle.clone()));
        app.insert_resource(state);
        let failed = |app: &App| {
            matches!(
                app.world().resource::<AssetServer>().load_state(&handle),
                bevy::asset::LoadState::Failed(_)
            )
        };
        for _ in 0..400 {
            if failed(&app) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
            app.update();
        }
        assert!(failed(&app), "the read never failed");
        app.update();
        assert!(
            app.world().resource::<TerrainResidency>().tile(32, 48),
            "a failed read keeps the old walk-on behaviour"
        );
        // A doodad standing on it shades as on a null tile, so it spawns and stops holding the
        // chunks that list it; on a tile still being read it waits.
        let (x, y) = tile_to_world(32, 48);
        let on_it = benilla_assets::coords::wow_to_bevy([x - 10.0, y - 10.0, 0.0]);
        let world = app.world();
        let shade = |state: &TerrainStreamer| {
            super::super::doodad_ground_shade(
                state,
                world.resource::<Assets<benilla_assets::AdtTile>>(),
                world.resource::<AssetServer>(),
                on_it,
            )
        };
        assert!(matches!(
            shade(world.resource::<TerrainStreamer>()),
            super::super::ShadeResolve::Ready(false)
        ));
        let mut reading = TerrainStreamer::default();
        let in_flight = world
            .resource::<AssetServer>()
            .load::<benilla_assets::AdtTile>("x/y.adt");
        reading
            .tiles
            .insert((32, 48), TileState::requested(in_flight));
        assert!(matches!(
            shade(&reading),
            super::super::ShadeResolve::Pending
        ));
    }

    /// A failed read counts as done for the load, as benilla walks on without the tile (the
    /// module's `Deviation:`; the reference's open ends the process, `0x694130`), so a hold on the
    /// destination ends; one still being read does not.
    #[test]
    fn a_failed_read_counts_as_done_for_the_load() {
        use super::super::{tile_done, TileState};
        let mut app = test_app();
        app.init_resource::<TerrainStreamer>();
        let server = app.world().resource::<AssetServer>().clone();
        let handle = server.load::<benilla_assets::AdtTile>("no_such_dir/no_such_tile.adt");
        for _ in 0..400 {
            if matches!(
                server.load_state(&handle),
                bevy::asset::LoadState::Failed(_)
            ) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
            app.update();
        }
        let failed = TileState::requested(handle);
        assert!(tile_done(&failed, &server), "a failed read is done");
        let reading = TileState::requested(server.load::<benilla_assets::AdtTile>("x/y.adt"));
        assert!(!tile_done(&reading, &server), "a read in flight is not");
    }

    /// The mover's class word: the doodad legs run under it (`0x6315f0`, 0x1 of 0xf0000f).
    const MOVER: u32 = crate::collision::WorldCollision::UNIT_MOVER_CLASS;
    /// The liquid-only word, with no doodad bit (`0x633e90`).
    const LIQUID: u32 = 0x30000;
    /// A word that takes only a game object's doodads (0xf00000, `0x6abcc7`), not the map's.
    const GAME_OBJECTS: u32 = 0x100000;

    fn wmo(spawned: bool, entities: Vec<Entity>, doodads: Vec<WmoDoodadInst>) -> Placement {
        Placement {
            uid: 7,
            ident: Default::default(),
            model: ModelHandle::Wmo(Default::default()),
            transform: Transform::IDENTITY,
            root_entities: entities.len(),
            entities,
            spawned,
            doodad_set: 0,
            name_set: 0,
            doodads,
            portal_instance: None,
            refs: 1,
            owner: (0, 0),
            map_box: Some([[0.0, 0.0, 0.0], [10.0, 10.0, 5.0]]),
            chunks: Vec::new(),
        }
    }

    fn prop(spawned: bool) -> WmoDoodadInst {
        WmoDoodadInst {
            handle: Default::default(),
            transform: Transform::IDENTITY,
            groups: std::sync::Arc::from([0u16].as_slice()),
            light: PropLight::Exterior,
            spawned,
        }
    }

    /// Registers `p` as placement 7, unsettled, and publishes.
    fn publish(app: &mut App, p: Placement) {
        let mut pl = app.world_mut().resource_mut::<Placements>();
        pl.by_id.insert(7, p);
        pl.unsettled.insert(7);
        app.update();
    }

    fn residency(app: &App) -> &TerrainResidency {
        app.world().resource::<TerrainResidency>()
    }

    fn pending_collider(app: &mut App) -> Entity {
        app.world_mut()
            .spawn(PendingCollider::ready(
                avian3d::prelude::Collider::sphere(1.0),
                None,
                true,
            ))
            .id()
    }

    const INSIDE: ([f32; 3], [f32; 3]) = ([4.0, 4.0, 1.0], [5.0, 5.0, 3.0]);

    /// A building's root fails every query over its box until it has spawned with its colliders
    /// built (`0x6aabaa`); a box beside or above it passes, the overlap being three-axis
    /// (`0x6acb40`, `0x699330`).
    #[test]
    fn a_map_object_holds_the_query_over_its_box_until_it_can_collide() {
        let mut app = test_app();
        let (lo, hi) = INSIDE;
        publish(&mut app, wmo(false, Vec::new(), Vec::new()));
        assert!(
            !residency(&app).map_objects_ready(lo, hi, LIQUID),
            "loading"
        );
        let beside = residency(&app).map_objects_ready([20.0, 4.0, 1.0], [21.0, 5.0, 3.0], MOVER);
        assert!(beside, "a box beside it");
        let above = residency(&app).map_objects_ready([4.0, 4.0, 6.0], [5.0, 5.0, 8.0], MOVER);
        assert!(above, "a box above it on z");

        let collider = pending_collider(&mut app);
        publish(&mut app, wmo(true, vec![collider], Vec::new()));
        assert!(
            !residency(&app).map_objects_ready(lo, hi, LIQUID),
            "its collider builds"
        );

        app.world_mut()
            .entity_mut(collider)
            .remove::<PendingCollider>();
        app.update();
        assert!(
            residency(&app).map_objects_ready(lo, hi, MOVER),
            "collidable"
        );
        assert!(
            app.world().resource::<Placements>().unsettled.is_empty(),
            "a ready placement leaves the set"
        );
    }

    /// Its props gate only a query that takes doodads (`0x6aad37`): until all have spawned, and
    /// while their hulls weld.
    #[test]
    fn a_map_objects_props_hold_only_a_query_that_takes_doodads() {
        let mut app = test_app();
        let (lo, hi) = INSIDE;
        publish(
            &mut app,
            wmo(true, Vec::new(), vec![prop(true), prop(false)]),
        );
        assert!(
            !residency(&app).map_objects_ready(lo, hi, MOVER),
            "a prop loading"
        );
        assert!(
            residency(&app).map_objects_ready(lo, hi, LIQUID),
            "no doodad bit"
        );
        assert!(
            residency(&app).map_objects_ready(lo, hi, GAME_OBJECTS),
            "a static prop is taken under 0xf alone (`0x6abccf`)"
        );

        app.world_mut().resource_mut::<HullWelds>().add_prop(
            7,
            vec![Vec3::ZERO; 3],
            vec![[0, 1, 2]],
        );
        publish(
            &mut app,
            wmo(true, Vec::new(), vec![prop(true), prop(true)]),
        );
        assert!(
            !residency(&app).map_objects_ready(lo, hi, MOVER),
            "its hulls weld"
        );

        app.world_mut().resource_mut::<HullWelds>().clear();
        app.update();
        assert!(residency(&app).map_objects_ready(lo, hi, MOVER), "all in");
    }

    /// A building's root and its props gate apart: a prop's collider still building holds only a
    /// query that takes doodads, the root's every query.
    #[test]
    fn a_map_objects_root_and_props_gate_apart() {
        let mut app = test_app();
        let (lo, hi) = INSIDE;
        let root = app.world_mut().spawn_empty().id();
        let prop_weld = pending_collider(&mut app);
        let p = Placement {
            root_entities: 1,
            ..wmo(true, vec![root, prop_weld], vec![prop(true)])
        };
        publish(&mut app, p);
        assert!(
            residency(&app).map_objects_ready(lo, hi, LIQUID),
            "the root can collide"
        );
        assert!(
            !residency(&app).map_objects_ready(lo, hi, MOVER),
            "a prop's collider builds"
        );
        let root_pending = pending_collider(&mut app);
        let p = Placement {
            root_entities: 1,
            ..wmo(true, vec![root_pending, root], vec![prop(true)])
        };
        publish(&mut app, p);
        assert!(
            !residency(&app).map_objects_ready(lo, hi, LIQUID),
            "the root's collider builds"
        );
    }

    /// A chunk fails a doodad-taking query while a doodad its list names has not loaded
    /// (`0x6abc40`), wherever the doodad stands in it; its tile's hull welds count.
    #[test]
    fn a_doodad_holds_the_query_over_the_chunks_that_list_it() {
        let mut app = test_app();
        let key = chunk_key(4.0, 4.0);
        let doodad = |spawned: bool| Placement {
            model: ModelHandle::M2(Default::default()),
            map_box: None,
            chunks: vec![key],
            ..wmo(spawned, Vec::new(), Vec::new())
        };
        let every =
            TerrainResidency::with_tiles((0..64).flat_map(|x| (0..64).map(move |y| (x, y))));
        let gather = |app: &App, lo: [f32; 3], hi: [f32; 3], mask: u32| {
            let mut r = residency(app).clone();
            r.rows = every.rows;
            r.gather_ready(lo, hi, mask)
        };
        let (lo, hi) = ([1.0, 1.0, -50.0], [2.0, 2.0, 50.0]);
        publish(&mut app, doodad(false));
        assert!(
            !gather(&app, lo, hi, MOVER),
            "a doodad loading in the chunk"
        );
        assert!(
            gather(&app, lo, hi, LIQUID),
            "the liquid word takes no doodads"
        );
        assert!(
            gather(&app, lo, hi, GAME_OBJECTS),
            "a map doodad is taken under 0xf alone (`0x6abccf`)"
        );
        assert!(
            gather(&app, [-40.0, 1.0, 0.0], [-39.0, 2.0, 1.0], MOVER),
            "another chunk"
        );

        app.world_mut().resource_mut::<HullWelds>().add_tile(
            (0, 0),
            vec![Vec3::ZERO; 3],
            vec![[0, 1, 2]],
        );
        publish(&mut app, doodad(true));
        assert!(!gather(&app, lo, hi, MOVER), "its tile's hulls weld");

        app.world_mut().resource_mut::<HullWelds>().clear();
        app.update();
        assert!(gather(&app, lo, hi, MOVER), "in");
    }

    /// A transport's def fails a query that takes game objects over the chunk it stands in until
    /// its model loads (`0x6abcc7`, `0x6abcda`); a word with only the map's bits passes it.
    #[test]
    fn an_unloaded_transport_def_holds_the_chunk_it_stands_in() {
        let mut app = test_app();
        let at = benilla_assets::coords::wow_to_bevy([4.0, 4.0, 0.0]);
        let lift = app
            .world_mut()
            .spawn((UnloadedObjectDef, Transform::from_translation(at)))
            .id();
        app.update();
        let every =
            TerrainResidency::with_tiles((0..64).flat_map(|x| (0..64).map(move |y| (x, y))));
        let gather = |app: &App, lo: [f32; 3], hi: [f32; 3], mask: u32| {
            let mut r = residency(app).clone();
            r.rows = every.rows;
            r.gather_ready(lo, hi, mask)
        };
        let (lo, hi) = ([1.0, 1.0, -50.0], [2.0, 2.0, 50.0]);
        assert!(!gather(&app, lo, hi, MOVER), "the lift's chunk");
        assert!(!gather(&app, lo, hi, GAME_OBJECTS), "under 0xf00000 alone");
        assert!(gather(&app, lo, hi, MAP_DOODADS), "not under the map's 0xf");
        assert!(
            gather(&app, [-40.0, 1.0, 0.0], [-39.0, 2.0, 1.0], MOVER),
            "another chunk"
        );
        app.world_mut()
            .entity_mut(lift)
            .remove::<UnloadedObjectDef>();
        app.update();
        assert!(gather(&app, lo, hi, MOVER), "its model loaded");
    }

    /// `gather_ready` is the three legs: a map object fails it on a one-object map too, and terrain
    /// fails it off a resident tile.
    #[test]
    fn gather_ready_takes_every_leg() {
        let pier = [[0.0, 0.0, 0.0], [10.0, 10.0, 5.0]];
        let r = TerrainResidency::with_unready_map_objects([pier]);
        let (lo, hi) = INSIDE;
        assert!(!r.gather_ready(lo, hi, MOVER), "the map object");
        assert!(r.gather_ready([20.0, 20.0, 0.0], [21.0, 21.0, 1.0], MOVER));
        let one = TerrainResidency {
            unready_roots: vec![pier],
            ..TerrainResidency::one_object()
        };
        assert!(
            !one.gather_ready(lo, hi, LIQUID),
            "a one-object map still tests its object"
        );
        assert!(one.gather_ready([20.0, 20.0, 0.0], [21.0, 21.0, 1.0], MOVER));
        assert!(
            !TerrainResidency::default().gather_ready(lo, hi, MOVER),
            "no terrain resident"
        );
    }

    /// A chunk's key is the streaming grid's chunk (`0x672730`'s axes): columns along y, rows
    /// along x, so a chunk listed by its centre is the chunk the box test finds (off a chunk edge,
    /// where `0x6aa9b4`'s rounding and the grid's truncation may part).
    #[test]
    fn a_chunk_key_is_the_streaming_grids_chunk() {
        for (x, y) in [(10.0, -20.0), (-1234.5, 678.9), (4010.0, 3990.0)] {
            let (cx, cy) = benilla_formats::world_to_chunk(x, y);
            assert_eq!(chunk_key(x, y), (cx as i32, cy as i32), "at ({x}, {y})");
        }
    }

    #[test]
    fn a_one_object_map_gates_nothing() {
        let r = TerrainResidency::one_object();
        assert!(r.box_resident([-10.0, -10.0], [10.0, 10.0]));
        assert!(r.box_resident([17060.0, 0.0], [17070.0, 1.0]));
        assert!(!TerrainResidency::default().box_resident([-10.0, -10.0], [10.0, 10.0]));
    }
}
