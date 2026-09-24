#![allow(dead_code)]
use std::f32::consts::PI;
use rand::Rng;
use crate::fastmath::{clampf, fwrap};
use crate::rig::*;
use crate::bubbles::BubblesState;

pub mod view {
    pub const SWIM_TOP: f32 = 60.0;
    pub const SWIM_BOT: f32 = 410.0;

    pub fn ymap(v: f32) -> f32 {
        SWIM_TOP + (v - 52.0) * ((SWIM_BOT - SWIM_TOP) / 188.0)
    }

    pub const X0: f32 = 28.0;
    pub const X1: f32 = 612.0;
    pub const W: usize = 640;
    pub const H: usize = 480;
}

#[derive(Clone, Copy, Debug)]
pub struct DepthEv {
    pub mode: u8,
    pub t: f32,
    pub dur: f32,
    pub from: f32,
    pub to: f32,
    pub sign: f32,
    pub cool: f32,
    pub bell: f32,
}

pub const TRAIL_CAP: usize = 128;

#[derive(Clone, Copy, Debug)]
pub struct Trail {
    pub x: [f32; TRAIL_CAP],
    pub y: [f32; TRAIL_CAP],
    pub n: usize,
    pub head: usize,
}

impl Trail {
    pub fn new() -> Self {
        Self {
            x: [0.0; TRAIL_CAP],
            y: [0.0; TRAIL_CAP],
            n: 0,
            head: TRAIL_CAP - 1,
        }
    }

    pub fn push(&mut self, px: f32, py: f32) {
        self.head = (self.head + 1) % TRAIL_CAP;
        self.x[self.head] = px;
        self.y[self.head] = py;
        if self.n < TRAIL_CAP {
            self.n += 1;
        }
    }

    pub fn idx(&self, back: usize) -> usize {
        (self.head + TRAIL_CAP * 2 - back) % TRAIL_CAP
    }
}

pub fn trail_at(tr: &Trail, back: f32) -> (f32, f32) {
    if tr.n == 0 {
        return (0.0, 0.0);
    }
    let mut n = 0;
    let mut acc = 0.0;
    let i0 = tr.idx(0);
    let mut px = tr.x[i0];
    let mut py = tr.y[i0];
    while n + 1 < tr.n && acc < back {
        let j = tr.idx(n + 1);
        let qx = tr.x[j];
        let qy = tr.y[j];
        let dx = px - qx;
        let dy = py - qy;
        let seg = (dx * dx + dy * dy).sqrt();
        if acc + seg >= back {
            let t = (back - acc) / if seg > 1e-6 { seg } else { 1e-6 };
            return (px + (qx - px) * t, py + (qy - py) * t);
        }
        acc += seg;
        px = qx;
        py = qy;
        n += 1;
    }
    (px, py)
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Bone {
    pub x: f32,
    pub y: f32,
    pub a: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Personality {
    None,
    Gulper,
    Loafer,
    Hoverer,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FishAct {
    Swim,
    Rise,
    Gulp,
    Sink,
    Park,
    Hover,
    Hold,
    Graze,
    Rest,
    Dash,
    Air,
    Settle,
    Pick,
    Crawl,
    SwimOff,
    Flick,
}

#[derive(Clone, Debug)]
pub struct Fish {
    pub cfg: SpeciesCfg,
    pub home: SpeciesCfg,
    pub x: f32,
    pub y: f32,
    pub heading: f32,
    pub speed: f32,
    pub prev_heading: f32,
    pub turn_rate: f32,
    pub tx: f32,
    pub ty: f32,
    pub retarget: f32,
    pub trail: Trail,
    pub beat: f32,
    pub phase: f32,
    pub sf: f32,
    pub depth: DepthEv,
    pub bias_f: f32,
    pub burst_x: f32,
    pub burst_y: f32,
    pub sep_x: f32,
    pub sep_y: f32,
    pub speed_norm: f32,
    pub school: bool,
    pub sid: usize,
    pub orbit_r: f32,
    pub pers: Personality,
    pub act: FishAct,
    pub act_t: f32,
    pub next_act: f32,
    pub hold_x: f32,
    pub hold_y: f32,
    pub effort: f32,
    pub lifted: f32,
    pub thrash: f32,
    pub bones: [Bone; BONES],
    pub facing: f32,
    pub mirror: f32,
    pub turn: f32,
    pub flare: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Surge {
    pub t: f32,
    pub dur: f32,
    pub dir: f32,
    pub mag: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Mote {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub a: f32,
    pub ph: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct School {
    pub x: f32,
    pub y: f32,
    pub tx: f32,
    pub ty: f32,
    pub timer: f32,
    pub dash: f32,
    pub home: f32,
    pub home_y: f32,
}

pub const N_SCHOOLS: usize = 3;
pub const N_FISH_MAX: usize = 20 + 8 + 3 + 2 + 5 + 3; // plenty for neons, guppies, tetras, corys, shrimps, cards
pub const N_NEON_SHRIMPDAY: usize = 5;
pub const N_GUPPY: usize = 8;
pub const CARD_FACE_EASE: f32 = 5.0;
pub const CARD_FACE_FLAT: f32 = 0.5;
pub const N_MOTES: usize = 22;
pub const MAX_SURGE: usize = 6;
pub const TRIG_WRAP: f32 = 200.0 * 2.0 * PI;
pub const GAG_CHANCE: f64 = 0.03;
pub const SHRIMP_CHANCE: f64 = 0.10;
pub const LIFT_REF_CM: f32 = 3.0;
pub const CORY_TOUCH: f32 = 36.0;

pub struct Sim {
    pub fish: Vec<Fish>,
    pub school: [School; N_SCHOOLS],
    pub surges: Vec<Surge>,
    pub motes: [Mote; N_MOTES],
    pub stress: f32,
    pub t: f32,
    pub tw: f32,
    pub ebi_day: bool,
    pub shrimp_day: bool,
    pub sway: f32,
    pub sway_v: f32,
}

impl Sim {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        let ebi_day = rng.gen_bool(GAG_CHANCE);
        let shrimp_day = rng.gen_bool(SHRIMP_CHANCE);

        let mut slot = [0usize, 1, 2];
        for k in (1..N_SCHOOLS).rev() {
            let j = rng.gen_range(0..=k);
            slot.swap(k, j);
        }

        let mut schools = [School {
            x: 0.0, y: 0.0, tx: 0.0, ty: 0.0, timer: 0.0, dash: 0.0, home: 0.0, home_y: 0.0
        }; N_SCHOOLS];

        for k in 0..N_SCHOOLS {
            let home = 124.0 + slot[k] as f32 * 196.0 + rng.gen_range(-48.0..48.0);
            let y = NEON.y_lo + (NEON.y_hi - NEON.y_lo) * rng.gen_range(0.12..0.86);
            schools[k] = School {
                x: home,
                y,
                tx: home,
                ty: y,
                timer: rng.gen_range(2.0..6.0),
                dash: 0.0,
                home,
                home_y: y,
            };
        }

        let mut fish_list = Vec::with_capacity(N_FISH_MAX);
        let n_neon = if shrimp_day { N_NEON_SHRIMPDAY } else { NEON.count };

        for k in 0..n_neon {
            let sid = if shrimp_day { 1 } else { k % N_SCHOOLS };
            fish_list.push(create_fish(
                NEON,
                true,
                sid,
                schools[sid].x,
                schools[sid].y,
            ));
        }

        let mut gs = Vec::new();
        for g in 0..5 {
            for _ in 0..GUPPY_STRAINS[g].count {
                if gs.len() < N_GUPPY {
                    gs.push(g);
                }
            }
        }

        for &g_idx in &gs {
            fish_list.push(create_fish(GUPPY_STRAINS[g_idx], false, 0, 0.0, 0.0));
        }

        for _ in 0..BLACKTETRA.count {
            fish_list.push(create_fish(BLACKTETRA, false, 0, 0.0, 0.0));
        }

        for _ in 0..CORYDORAS.count {
            fish_list.push(create_fish(CORYDORAS, false, 0, 0.0, 0.0));
        }

        if shrimp_day {
            for _ in 0..YAMATO.count {
                fish_list.push(create_fish(YAMATO, false, 0, 0.0, 0.0));
            }
        }

        if ebi_day {
            for f in fish_list.iter_mut() {
                if f.home.key == SpKey::Guppy {
                    f.cfg = EBIFRY;
                }
            }
        }

        let mut motes = [Mote { x: 0.0, y: 0.0, vx: 0.0, vy: 0.0, a: 0.0, ph: 0.0 }; N_MOTES];
        let floor_y = view::ymap(268.0);
        let horizon_y = view::ymap(36.0);
        for m in motes.iter_mut() {
            *m = Mote {
                x: rng.gen_range(20.0..620.0),
                y: rng.gen_range(horizon_y..floor_y),
                vx: rng.gen_range(-2.4..2.4),
                vy: rng.gen_range(-0.8..1.8),
                a: rng.gen_range(0.05..0.22),
                ph: rng.gen_range(0.0..6.0),
            };
        }

        Self {
            fish: fish_list,
            school: schools,
            surges: Vec::new(),
            motes,
            stress: 0.0,
            t: 0.0,
            tw: 0.0,
            ebi_day,
            shrimp_day,
            sway: 0.0,
            sway_v: 0.0,
        }
    }

    pub fn tap_water(&mut self, x: f32, y: f32) {
        let mut rng = rand::thread_rng();
        let dir = if x < 320.0 { 1.0 } else { -1.0 };
        if self.surges.len() < MAX_SURGE {
            self.surges.push(Surge { t: 0.0, dur: 0.9, dir, mag: 1.0 });
        }
        self.stress = (self.stress + 0.7).min(1.0);

        for k in 0..N_SCHOOLS {
            let s = &mut self.school[k];
            let sign = if s.home < x { -1.0 } else { 1.0 };
            s.tx = clampf(s.home + sign * rng.gen_range(70.0..190.0), 50.0, 590.0);
            s.ty = clampf(s.home_y + rng.gen_range(-44.0..44.0), NEON.y_lo + 16.0, NEON.y_hi - 16.0);
            s.timer = 4.0;
            s.dash = 1.0;
        }

        let mut order: Vec<(usize, f32)> = self.fish.iter().enumerate().map(|(i, f)| {
            let dx = f.x - x;
            let dy = f.y - y;
            (i, (dx * dx + dy * dy).sqrt())
        }).collect();

        order.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        for &(idx, dist) in &order {
            if dist > 220.0 {
                break;
            }
            let f = &mut self.fish[idx];
            let kk = 1.0 - dist / 220.0;
            let ang = (f.y - y).atan2(f.x - x);
            f.burst_x += ang.cos() * 170.0 * kk;
            f.burst_y += ang.sin() * 120.0 * kk;
        }

        for &(idx, dist) in order.iter().take(2) {
            if dist < 260.0 {
                start_depth_event(&mut self.fish[idx], true);
            }
        }
    }

    pub fn step(&mut self, dt: f32, bubbles: &BubblesState) {
        self.t += dt;
        self.tw += dt;
        if self.tw > TRIG_WRAP {
            self.tw -= TRIG_WRAP;
        }
        self.stress *= (-1.1 * dt).exp();

        for s in self.school.iter_mut() {
            step_school(s, dt);
        }

        step_separation(&mut self.fish);
        step_shrimp_scatter(&mut self.fish);

        for i in 0..self.fish.len() {
            let (_fish_front, fish_rest) = self.fish.split_at_mut(i);
            let f = &mut fish_rest[0];
            step_fish(f, self.tw, self.stress, &self.school, dt, bubbles);
        }

        // Water sway
        let mut i = 0;
        while i < self.surges.len() {
            let s = &mut self.surges[i];
            s.t += dt;
            if s.t < s.dur {
                let p = s.t / s.dur;
                self.sway_v += s.dir * s.mag * (p * PI).sin() * dt * 16.0;
                i += 1;
            } else {
                self.surges.swap_remove(i);
            }
        }
        self.sway_v += -self.sway * 26.0 * dt - self.sway_v * 5.5 * dt;
        self.sway += self.sway_v * dt;
        self.sway = clampf(self.sway, -7.0, 7.0);

        // Motes
        let floor_y = view::ymap(268.0);
        let horizon_y = view::ymap(36.0);
        for m in self.motes.iter_mut() {
            m.x += (m.vx + (self.tw * 0.4 + m.ph).sin() * 1.0) * dt;
            m.y += m.vy * dt;
            if m.y > floor_y - 6.0 {
                m.y = horizon_y + 6.0;
            }
            if m.y < horizon_y + 4.0 {
                m.y = floor_y - 8.0;
            }
            if m.x < 12.0 {
                m.x = 628.0;
            }
            if m.x > 628.0 {
                m.x = 12.0;
            }
        }
    }
}

fn create_fish(cfg: SpeciesCfg, school: bool, sid: usize, sx: f32, sy: f32) -> Fish {
    let mut rng = rand::thread_rng();
    let x = if school {
        sx + rng.gen_range(-60.0..60.0)
    } else {
        rng.gen_range(44.0..596.0)
    };
    let y = if school {
        sy + rng.gen_range(-32.0..32.0)
    } else {
        rng.gen_range(cfg.y_lo..cfg.y_hi)
    };
    let x = clampf(x, view::X0 + 8.0, view::X1 - 8.0);
    let y = clampf(y, cfg.y_lo, cfg.y_hi);
    let heading = rng.gen_range(0.0..PI * 2.0);

    let mut trail = Trail::new();
    for k in (0..TRAIL_CAP).rev() {
        trail.push(
            x - heading.cos() * k as f32 * 1.0,
            y - heading.sin() * k as f32 * 1.0,
        );
    }

    let mut bones = [Bone::default(); BONES];
    for k in 0..BONES {
        bones[k] = Bone {
            x: x - heading.cos() * k as f32 * cfg.spacing,
            y: y - heading.sin() * k as f32 * cfg.spacing,
            a: heading,
        };
    }

    let r: f32 = rng.gen();
    let pers = match cfg.key {
        SpKey::BlackTetra => Personality::Hoverer,
        SpKey::Corydoras | SpKey::Yamato => Personality::None,
        SpKey::Guppy => {
            if r < 0.32 { Personality::Loafer }
            else if r < 0.62 { Personality::Gulper }
            else { Personality::None }
        }
        _ => {
            if r < 0.08 { Personality::Loafer }
            else if r < 0.20 { Personality::Gulper }
            else { Personality::None }
        }
    };

    let act = match cfg.key {
        SpKey::Corydoras => FishAct::Graze,
        SpKey::Yamato => FishAct::Pick,
        _ => FishAct::Swim,
    };

    let next_act = match cfg.key {
        SpKey::Corydoras => rng.gen_range(10.0..90.0),
        SpKey::Yamato => rng.gen_range(8.0..45.0),
        _ => rng.gen_range(5.0..30.0),
    };

    let mirror = if heading.cos() < 0.0 { -1.0 } else { 1.0 };
    let turn = if mirror < 0.0 { PI } else { 0.0 };

    Fish {
        cfg,
        home: cfg,
        x,
        y,
        heading,
        speed: cfg.base_speed,
        prev_heading: heading,
        turn_rate: 0.0,
        tx: rng.gen_range(60.0..580.0),
        ty: rng.gen_range(view::ymap(60.0)..view::ymap(220.0)),
        retarget: rng.gen_range(2.0..6.0),
        trail,
        beat: rng.gen_range(0.0..10.0),
        phase: rng.gen_range(0.0..PI * 2.0),
        sf: rng.gen_range(0.88..1.12),
        depth: DepthEv {
            mode: 0,
            t: 0.0,
            dur: 1.0,
            from: 1.0,
            to: 1.0,
            sign: 1.0,
            cool: rng.gen_range(1.0..5.0),
            bell: 0.0,
        },
        bias_f: 0.0,
        burst_x: 0.0,
        burst_y: 0.0,
        sep_x: 0.0,
        sep_y: 0.0,
        speed_norm: 0.5,
        school,
        sid,
        orbit_r: rng.gen_range(20.0..68.0),
        pers,
        act,
        act_t: rng.gen_range(1.0..4.0),
        next_act,
        hold_x: x,
        hold_y: y,
        effort: 1.0,
        lifted: 0.0,
        thrash: 0.0,
        bones,
        facing: 1.0,
        mirror,
        turn,
        flare: 0.0,
    }
}

pub fn start_depth_event(f: &mut Fish, stress: bool) {
    if f.depth.mode == 1 {
        return;
    }
    if !stress {
        f.depth.cool = 5.0;
        return;
    }
    let mut rng = rand::thread_rng();
    let max_d = 0.3;
    let mag = (0.4 + rng.gen::<f32>() * 0.6) * max_d;
    let mut sign = if rng.gen_bool(0.5) { -1.0 } else { 1.0 };
    if f.turn_rate.abs() > 1.2 && rng.gen_bool(0.65) {
        sign = -1.0;
    }
    let to = clampf(f.sf + sign * mag, 0.74, 1.28);
    f.depth.mode = 1;
    f.depth.t = 0.0;
    f.depth.dur = 0.45 + (to - f.sf).abs() * 2.2 + rng.gen::<f32>() * 0.3;
    f.depth.from = f.sf;
    f.depth.to = to;
    let s = to - f.sf;
    f.depth.sign = if s > 0.0 { 1.0 } else { -1.0 };
}

fn step_depth(f: &mut Fish, dt: f32) {
    let mut rng = rand::thread_rng();
    let d = &mut f.depth;
    if d.mode == 0 {
        d.cool -= dt;
        d.bell = (d.bell - dt * 3.0).max(0.0);
        if d.cool <= 0.0 {
            d.cool = rng.gen_range(20.0..40.0);
        }
        return;
    }
    d.t += dt;
    let p = clampf(d.t / d.dur, 0.0, 1.0);
    let e = p * p * (3.0 - 2.0 * p);
    f.sf = d.from + (d.to - d.from) * e;
    d.bell = (PI * p).sin();
    if p >= 1.0 {
        d.mode = 0;
        d.bell = 0.0;
    }
}

fn step_chain(f: &mut Fish) {
    let last = f.trail.idx(0);
    let dx = f.x - f.trail.x[last];
    let dy = f.y - f.trail.y[last];
    if (dx * dx + dy * dy).sqrt() >= 0.9 {
        f.trail.push(f.x, f.y);
    }

    let hx = f.heading.cos();
    let hy = f.heading.sin();
    let k_relax = if f.cfg.key == SpKey::Neon { 0.5 } else { 0.45 };
    let k_ang = if f.cfg.key == SpKey::Neon { 0.42 } else { 0.55 };

    for b in 0..BONES {
        let (ptx, pty) = trail_at(&f.trail, b as f32 * f.cfg.spacing);
        if b == 0 {
            f.bones[0].a = f.heading;
        } else {
            let target = (f.bones[b - 1].y - pty).atan2(f.bones[b - 1].x - ptx);
            f.bones[b].a = fwrap(f.bones[b].a + fwrap(target - f.bones[b].a) * k_ang);
        }
        let sx = f.x - hx * b as f32 * f.cfg.spacing;
        let sy = f.y - hy * b as f32 * f.cfg.spacing;
        f.bones[b].x = ptx + (sx - ptx) * k_relax;
        f.bones[b].y = pty + (sy - pty) * k_relax;
    }
}

fn step_school(s: &mut School, dt: f32) {
    let mut rng = rand::thread_rng();
    s.timer -= dt;
    let dx = s.tx - s.x;
    let dy = s.ty - s.y;
    let dist = (dx * dx + dy * dy).sqrt();
    if s.timer <= 0.0 || dist < 24.0 {
        s.timer = rng.gen_range(5.0..11.0);
        let far = rng.gen_bool(0.25);
        s.dash = if far { 1.0 } else { 0.0 };
        s.tx = clampf(s.home + rng.gen_range(-190.0..190.0) + if far { rng.gen_range(-120.0..120.0) } else { 0.0 }, 50.0, 590.0);
        s.ty = clampf(s.home_y + rng.gen_range(-40.0..40.0), NEON.y_lo + 16.0, NEON.y_hi - 16.0);
    }
    let v = if s.dash > 0.0 { 120.0 } else { 44.0 };
    if dist > 0.5 {
        s.x += (dx / dist) * (v * dt).min(dist);
        s.y += (dy / dist) * (v * dt).min(dist);
    }
    s.dash = (s.dash - dt * 0.5).max(0.0);
}

fn step_separation(fish: &mut [Fish]) {
    const STRENGTH: f32 = 52.0;
    for f in fish.iter_mut() {
        f.sep_x = 0.0;
        f.sep_y = 0.0;
    }
    let n = fish.len();
    for i in 0..n {
        for j in (i + 1)..n {
            let dx = fish[j].x - fish[i].x;
            let dy = fish[j].y - fish[i].y;
            let r = (fish[i].cfg.w + fish[j].cfg.w) as f32 * 0.45;
            let d2 = dx * dx + dy * dy;
            if d2 >= r * r || d2 < 1e-4 {
                continue;
            }
            let d = d2.sqrt();
            let push = (1.0 - d / r) * STRENGTH;
            let ux = dx / d;
            let uy = dy / d;
            fish[i].sep_x -= ux * push;
            fish[i].sep_y -= uy * push * 0.7;
            fish[j].sep_x += ux * push;
            fish[j].sep_y += uy * push * 0.7;
        }
    }
}

fn step_shrimp_scatter(fish: &mut [Fish]) {
    let mut rng = rand::thread_rng();
    let n = fish.len();
    for i in 0..n {
        if fish[i].cfg.key != SpKey::Yamato || fish[i].act == FishAct::Flick {
            continue;
        }
        let sh_x = fish[i].x;
        let sh_y = fish[i].y;

        for j in 0..n {
            if fish[j].cfg.key != SpKey::Corydoras {
                continue;
            }
            let dx = sh_x - fish[j].x;
            let dy = sh_y - fish[j].y;
            let d2 = dx * dx + dy * dy;
            if d2 > CORY_TOUCH * CORY_TOUCH {
                continue;
            }

            let d = d2.sqrt();
            let (ux, mut uy) = if d > 0.01 {
                (dx / d, dy / d)
            } else {
                (if rng.gen_bool(0.5) { -1.0 } else { 1.0 }, 0.0)
            };
            uy -= 0.85;
            let norm = (ux * ux + uy * uy).sqrt();

            fish[i].act = FishAct::Flick;
            fish[i].act_t = rng.gen_range(0.28..0.48);
            fish[i].burst_x = ux / norm * 400.0;
            fish[i].burst_y = uy / norm * 400.0;
            fish[i].thrash = 1.0;
            break;
        }
    }
}

fn step_act(f: &mut Fish, dt: f32, tw: f32, stress: f32) -> f32 {
    let mut rng = rand::thread_rng();
    let surface_y = view::SWIM_TOP + 6.0;
    let mut want: f32 = 1.0;
    let mut want_thrash: f32 = 0.0;

    if f.cfg.key == SpKey::Yamato {
        let sand_y = f.cfg.y_hi - 2.0;
        match f.act {
            FishAct::Pick => {
                f.tx = f.x + if f.heading.cos() >= 0.0 { 16.0 } else { -16.0 };
                f.ty = f.hold_y;
                want = 0.02;
                f.act_t -= dt;
                if f.act_t <= 0.0 {
                    f.act = FishAct::Crawl;
                    f.act_t = rng.gen_range(0.7..2.2);
                    f.hold_x = clampf(f.x + rng.gen_range(-50.0..50.0), 48.0, 592.0);
                    f.hold_y = sand_y - rng.gen_range(0.0..8.0);
                }
            }
            FishAct::Crawl => {
                f.tx = f.hold_x;
                f.ty = f.hold_y;
                want = 0.22;
                f.act_t -= dt;
                f.next_act -= dt;
                if (f.x - f.hold_x).abs() < 12.0 || f.act_t <= 0.0 {
                    if f.next_act <= 0.0 && rng.gen_bool(0.35) {
                        f.act = FishAct::SwimOff;
                        f.act_t = rng.gen_range(2.5..6.0);
                        f.next_act = rng.gen_range(12.0..50.0);
                        f.hold_x = clampf(f.x + rng.gen_range(-140.0..140.0), 48.0, 592.0);
                        f.hold_y = rng.gen_range(f.cfg.y_lo + 8.0..sand_y);
                    } else {
                        f.act = FishAct::Pick;
                        f.act_t = rng.gen_range(1.5..7.0);
                    }
                }
            }
            FishAct::SwimOff => {
                f.tx = f.hold_x;
                f.ty = f.hold_y;
                want = 0.85;
                f.act_t -= dt;
                if ((f.x - f.hold_x).abs() < 16.0 && (f.y - f.hold_y).abs() < 12.0) || f.act_t <= 0.0 {
                    f.act = FishAct::Pick;
                    f.act_t = rng.gen_range(1.5..5.0);
                    f.hold_y = clampf(f.y, f.cfg.y_lo, sand_y);
                }
            }
            FishAct::Flick => {
                want = 0.0;
                want_thrash = 1.0;
                f.act_t -= dt;
                if f.act_t <= 0.0 {
                    f.act = FishAct::Pick;
                    f.act_t = rng.gen_range(1.0..3.0);
                    f.hold_y = clampf(f.y, f.cfg.y_lo, sand_y);
                }
            }
            _ => {
                f.act = FishAct::Pick;
                f.act_t = rng.gen_range(2.0..6.0);
                f.hold_y = clampf(f.y, f.cfg.y_lo, sand_y);
            }
        }

        if f.retarget > 0.0 {
            f.retarget -= dt;
        }
        if stress > 0.55 && f.act != FishAct::Flick && f.retarget <= 0.0 && rng.gen::<f32>() < dt * 0.55 {
            f.act = FishAct::Flick;
            f.act_t = rng.gen_range(0.30..0.55);
            f.retarget = rng.gen_range(40.0..120.0);
            f.burst_x = -f.heading.cos() * 350.0;
            f.burst_y = -f.heading.sin() * 350.0 - 76.0;
        }
        if f.act == FishAct::Pick && f.y < f.cfg.y_lo - 20.0 {
            f.act = FishAct::SwimOff;
            f.act_t = rng.gen_range(2.5..5.0);
            f.hold_x = clampf(f.x + rng.gen_range(-60.0..60.0), 48.0, 592.0);
            f.hold_y = rng.gen_range(f.cfg.y_lo + 8.0..sand_y);
        }

        f.thrash += (want_thrash - f.thrash) * (dt * if want_thrash > f.thrash { 14.0 } else { 4.0 }).min(1.0);
        let ks = if want > f.effort { (dt * 5.0).min(1.0) } else { (dt * 2.2).min(1.0) };
        f.effort += (want - f.effort) * ks;
        return f.effort;
    }

    if f.cfg.key == SpKey::Corydoras {
        let floor_y = f.cfg.y_hi - 2.0;
        let sky_y = view::SWIM_TOP + 6.0;
        match f.act {
            FishAct::Graze => {
                f.tx = f.hold_x;
                f.ty = f.hold_y;
                want = 0.22;
                f.act_t -= dt;
                f.next_act -= dt;
                if ((f.x - f.hold_x).abs() < 18.0 && (f.y - f.hold_y).abs() < 12.0) || f.act_t <= 0.0 {
                    f.act = FishAct::Rest;
                    f.act_t = rng.gen_range(2.5..8.0);
                    f.hold_y = floor_y - rng.gen_range(0.0..(f.cfg.y_hi - f.cfg.y_lo) / 3.0);
                }
            }
            FishAct::Rest => {
                f.tx = f.x + if f.heading.cos() >= 0.0 { 40.0 } else { -40.0 };
                f.ty = f.hold_y;
                want = 0.03;
                f.act_t -= dt;
                f.next_act -= dt;
                if f.act_t <= 0.0 {
                    if f.next_act <= 0.0 {
                        f.act = FishAct::Dash;
                        f.act_t = 5.0;
                        f.hold_x = clampf(f.x + rng.gen_range(-90.0..90.0), 80.0, 560.0);
                    } else {
                        f.act = FishAct::Graze;
                        f.act_t = rng.gen_range(2.0..5.5);
                        f.hold_x = clampf(f.x + rng.gen_range(-160.0..160.0), 40.0, 600.0);
                        f.hold_y = floor_y - rng.gen_range(0.0..(f.cfg.y_hi - f.cfg.y_lo) / 3.0);
                    }
                }
            }
            FishAct::Dash => {
                f.tx = f.hold_x;
                f.ty = sky_y;
                want = 5.5;
                want_thrash = 1.0;
                f.act_t -= dt;
                if f.y < sky_y + 12.0 || f.act_t <= 0.0 {
                    f.act = FishAct::Air;
                    f.act_t = rng.gen_range(0.2..0.45);
                }
            }
            FishAct::Air => {
                f.tx = f.x + if f.heading.cos() >= 0.0 { 12.0 } else { -12.0 };
                f.ty = sky_y - 4.0;
                want = 0.05;
                f.act_t -= dt;
                if f.act_t <= 0.0 {
                    f.act = FishAct::Settle;
                    f.act_t = 14.0;
                    f.hold_x = clampf(f.x + rng.gen_range(-190.0..190.0), 40.0, 600.0);
                    f.hold_y = floor_y - rng.gen_range(0.0..(f.cfg.y_hi - f.cfg.y_lo) / 3.0);
                }
            }
            FishAct::Settle => {
                f.tx = f.hold_x;
                f.ty = f.hold_y;
                want = 0.9;
                f.act_t -= dt;
                if f.y > f.hold_y - 18.0 || f.act_t <= 0.0 {
                    f.act = FishAct::Graze;
                    f.act_t = rng.gen_range(2.0..5.5);
                    f.next_act = rng.gen_range(35.0..130.0);
                }
            }
            _ => {
                f.act = FishAct::Graze;
                f.act_t = rng.gen_range(1.5..4.0);
                f.hold_x = clampf(f.x + rng.gen_range(-140.0..140.0), 40.0, 600.0);
                f.hold_y = floor_y - rng.gen_range(0.0..(f.cfg.y_hi - f.cfg.y_lo) / 3.0);
            }
        }

        if stress > 0.45 && f.act == FishAct::Rest {
            f.act = FishAct::Graze;
            f.act_t = rng.gen_range(1.0..2.5);
            f.hold_x = clampf(f.x + rng.gen_range(-160.0..160.0), 40.0, 600.0);
        }
        if (f.act == FishAct::Graze || f.act == FishAct::Rest) && f.y < f.cfg.y_lo - 24.0 {
            f.act = FishAct::Settle;
            f.act_t = 14.0;
            f.hold_x = clampf(f.x + rng.gen_range(-80.0..80.0), 40.0, 600.0);
            f.hold_y = floor_y - rng.gen_range(0.0..8.0);
        }

        f.thrash += (want_thrash - f.thrash) * (dt * if want_thrash > f.thrash { 9.0 } else { 3.0 }).min(1.0);
        let ke = if want > f.effort { (dt * 6.0).min(1.0) } else { (dt * 1.8).min(1.0) };
        f.effort += (want - f.effort) * ke;
        return f.effort;
    }

    match f.act {
        FishAct::Hold => {
            f.tx = f.hold_x + if f.heading.cos() >= 0.0 { 44.0 } else { -44.0 };
            f.ty = f.hold_y + (tw * 0.7 + f.phase).sin() * 4.0;
            want = 0.05;
            f.act_t -= dt;
            if f.act_t <= 0.0 {
                f.act = FishAct::Swim;
                f.next_act = rng.gen_range(1.5..5.0);
            }
        }
        FishAct::Swim => {
            f.next_act -= dt;
            if f.pers != Personality::None && f.next_act <= 0.0 && stress < 0.2 {
                if f.pers == Personality::Hoverer {
                    f.act = FishAct::Hold;
                    f.act_t = rng.gen_range(4.0..15.0);
                    f.hold_x = f.x;
                    f.hold_y = f.y;
                } else if f.pers == Personality::Gulper {
                    f.act = FishAct::Rise;
                    f.act_t = 9.0;
                    f.hold_x = clampf(f.x + rng.gen_range(-80.0..80.0), 70.0, 570.0);
                    f.hold_y = surface_y;
                } else {
                    f.act = FishAct::Park;
                    f.act_t = 10.0;
                    let lo = f.cfg.y_lo;
                    let hi = f.cfg.y_hi;
                    if rng.gen_bool(0.55) {
                        f.hold_x = if rng.gen_bool(0.5) { view::X0 + 22.0 } else { view::X1 - 22.0 };
                        f.hold_y = rng.gen_range(lo + (hi - lo) * 0.3..hi);
                    } else {
                        f.hold_x = rng.gen_range(90.0..550.0);
                        f.hold_y = hi - rng.gen_range(0.0..16.0);
                    }
                }
            }
        }
        FishAct::Rise => {
            f.tx = f.hold_x;
            f.ty = f.hold_y;
            want = 0.75;
            f.act_t -= dt;
            if f.y < surface_y + 12.0 || f.act_t <= 0.0 {
                f.act = FishAct::Gulp;
                f.act_t = rng.gen_range(0.7..1.7);
            }
        }
        FishAct::Gulp => {
            f.tx = f.x + if f.heading.cos() >= 0.0 { 14.0 } else { -14.0 };
            f.ty = surface_y - 4.0;
            want = 0.06;
            f.act_t -= dt;
            if f.act_t <= 0.0 {
                f.act = FishAct::Sink;
                f.act_t = 7.0;
                f.hold_y = rng.gen_range(view::ymap(90.0)..view::ymap(184.0));
            }
        }
        FishAct::Sink => {
            f.tx = clampf(f.x + if f.heading.cos() >= 0.0 { 60.0 } else { -60.0 }, 70.0, 570.0);
            f.ty = f.hold_y;
            want = 0.5;
            f.act_t -= dt;
            if (f.y - f.hold_y).abs() < 16.0 || f.act_t <= 0.0 {
                f.act = FishAct::Swim;
                f.next_act = rng.gen_range(12.0..32.0);
            }
        }
        FishAct::Park => {
            f.tx = f.hold_x;
            f.ty = f.hold_y;
            want = 0.7;
            f.act_t -= dt;
            if ((f.x - f.hold_x).abs() < 20.0 && (f.y - f.hold_y).abs() < 16.0) || f.act_t <= 0.0 {
                f.act = FishAct::Hover;
                f.act_t = rng.gen_range(4.0..13.0);
            }
        }
        FishAct::Hover => {
            f.tx = f.hold_x + if f.heading.cos() >= 0.0 { 44.0 } else { -44.0 };
            f.ty = f.hold_y + (tw * 0.9 + f.phase).sin() * 3.0;
            want = 0.045;
            f.act_t -= dt;
            if f.act_t <= 0.0 {
                f.act = FishAct::Swim;
                f.next_act = rng.gen_range(15.0..38.0);
            }
        }
        _ => {}
    }

    f.thrash += (0.0 - f.thrash) * (dt * 3.0).min(1.0);
    let k = if want > f.effort { (dt * 6.0).min(1.0) } else { (dt * 1.6).min(1.0) };
    f.effort += (want - f.effort) * k;
    f.effort
}

fn step_fish(f: &mut Fish, tw: f32, stress: f32, schools: &[School; N_SCHOOLS], dt: f32, bubbles: &BubblesState) {
    let mut rng = rand::thread_rng();
    let eff = step_act(f, dt, tw, stress);
    let busy = f.act != FishAct::Swim;

    if busy {
        // Target already set by step_act
    } else if f.school {
        let s = &schools[f.sid];
        let oa = tw * 0.35 + f.phase;
        f.tx = s.x + oa.cos() * f.orbit_r;
        f.ty = s.y + (oa * 0.8 + f.phase).sin() * f.orbit_r * 0.60;
    } else {
        f.retarget -= dt;
        let dx = f.tx - f.x;
        let dy = f.ty - f.y;
        if f.retarget <= 0.0 || dx * dx + dy * dy < 400.0 {
            f.retarget = rng.gen_range(6.0..16.0);
            if f.cfg.roam > 0.0 {
                f.tx = clampf(f.x + rng.gen_range(-f.cfg.roam..f.cfg.roam), 44.0, 596.0);
                f.ty = clampf(f.y + rng.gen_range(-f.cfg.roam * 0.45..f.cfg.roam * 0.45), f.cfg.y_lo, f.cfg.y_hi);
            } else {
                let (lo, hi) = if (f.x < 320.0) != rng.gen_bool(0.25) {
                    ((f.x + 140.0).min(500.0), 596.0)
                } else {
                    (44.0, (f.x - 140.0).max(140.0))
                };
                f.tx = rng.gen_range(lo..hi);
                let mid = f.cfg.y_lo + (f.cfg.y_hi - f.cfg.y_lo) * 0.55;
                f.ty = if rng.gen_bool(0.80) {
                    rng.gen_range(f.cfg.y_lo..mid)
                } else {
                    rng.gen_range(mid..f.cfg.y_hi)
                };
            }
        }
    }

    if !busy {
        if f.x < view::X0 + 16.0 {
            f.tx = f.tx.max(240.0);
        }
        if f.x > view::X1 - 16.0 {
            f.tx = f.tx.min(400.0);
        }
        if f.y < f.cfg.y_lo + 8.0 {
            f.ty = f.ty.max(f.cfg.y_lo + (f.cfg.y_hi - f.cfg.y_lo) * 0.35);
        }
        if f.y > f.cfg.y_hi - 8.0 {
            f.ty = f.ty.min(f.cfg.y_hi - (f.cfg.y_hi - f.cfg.y_lo) * 0.35);
        }
    }

    let desired = (f.ty - f.y).atan2(f.tx - f.x);
    let diff = fwrap(desired - f.heading);
    let max_turn = f.cfg.turn_rate * dt * (0.75 + 0.5 * f.speed_norm);
    f.heading = fwrap(f.heading + clampf(diff, -max_turn, max_turn));

    let inst = fwrap(f.heading - f.prev_heading) / dt.max(1e-4);
    f.prev_heading = f.heading;
    f.turn_rate += (inst - f.turn_rate) * (dt * 7.0).min(1.0);

    let spd = f.cfg.base_speed * (0.8 + 0.35 * (tw * 0.23 + f.phase).sin());
    f.speed += (spd - f.speed) * (dt * 2.0).min(1.0);
    let boost = (1.0 + stress * 1.3 + if f.school && !busy { schools[f.sid].dash * 0.9 } else { 0.0 }) * eff;
    f.speed_norm = clampf((f.speed * boost) / (f.cfg.base_speed * 2.4), 0.0, 1.0);

    // Air stone lift
    let (mut wx, mut wy) = bubbles.air_flow_at(f.x * 0.5, f.y * 0.5);
    wx *= 2.0;
    wy *= 2.0;
    if wy != 0.0 {
        let r = LIFT_REF_CM / f.cfg.len_cm;
        let k = clampf(r * r, 0.25, 1.4);
        wx *= k;
        wy *= k;
        f.thrash = f.thrash.max(k * (-wy * (1.0 / 90.0)).min(1.0) * 0.45);
        f.lifted = 1.2;
    }

    f.x += (f.heading.cos() * f.speed * boost + f.burst_x + f.sep_x + wx) * dt;
    f.y += (f.heading.sin() * f.speed * boost + f.burst_y + f.sep_y + wy) * dt;
    f.x = clampf(f.x, view::X0 - 12.0, view::X1 + 12.0);

    if f.lifted > 0.0 {
        f.lifted -= dt;
        if f.lifted <= 0.0 && f.y < f.cfg.y_lo - 16.0 {
            f.lifted = 0.4;
        }
    }

    let loose = if f.cfg.key == SpKey::Yamato {
        f.lifted > 0.0 || f.act == FishAct::Flick
    } else {
        busy || f.lifted > 0.0
    };

    if loose {
        f.y = clampf(f.y, view::SWIM_TOP - 12.0, view::SWIM_BOT + 8.0);
    } else {
        f.y = clampf(f.y, f.cfg.y_lo - 16.0, f.cfg.y_hi + 16.0);
    }

    f.burst_x *= (-3.0 * dt).exp();
    f.burst_y *= (-3.0 * dt).exp();

    f.beat += dt * PI * 2.0 * f.cfg.beat_hz * (0.65 + 0.9 * f.speed_norm + stress * 0.5 + f.thrash * 2.2);
    if f.beat > TRIG_WRAP {
        f.beat -= TRIG_WRAP;
    }

    step_depth(f, dt);

    let chd = f.heading.cos();
    if chd > 0.15 {
        f.mirror = 1.0;
    } else if chd < -0.15 {
        f.mirror = -1.0;
    }

    if f.cfg.key == SpKey::Card {
        let w = (if chd >= 0.0 { 1.0 } else { -1.0 }) * chd.abs().powf(CARD_FACE_FLAT);
        let target = clampf(w, -1.0, 1.0).acos();
        f.turn += (target - f.turn) * (dt * CARD_FACE_EASE).min(1.0);
    }

    let want_flare = clampf(f.depth.bell * 0.9 + f.turn_rate.abs() * 0.45, 0.0, 1.5);
    let k = if want_flare > f.flare { (dt * 10.0).min(1.0) } else { (dt * 2.2).min(1.0) };
    f.flare += (want_flare - f.flare) * k;

    let roll = clampf(f.turn_rate * 0.35 + f.depth.bell * f.depth.sign * 1.1, -2.0, 2.0);
    f.bias_f += (roll - f.bias_f) * (dt * 9.0).min(1.0);
    f.facing = 1.0 - f.depth.bell * 0.85;

    step_chain(f);
}
