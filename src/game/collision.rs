// Server-side shot occlusion: does a wall block the line from shooter to target?
//
// Walls are axis-aligned boxes that may be rotated about the Y axis (the map's
// diagonal walls). To test a segment against a rotated box we transform the
// segment into the box's LOCAL space (un-rotate about Y around the box center),
// where the box becomes axis-aligned, then run a standard slab (ray-AABB) test.
//
// We only care about XZ occlusion plus a Y band (so a shot under/over a wall
// isn't wrongly blocked). Players aim roughly at torso/head height, well within
// every wall's Y span, so the Y band mostly guards against future low cover.

// A static map wall (generated from the frontend MapBuilder — see MAP_WALLS).
#[derive(Debug, Clone, Copy)]
pub struct MapWall {
    pub cx: f32,
    pub cz: f32,
    pub hw: f32,   // half-extent along the wall's LOCAL x (width/2)
    pub hd: f32,   // half-extent along the wall's LOCAL z (depth/2)
    pub rot: f32,  // rotation about Y, radians
    pub y_min: f32,
    pub y_max: f32,
}

// A wall placed by a player during build mode (+ its bullet holes, for Phase 2).
#[derive(Debug, Clone)]
pub struct Wall {
    pub cx: f32,
    pub cz: f32,
    pub hw: f32,
    pub hd: f32,
    pub rot: f32,
    pub y_min: f32,
    pub y_max: f32,
    pub destructible: bool,
    // Bullet holes in LOCAL wall coords (x along width, y vertical). Phase 2.
    pub holes: Vec<Hole>,
}

#[derive(Debug, Clone, Copy)]
pub struct Hole {
    pub lx: f32, // local x (along width)
    pub ly: f32, // local y (vertical)
    pub radius: f32,
}

impl Wall {
    pub fn as_box(&self) -> BoxParams {
        BoxParams {
            cx: self.cx, cz: self.cz, hw: self.hw, hd: self.hd,
            rot: self.rot, y_min: self.y_min, y_max: self.y_max,
        }
    }
}

impl MapWall {
    pub fn as_box(&self) -> BoxParams {
        BoxParams {
            cx: self.cx, cz: self.cz, hw: self.hw, hd: self.hd,
            rot: self.rot, y_min: self.y_min, y_max: self.y_max,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BoxParams {
    pub cx: f32,
    pub cz: f32,
    pub hw: f32,
    pub hd: f32,
    pub rot: f32,
    pub y_min: f32,
    pub y_max: f32,
}

/// Does the segment from `s` (shooter) to `t` (target) intersect this box?
/// Returns Some(local_hit_xy) at the entry point in the box's LOCAL frame
/// (x along width, y vertical) if it hits within [0,1] of the segment, else None.
/// The local hit lets Phase 2 check whether the entry lands inside a bullet hole.
pub fn segment_hits_box(s: (f32, f32, f32), t: (f32, f32, f32), b: &BoxParams) -> Option<(f32, f32)> {
    // Translate so the box center is the origin, then un-rotate about Y by -rot
    // so the box is axis-aligned. (Y is unchanged by a Y-rotation.)
    let (sin, cos) = (-b.rot).sin_cos();
    let local = |p: (f32, f32, f32)| {
        let dx = p.0 - b.cx;
        let dz = p.2 - b.cz;
        // rotate (dx,dz) by -rot:  x' = dx*cos - dz*sin ; z' = dx*sin + dz*cos
        let lx = dx * cos - dz * sin;
        let lz = dx * sin + dz * cos;
        (lx, p.1, lz)
    };
    let ls = local(s);
    let lt = local(t);
    let d = (lt.0 - ls.0, lt.1 - ls.1, lt.2 - ls.2);

    // Slab test on the axis-aligned box [-hw,hw] x [y_min - cy? ...] — note Y is
    // absolute (not centered), so use y_min/y_max directly. X/Z are centered.
    let mut tmin = 0.0f32;
    let mut tmax = 1.0f32;

    // Helper: clip the segment parameter range against one axis slab [lo,hi].
    let clip = |origin: f32, dir: f32, lo: f32, hi: f32, tmin: &mut f32, tmax: &mut f32| -> bool {
        if dir.abs() < 1e-8 {
            // Parallel to slab: must already be inside.
            return origin >= lo && origin <= hi;
        }
        let mut t0 = (lo - origin) / dir;
        let mut t1 = (hi - origin) / dir;
        if t0 > t1 { std::mem::swap(&mut t0, &mut t1); }
        if t0 > *tmin { *tmin = t0; }
        if t1 < *tmax { *tmax = t1; }
        *tmin <= *tmax
    };

    if !clip(ls.0, d.0, -b.hw, b.hw, &mut tmin, &mut tmax) { return None; }   // local X (width)
    if !clip(ls.2, d.2, -b.hd, b.hd, &mut tmin, &mut tmax) { return None; }   // local Z (depth)
    if !clip(ls.1, d.1, b.y_min, b.y_max, &mut tmin, &mut tmax) { return None; } // absolute Y

    if tmin > tmax || tmax < 0.0 || tmin > 1.0 {
        return None;
    }
    // Entry point in local frame (clamp tmin to >=0 in case shooter is inside).
    let te = tmin.max(0.0);
    let hit_lx = ls.0 + d.0 * te;
    let hit_ly = ls.1 + d.1 * te;
    Some((hit_lx, hit_ly))
}

/// Is the shot from `shooter` to `target` blocked by any wall?
/// `placed` are the build-mode walls (with holes). Map walls are always solid.
/// A destructible wall does NOT block if the entry point falls inside a hole.
pub fn shot_blocked(
    shooter: (f32, f32, f32),
    target: (f32, f32, f32),
    placed: &[Wall],
) -> bool {
    // Static map walls — always solid.
    for mw in MAP_WALLS {
        if segment_hits_box(shooter, target, &mw.as_box()).is_some() {
            return true;
        }
    }
    // Player-placed walls — solid unless the entry lands in a bullet hole.
    for w in placed {
        if let Some((lx, ly)) = segment_hits_box(shooter, target, &w.as_box()) {
            if w.destructible && in_hole(lx, ly, &w.holes) {
                continue; // shot passes through the hole
            }
            return true;
        }
    }
    false
}

fn in_hole(lx: f32, ly: f32, holes: &[Hole]) -> bool {
    holes.iter().any(|h| {
        let dx = lx - h.lx;
        let dy = ly - h.ly;
        dx * dx + dy * dy <= h.radius * h.radius
    })
}

// AUTO-GENERATED from MapBuilder (do not hand-edit). 354 static map walls.
// Each: center (x,z), half-extents (hw along local X, hd along local Z), rotation (rad about Y), y_min, y_max.
pub const MAP_WALLS: &[MapWall] = &[
    MapWall { cx: 0.000, cz: -360.000, hw: 60.0000, hd: 2.0000, rot: 0.000000, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -60.000, cz: -333.051, hw: 2.0000, hd: 26.9490, rot: 0.000000, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 60.000, cz: -333.051, hw: 2.0000, hd: 26.9490, rot: 0.000000, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 0.000, cz: 360.000, hw: 60.0000, hd: 2.0000, rot: 0.000000, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -60.000, cz: 333.051, hw: 2.0000, hd: 26.9490, rot: 0.000000, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 60.000, cz: 333.051, hw: 2.0000, hd: 26.9490, rot: 0.000000, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -290.043, cz: -90.000, hw: 49.9575, hd: 2.0000, rot: 0.000000, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -290.043, cz: 90.000, hw: 49.9575, hd: 2.0000, rot: 0.000000, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -340.000, cz: 0.000, hw: 2.0000, hd: 90.0000, rot: 0.000000, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 290.043, cz: -90.000, hw: 49.9575, hd: 2.0000, rot: 0.000000, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 290.043, cz: 90.000, hw: 49.9575, hd: 2.0000, rot: 0.000000, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 340.000, cz: 0.000, hw: 2.0000, hd: 90.0000, rot: 0.000000, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 0.000, cz: -50.000, hw: 141.0000, hd: 2.0000, rot: 0.000000, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 0.000, cz: 50.000, hw: 141.0000, hd: 2.0000, rot: 0.000000, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -60.791, cz: -305.153, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -64.579, cz: -300.608, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -68.366, cz: -296.063, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -72.154, cz: -291.518, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -75.942, cz: -286.972, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -79.730, cz: -282.427, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -83.517, cz: -277.882, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -87.305, cz: -273.336, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -91.093, cz: -268.791, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -94.881, cz: -264.246, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -98.668, cz: -259.700, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -102.456, cz: -255.155, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -106.244, cz: -250.610, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -110.032, cz: -246.064, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -113.819, cz: -241.519, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -117.607, cz: -236.974, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -121.395, cz: -232.429, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -125.183, cz: -227.883, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -128.970, cz: -223.338, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -132.758, cz: -218.793, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -136.546, cz: -214.247, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -140.334, cz: -209.702, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -144.122, cz: -205.157, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -147.909, cz: -200.611, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -151.697, cz: -196.066, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -155.485, cz: -191.521, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -159.273, cz: -186.975, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -163.060, cz: -182.430, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -166.848, cz: -177.885, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -170.636, cz: -173.340, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -174.424, cz: -168.794, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -178.211, cz: -164.249, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -181.999, cz: -159.704, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -185.787, cz: -155.158, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -189.575, cz: -150.613, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -193.362, cz: -146.068, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -197.150, cz: -141.522, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -200.938, cz: -136.977, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -204.726, cz: -132.432, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -208.513, cz: -127.886, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -212.301, cz: -123.341, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -216.089, cz: -118.796, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -219.877, cz: -114.250, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -223.664, cz: -109.705, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -227.452, cz: -105.160, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -231.240, cz: -100.615, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -235.028, cz: -96.069, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -238.815, cz: -91.524, hw: 2.0000, hd: 2.9585, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -2.552, cz: -218.835, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -6.376, cz: -214.246, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -10.200, cz: -209.658, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -14.023, cz: -205.069, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -17.847, cz: -200.481, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -21.671, cz: -195.892, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -25.495, cz: -191.304, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -29.319, cz: -186.715, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -33.143, cz: -182.126, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -36.966, cz: -177.538, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -40.790, cz: -172.949, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -44.614, cz: -168.361, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -48.438, cz: -163.772, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -52.262, cz: -159.184, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -56.085, cz: -154.595, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -59.909, cz: -150.007, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -63.733, cz: -145.418, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -67.557, cz: -140.829, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -71.381, cz: -136.241, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -75.204, cz: -131.652, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -79.028, cz: -127.064, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -82.852, cz: -122.475, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -86.676, cz: -117.887, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -90.500, cz: -113.298, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -94.323, cz: -108.709, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -98.147, cz: -104.121, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -101.971, cz: -99.532, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -105.795, cz: -94.944, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -109.619, cz: -90.355, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -113.442, cz: -85.767, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -117.266, cz: -81.178, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -121.090, cz: -76.589, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -124.914, cz: -72.001, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -128.738, cz: -67.412, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -132.561, cz: -62.824, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -136.385, cz: -58.235, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -140.209, cz: -53.647, hw: 2.0000, hd: 2.9865, rot: -0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 2.552, cz: -218.835, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 6.376, cz: -214.246, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 10.200, cz: -209.658, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 14.023, cz: -205.069, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 17.847, cz: -200.481, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 21.671, cz: -195.892, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 25.495, cz: -191.304, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 29.319, cz: -186.715, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 33.143, cz: -182.126, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 36.966, cz: -177.538, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 40.790, cz: -172.949, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 44.614, cz: -168.361, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 48.438, cz: -163.772, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 52.262, cz: -159.184, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 56.085, cz: -154.595, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 59.909, cz: -150.007, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 63.733, cz: -145.418, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 67.557, cz: -140.829, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 71.381, cz: -136.241, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 75.204, cz: -131.652, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 79.028, cz: -127.064, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 82.852, cz: -122.475, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 86.676, cz: -117.887, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 90.500, cz: -113.298, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 94.323, cz: -108.709, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 98.147, cz: -104.121, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 101.971, cz: -99.532, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 105.795, cz: -94.944, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 109.619, cz: -90.355, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 113.442, cz: -85.767, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 117.266, cz: -81.178, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 121.090, cz: -76.589, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 124.914, cz: -72.001, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 128.738, cz: -67.412, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 132.561, cz: -62.824, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 136.385, cz: -58.235, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 140.209, cz: -53.647, hw: 2.0000, hd: 2.9865, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 60.791, cz: -305.153, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 64.579, cz: -300.608, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 68.366, cz: -296.063, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 72.154, cz: -291.518, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 75.942, cz: -286.972, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 79.730, cz: -282.427, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 83.517, cz: -277.882, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 87.305, cz: -273.336, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 91.093, cz: -268.791, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 94.881, cz: -264.246, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 98.668, cz: -259.700, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 102.456, cz: -255.155, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 106.244, cz: -250.610, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 110.032, cz: -246.064, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 113.819, cz: -241.519, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 117.607, cz: -236.974, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 121.395, cz: -232.429, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 125.183, cz: -227.883, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 128.970, cz: -223.338, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 132.758, cz: -218.793, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 136.546, cz: -214.247, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 140.334, cz: -209.702, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 144.122, cz: -205.157, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 147.909, cz: -200.611, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 151.697, cz: -196.066, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 155.485, cz: -191.521, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 159.273, cz: -186.975, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 163.060, cz: -182.430, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 166.848, cz: -177.885, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 170.636, cz: -173.340, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 174.424, cz: -168.794, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 178.211, cz: -164.249, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 181.999, cz: -159.704, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 185.787, cz: -155.158, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 189.575, cz: -150.613, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 193.362, cz: -146.068, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 197.150, cz: -141.522, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 200.938, cz: -136.977, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 204.726, cz: -132.432, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 208.513, cz: -127.886, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 212.301, cz: -123.341, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 216.089, cz: -118.796, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 219.877, cz: -114.250, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 223.664, cz: -109.705, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 227.452, cz: -105.160, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 231.240, cz: -100.615, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 235.028, cz: -96.069, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 238.815, cz: -91.524, hw: 2.0000, hd: 2.9585, rot: 0.694740, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -2.552, cz: 218.835, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -6.376, cz: 214.246, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -10.200, cz: 209.658, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -14.023, cz: 205.069, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -17.847, cz: 200.481, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -21.671, cz: 195.892, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -25.495, cz: 191.304, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -29.319, cz: 186.715, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -33.143, cz: 182.126, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -36.966, cz: 177.538, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -40.790, cz: 172.949, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -44.614, cz: 168.361, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -48.438, cz: 163.772, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -52.262, cz: 159.184, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -56.085, cz: 154.595, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -59.909, cz: 150.007, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -63.733, cz: 145.418, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -67.557, cz: 140.829, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -71.381, cz: 136.241, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -75.204, cz: 131.652, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -79.028, cz: 127.064, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -82.852, cz: 122.475, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -86.676, cz: 117.887, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -90.500, cz: 113.298, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -94.323, cz: 108.709, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -98.147, cz: 104.121, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -101.971, cz: 99.532, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -105.795, cz: 94.944, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -109.619, cz: 90.355, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -113.442, cz: 85.767, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -117.266, cz: 81.178, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -121.090, cz: 76.589, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -124.914, cz: 72.001, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -128.738, cz: 67.412, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -132.561, cz: 62.824, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -136.385, cz: 58.235, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -140.209, cz: 53.647, hw: 2.0000, hd: 2.9865, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -60.791, cz: 305.153, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -64.579, cz: 300.608, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -68.366, cz: 296.063, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -72.154, cz: 291.518, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -75.942, cz: 286.972, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -79.730, cz: 282.427, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -83.517, cz: 277.882, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -87.305, cz: 273.336, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -91.093, cz: 268.791, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -94.881, cz: 264.246, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -98.668, cz: 259.700, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -102.456, cz: 255.155, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -106.244, cz: 250.610, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -110.032, cz: 246.064, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -113.819, cz: 241.519, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -117.607, cz: 236.974, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -121.395, cz: 232.429, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -125.183, cz: 227.883, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -128.970, cz: 223.338, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -132.758, cz: 218.793, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -136.546, cz: 214.247, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -140.334, cz: 209.702, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -144.122, cz: 205.157, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -147.909, cz: 200.611, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -151.697, cz: 196.066, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -155.485, cz: 191.521, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -159.273, cz: 186.975, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -163.060, cz: 182.430, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -166.848, cz: 177.885, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -170.636, cz: 173.340, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -174.424, cz: 168.794, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -178.211, cz: 164.249, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -181.999, cz: 159.704, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -185.787, cz: 155.158, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -189.575, cz: 150.613, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -193.362, cz: 146.068, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -197.150, cz: 141.522, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -200.938, cz: 136.977, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -204.726, cz: 132.432, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -208.513, cz: 127.886, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -212.301, cz: 123.341, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -216.089, cz: 118.796, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -219.877, cz: 114.250, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -223.664, cz: 109.705, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -227.452, cz: 105.160, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -231.240, cz: 100.615, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -235.028, cz: 96.069, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: -238.815, cz: 91.524, hw: 2.0000, hd: 2.9585, rot: -2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 60.791, cz: 305.153, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 64.579, cz: 300.608, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 68.366, cz: 296.063, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 72.154, cz: 291.518, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 75.942, cz: 286.972, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 79.730, cz: 282.427, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 83.517, cz: 277.882, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 87.305, cz: 273.336, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 91.093, cz: 268.791, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 94.881, cz: 264.246, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 98.668, cz: 259.700, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 102.456, cz: 255.155, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 106.244, cz: 250.610, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 110.032, cz: 246.064, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 113.819, cz: 241.519, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 117.607, cz: 236.974, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 121.395, cz: 232.429, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 125.183, cz: 227.883, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 128.970, cz: 223.338, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 132.758, cz: 218.793, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 136.546, cz: 214.247, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 140.334, cz: 209.702, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 144.122, cz: 205.157, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 147.909, cz: 200.611, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 151.697, cz: 196.066, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 155.485, cz: 191.521, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 159.273, cz: 186.975, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 163.060, cz: 182.430, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 166.848, cz: 177.885, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 170.636, cz: 173.340, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 174.424, cz: 168.794, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 178.211, cz: 164.249, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 181.999, cz: 159.704, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 185.787, cz: 155.158, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 189.575, cz: 150.613, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 193.362, cz: 146.068, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 197.150, cz: 141.522, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 200.938, cz: 136.977, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 204.726, cz: 132.432, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 208.513, cz: 127.886, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 212.301, cz: 123.341, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 216.089, cz: 118.796, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 219.877, cz: 114.250, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 223.664, cz: 109.705, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 227.452, cz: 105.160, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 231.240, cz: 100.615, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 235.028, cz: 96.069, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 238.815, cz: 91.524, hw: 2.0000, hd: 2.9585, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 2.552, cz: 218.835, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 6.376, cz: 214.246, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 10.200, cz: 209.658, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 14.023, cz: 205.069, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 17.847, cz: 200.481, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 21.671, cz: 195.892, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 25.495, cz: 191.304, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 29.319, cz: 186.715, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 33.143, cz: 182.126, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 36.966, cz: 177.538, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 40.790, cz: 172.949, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 44.614, cz: 168.361, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 48.438, cz: 163.772, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 52.262, cz: 159.184, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 56.085, cz: 154.595, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 59.909, cz: 150.007, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 63.733, cz: 145.418, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 67.557, cz: 140.829, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 71.381, cz: 136.241, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 75.204, cz: 131.652, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 79.028, cz: 127.064, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 82.852, cz: 122.475, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 86.676, cz: 117.887, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 90.500, cz: 113.298, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 94.323, cz: 108.709, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 98.147, cz: 104.121, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 101.971, cz: 99.532, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 105.795, cz: 94.944, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 109.619, cz: 90.355, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 113.442, cz: 85.767, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 117.266, cz: 81.178, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 121.090, cz: 76.589, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 124.914, cz: 72.001, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 128.738, cz: 67.412, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 132.561, cz: 62.824, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 136.385, cz: 58.235, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
    MapWall { cx: 140.209, cz: 53.647, hw: 2.0000, hd: 2.9865, rot: 2.446850, y_min: 0.0, y_max: 25.0 },
];
