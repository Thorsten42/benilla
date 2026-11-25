//! ClassicAPI's priority targeting (`target/Priority.cpp`), the 3.80.2 Classic client's candidate
//! list that its soft targets read: each candidate's geometry flags and score from bank 1's
//! weights, the soft order (best first) and [`first`], the best candidate of a kind inside an arc.
//! Pure: the caller gathers the candidates and hands in the view frustum and the line-of-sight ray
//! ([`View`]). Positions are WoW's: x and y on the ground, z up, angles in radians.

use std::f32::consts::PI;

/// A candidate's flag word, 3.80.2's bits.
pub(crate) mod flag {
    pub(crate) const UNIT: u32 = 0x1;
    pub(crate) const PLAYER: u32 = 0x2;
    pub(crate) const GAME_OBJECT: u32 = 0x4;
    // Deviation: party and raid membership are not classified yet; nothing sets these.
    #[allow(dead_code)]
    pub(crate) const PARTY: u32 = 0x8;
    #[allow(dead_code)]
    pub(crate) const RAID: u32 = 0x10;
    pub(crate) const DEAD: u32 = 0x20;
    pub(crate) const ENEMY: u32 = 0x40;
    pub(crate) const FRIEND: u32 = 0x80;
    pub(crate) const INTERACT: u32 = 0x100;
    pub(crate) const OWN_OBJECT: u32 = 0x200;
    pub(crate) const BOSS: u32 = 0x800;
    pub(crate) const PVP_PRIORITY: u32 = 0x4000;
    pub(crate) const COMBAT_LOCK: u32 = 0x8000;
    pub(crate) const POINT_BLANK: u32 = 0x10000;
    pub(crate) const IN_MELEE: u32 = 0x20000;
    pub(crate) const IN_FRUSTUM: u32 = 0x40000;
    pub(crate) const VISIBLE: u32 = 0x80000;
    pub(crate) const IN_RANGE: u32 = 0x100000;
    pub(crate) const EXCLUDED: u32 = 0x200000;
    pub(crate) const IN_CONE: u32 = 0x400000;
    pub(crate) const CAMERA_RAY: u32 = 0x800000;
    pub(crate) const FACING_RAY: u32 = 0x1000000;
    pub(crate) const CRITTER: u32 = 0x2000000;
}

/// Bank 1, soft targeting's (`kBanks[1]`).
pub(crate) struct Bank {
    pub(crate) range_far: f32,
    pub(crate) range_near: f32,
    pub(crate) ang_inner: f32,
    pub(crate) ang_outer: f32,
    pub(crate) exp_a: f32,
    pub(crate) exp_b: f32,
    pub(crate) cone_ang: f32,
    pub(crate) interact_range: f32,
    fwd: f32,
    dist_mix: f32,
    dist_w: f32,
    side_w: f32,
    from_target_w: f32,
    melee: f32,
    face_in: f32,
    face_out: f32,
    cam_in: f32,
    cam_out: f32,
    cone: f32,
    cam_ray: f32,
    close: f32,
    pvp_player: f32,
    pvp_npc: f32,
    pub(crate) combat: f32,
    boss: f32,
    critter: f32,
    no_los: f32,
    own_object: f32,
    casting: f32,
}

const DEG: f32 = PI / 180.0;

pub(crate) const BANK: Bank = Bank {
    range_far: 51.0,
    range_near: 3.0,
    ang_inner: 20.0 * DEG,
    ang_outer: 90.0 * DEG,
    exp_a: 1.0,
    exp_b: 2.0,
    cone_ang: 18.0 * DEG,
    interact_range: 16.0,
    fwd: 200.0,
    dist_mix: 0.5,
    dist_w: -10.0,
    side_w: -10.0,
    from_target_w: -50.0,
    melee: 200.0,
    face_in: 100.0,
    face_out: 400.0,
    cam_in: 300.0,
    cam_out: 500.0,
    cone: 2000.0,
    cam_ray: 0.0,
    close: 1500.0,
    pvp_player: 30000.0,
    pvp_npc: 10000.0,
    combat: 40000.0,
    boss: 1000.0,
    critter: -20000.0,
    no_los: -4000.0,
    own_object: 20000.0,
    casting: 4000.0,
};

const MIN_RADIUS: f32 = 0.5;
const CLOSE_DISTANCE: f32 = 1.5;

type V3 = [f32; 3];

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn wrap(mut a: f32) -> f32 {
    while a > PI {
        a -= 2.0 * PI;
    }
    while a < -PI {
        a += 2.0 * PI;
    }
    a
}

fn clamp01(v: f32) -> f32 {
    v.clamp(0.0, 1.0)
}

/// One gathered candidate: what the classifier made of it and its model box.
#[derive(Clone, Debug, Default)]
pub(crate) struct Candidate {
    pub(crate) guid: u64,
    /// The classification bits (unit, player, enemy, friend, interact, …), before the geometry.
    pub(crate) flags: u32,
    pub(crate) pos: V3,
    /// Half the model box's larger ground side, times the scale.
    pub(crate) radius: f32,
    /// The box's bottom and height above the position.
    pub(crate) base: f32,
    pub(crate) height: f32,
    /// `UNIT_FIELD_BOUNDINGRADIUS`, 0 for a game object.
    pub(crate) reach: f32,
    /// An enemy unit casting or channeling (the tab score's term).
    pub(crate) casting: bool,
    /// An enemy whose target is the player, inside its own melee reach of the player.
    pub(crate) hits_me: bool,
    // Filled by [`geometry`] and [`score`].
    pub(crate) a_face: f32,
    pub(crate) a_cam: f32,
    pub(crate) cone: f32,
    pub(crate) player_los: bool,
    pub(crate) score: f32,
    pub(crate) tab_score: f32,
}

/// The camera as the picker reads it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Camera {
    pub(crate) pos: V3,
    pub(crate) forward: V3,
    pub(crate) left: V3,
    pub(crate) yaw: f32,
}

/// The player and the camera, with the frustum test and the line-of-sight ray.
pub(crate) struct View<'a> {
    pub(crate) player: V3,
    pub(crate) player_base: f32,
    pub(crate) player_height: f32,
    pub(crate) player_reach: f32,
    pub(crate) facing: f32,
    pub(crate) camera: Camera,
    /// The candidate's box inside the view frustum pulled in by 0.2 on every side.
    pub(crate) in_frustum: &'a dyn Fn(&Candidate) -> bool,
    /// Whether the segment between two points is clear of line-of-sight geometry.
    pub(crate) clear: &'a dyn Fn(V3, V3) -> bool,
}

/// `max(rA + rB + 1.333, 5)`, as ClassicAPI measures melee with the bounding radii.
fn melee_range(a: f32, b: f32) -> f32 {
    (a + b + 1.333_33).max(5.0)
}

/// Line of sight is asked of an enemy that is not a player (`NeedsLos`).
fn needs_los(flags: u32) -> bool {
    flags & (flag::ENEMY | flag::PLAYER) == flag::ENEMY
}

/// `Geometry`: the candidate's frustum, visibility, angle, range, cone and ray flags; whether it
/// is a combat-locked enemy inside the cone (the relaxation's input).
pub(crate) fn geometry(view: &View, bank: &Bank, e: &mut Candidate) -> bool {
    if (view.in_frustum)(e) {
        e.flags |= flag::IN_FRUSTUM;
        if !needs_los(e.flags) {
            e.flags |= flag::VISIBLE;
        } else {
            let end = [e.pos[0], e.pos[1], e.pos[2] + e.base + 0.5 * e.height];
            let eye = [
                view.player[0],
                view.player[1],
                view.player[2] + view.player_base + 0.5 * view.player_height,
            ];
            e.player_los = (view.clear)(eye, end);
            if e.player_los || (view.clear)(view.camera.pos, end) {
                e.flags |= flag::VISIBLE;
            }
        }
    }
    let rel = sub(e.pos, view.player);
    let mut d = dot(rel, rel).sqrt();
    let rad = e.radius.max(MIN_RADIUS);
    if d < rad {
        e.flags |= flag::POINT_BLANK;
    }
    let heading = rel[1].atan2(rel[0]);
    e.a_face = wrap(heading - view.facing).abs();
    let cam_angle = wrap(heading - view.camera.yaw);
    e.a_cam = cam_angle.abs();
    if e.flags & (flag::UNIT | flag::DEAD) == flag::UNIT {
        d -= view.player_reach + e.reach;
    }
    let mut hits_me = false;
    if e.flags & flag::ENEMY != 0 {
        if dot(rel, rel) < melee_range(view.player_reach, e.reach).powi(2) {
            e.flags |= flag::IN_MELEE;
        }
        hits_me = e.hits_me;
    }
    if e.a_face > bank.ang_outer
        && e.a_cam > bank.ang_outer
        && !hits_me
        && d >= rad
        && d >= CLOSE_DISTANCE
    {
        e.flags |= flag::EXCLUDED;
    }
    let span = bank.ang_outer - bank.ang_inner;
    let t1 = clamp01((e.a_face - bank.ang_inner) / span);
    let t2 = clamp01((e.a_cam - bank.ang_inner) / span);
    let a = (1.0 - (1.0 - t1) * (1.0 - t1)).min(1.0 - (1.0 - t2) * (1.0 - t2));
    let w = (1.0 - (1.0 - a).powf(bank.exp_b)) * a + a.powf(bank.exp_a) * (1.0 - a);
    if d < (1.0 - w) * bank.range_far + w * bank.range_near || hits_me {
        e.flags |= flag::IN_RANGE;
    }
    e.cone = 0.0;
    let side = if cam_angle < 0.0 { -1.0 } else { 1.0 };
    let edge_x = e.pos[0] - rad * view.camera.left[0] * side - view.player[0];
    let edge_y = e.pos[1] - rad * view.camera.left[1] * side - view.player[1];
    let edge_angle = wrap(edge_y.atan2(edge_x) - view.camera.yaw);
    let crosses = (edge_angle < 0.0) != (cam_angle < 0.0);
    let camera_ray = crosses && e.a_cam < PI / 2.0 && edge_angle.abs() < PI / 2.0;
    if camera_ray {
        e.flags |= flag::CAMERA_RAY;
    }
    let edge_offset = if crosses {
        -edge_angle.abs()
    } else {
        edge_angle.abs()
    };
    if e.a_cam <= bank.cone_ang {
        e.cone = 1.0;
    } else if edge_offset < bank.cone_ang {
        let k = clamp01((e.a_cam - bank.cone_ang) / (e.a_cam - edge_offset));
        e.cone = 1.0 - k;
    }
    if e.cone > 0.0 {
        e.flags |= flag::IN_CONE;
    }
    let (sin_f, cos_f) = view.facing.sin_cos();
    let along = cos_f * rel[0] + sin_f * rel[1];
    let (px, py) = (rel[0] - along * cos_f, rel[1] - along * sin_f);
    if along > 0.0 && px * px + py * py < rad * rad {
        e.flags |= flag::FACING_RAY;
    }
    if e.flags & flag::IN_FRUSTUM != 0
        && d < bank.range_far
        && (camera_ray || e.flags & flag::FACING_RAY != 0 || e.cone > 0.0)
    {
        e.flags = (e.flags & !flag::EXCLUDED) | flag::IN_RANGE;
    }
    e.flags & (flag::IN_CONE | flag::COMBAT_LOCK) == (flag::IN_CONE | flag::COMBAT_LOCK)
}

/// `Score`: bank 1's weighted terms; the tab score adds the casting term.
pub(crate) fn score(view: &View, target: Option<V3>, combat: f32, bank: &Bank, e: &mut Candidate) {
    let to_cam = sub(e.pos, view.camera.pos);
    let cam_len = dot(to_cam, to_cam).sqrt();
    let mut s = if cam_len > 1e-5 {
        dot(to_cam, view.camera.forward) / cam_len * bank.fwd
    } else {
        bank.fwd
    };
    if e.flags & flag::ENEMY != 0 {
        if e.flags & flag::BOSS != 0 {
            s += bank.boss;
        }
        if e.flags & flag::COMBAT_LOCK != 0 {
            s += combat;
        }
    }
    if e.flags & flag::CRITTER != 0 {
        s += bank.critter;
    }
    if e.flags & flag::VISIBLE != 0 && needs_los(e.flags) && !e.player_los {
        s += bank.no_los;
    }
    let rel = sub(e.pos, view.player);
    let d2 = dot(rel, rel);
    let d = d2.sqrt();
    s += ((1.0 - bank.dist_mix) * d + d2 * bank.dist_mix) * bank.dist_w;
    s -= dot(rel, view.camera.left) * bank.side_w;
    if let Some(t) = target {
        let dt = sub(t, e.pos);
        let dt2 = dot(dt, dt);
        if dt2 > 1e-5 {
            s += dt2.sqrt() * bank.from_target_w;
        }
    }
    if e.flags & flag::IN_MELEE != 0 {
        s += bank.melee;
    }
    let span = bank.ang_outer - bank.ang_inner;
    if e.a_face < bank.ang_outer {
        let t = clamp01((e.a_face - bank.ang_inner) / span);
        let k = (1.0 - t) * (1.0 - t);
        s += (1.0 - k) * bank.face_in + k * bank.face_out;
    }
    if e.a_cam < bank.ang_outer {
        let t = clamp01((e.a_cam - bank.ang_inner) / span);
        let k = (1.0 - t) * (1.0 - t);
        s += (1.0 - k) * bank.cam_in + k * bank.cam_out;
    }
    if d < e.radius || d < CLOSE_DISTANCE {
        s += bank.close;
    }
    if e.flags & flag::CAMERA_RAY != 0 {
        s += bank.cam_ray;
    }
    if e.flags & flag::PVP_PRIORITY != 0 {
        s += if e.flags & flag::PLAYER != 0 {
            bank.pvp_player
        } else {
            bank.pvp_npc
        };
    }
    if e.flags & flag::OWN_OBJECT != 0 {
        s += bank.own_object;
    }
    s += bank.cone * e.cone;
    e.score = s;
    e.tab_score = s;
    if e.flags & (flag::UNIT | flag::ENEMY) == (flag::UNIT | flag::ENEMY) && e.casting {
        e.tab_score += bank.casting;
    }
}

/// The rebuilt list: every candidate with its flags and scores, the soft order best first.
#[derive(Default)]
pub(crate) struct List {
    pub(crate) entries: Vec<Candidate>,
    /// Indices into `entries`, best soft score first.
    order: Vec<usize>,
    all_flags: u32,
}

impl List {
    /// `Build` past the gather: the geometry of each candidate, then the scores, the combat lock's
    /// weight relaxed (`relaxation`) unless a combat-locked enemy stands in the cone.
    pub(crate) fn build(
        view: &View,
        target: Option<V3>,
        relaxation: bool,
        mut entries: Vec<Candidate>,
    ) -> Self {
        let bank = &BANK;
        let mut locked_in_cone = false;
        for e in &mut entries {
            locked_in_cone |= geometry(view, bank, e);
        }
        let combat = if locked_in_cone || !relaxation {
            bank.combat
        } else {
            0.0
        };
        let mut all_flags = 0;
        for e in &mut entries {
            score(view, target, combat, bank, e);
            all_flags |= e.flags;
        }
        let mut order: Vec<usize> = (0..entries.len()).collect();
        order.sort_by(|&a, &b| entries[b].score.total_cmp(&entries[a].score));
        Self {
            entries,
            order,
            all_flags,
        }
    }

    /// `FlagsOf`: a candidate's flags, 0 for none.
    pub(crate) fn flags_of(&self, guid: u64) -> u32 {
        self.entries
            .iter()
            .find(|e| e.guid == guid)
            .map_or(0, |e| e.flags)
    }

    /// `First(type, arc)`: the best candidate carrying every `kind` bit, in the frustum, visible,
    /// in range unless an interact pick, not excluded, and inside the arc: 0 point-blank or on the
    /// camera's or facing's line, 1 also inside the cone, 2 anywhere in the frustum.
    pub(crate) fn first(&self, kind: u32, arc: i32) -> Option<u64> {
        if self.all_flags & kind != kind {
            return None;
        }
        let interact = if kind & flag::INTERACT != 0 {
            0
        } else {
            flag::IN_RANGE
        };
        let need = kind | flag::IN_FRUSTUM | flag::VISIBLE | interact;
        let any = match arc {
            0 => flag::POINT_BLANK | flag::CAMERA_RAY | flag::FACING_RAY,
            1 => flag::POINT_BLANK | flag::CAMERA_RAY | flag::FACING_RAY | flag::IN_CONE,
            _ => flag::IN_FRUSTUM,
        };
        self.order
            .iter()
            .map(|&i| &self.entries[i])
            .find(|e| e.flags & flag::EXCLUDED == 0 && e.flags & need == need && e.flags & any != 0)
            .map(|e| e.guid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view<'a>(
        in_frustum: &'a dyn Fn(&Candidate) -> bool,
        clear: &'a dyn Fn(V3, V3) -> bool,
    ) -> View<'a> {
        // The player at the origin facing +x, the camera 5 yd behind and above, looking along +x.
        View {
            player: [0.0, 0.0, 0.0],
            player_base: 0.0,
            player_height: 2.0,
            player_reach: 0.5,
            facing: 0.0,
            camera: Camera {
                pos: [-5.0, 0.0, 2.0],
                forward: [1.0, 0.0, 0.0],
                left: [0.0, 1.0, 0.0],
                yaw: 0.0,
            },
            in_frustum,
            clear,
        }
    }

    fn unit(guid: u64, flags: u32, x: f32, y: f32) -> Candidate {
        Candidate {
            guid,
            flags: flag::UNIT | flags,
            pos: [x, y, 0.0],
            radius: 0.5,
            height: 2.0,
            reach: 0.5,
            ..Default::default()
        }
    }

    /// The enemy straight ahead beats one off to the side; one behind is excluded and never
    /// picked; arc 0 takes only what stands on the facing or camera line.
    #[test]
    fn ahead_beats_aside_and_behind_is_excluded() {
        let frustum = |e: &Candidate| e.pos[0] > -4.0;
        let clear = |_: V3, _: V3| true;
        let v = view(&frustum, &clear);
        let list = List::build(
            &v,
            None,
            true,
            vec![
                unit(1, flag::ENEMY, 10.0, 0.0),
                unit(2, flag::ENEMY, 8.0, 6.0),
                unit(3, flag::ENEMY, -10.0, 0.0),
            ],
        );
        assert_eq!(list.first(flag::ENEMY, 2), Some(1));
        assert_eq!(list.first(flag::ENEMY, 0), Some(1));
        assert_ne!(list.flags_of(3) & flag::EXCLUDED, 0);
        assert_ne!(list.flags_of(1) & flag::FACING_RAY, 0);
        assert_eq!(list.first(flag::FRIEND, 2), None);
    }

    /// An enemy behind a wall for both the player and the camera is not visible and not picked;
    /// a friend needs no line of sight.
    #[test]
    fn an_enemy_out_of_sight_is_not_picked() {
        let frustum = |_: &Candidate| true;
        let wall = |_: V3, to: V3| to[0] < 9.0;
        let v = view(&frustum, &wall);
        let list = List::build(
            &v,
            None,
            true,
            vec![
                unit(1, flag::ENEMY, 10.0, 0.0),
                unit(2, flag::FRIEND, 10.0, 0.5),
            ],
        );
        assert_eq!(list.flags_of(1) & flag::VISIBLE, 0);
        assert_eq!(list.first(flag::ENEMY, 2), None);
        assert_eq!(list.first(flag::FRIEND, 2), Some(2));
    }

    /// An interact pick needs no range bit, and the player's own object (the bobber) outranks the
    /// rest; a critter sorts last.
    #[test]
    fn own_object_first_and_critters_last() {
        let frustum = |_: &Candidate| true;
        let clear = |_: V3, _: V3| true;
        let v = view(&frustum, &clear);
        let mut bobber = unit(5, 0, 14.0, 4.0);
        bobber.flags = flag::GAME_OBJECT | flag::INTERACT | flag::OWN_OBJECT;
        let list = List::build(
            &v,
            None,
            true,
            vec![
                unit(4, flag::INTERACT, 3.0, 0.0),
                bobber,
                unit(6, flag::ENEMY | flag::CRITTER, 4.0, 0.0),
                unit(7, flag::ENEMY, 12.0, 0.0),
            ],
        );
        assert_eq!(list.first(flag::INTERACT, 2), Some(5));
        assert_eq!(list.first(flag::ENEMY, 2), Some(7));
    }
}
