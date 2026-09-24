#![allow(dead_code)]
use std::f32::consts::PI;
use rand::Rng;
use crate::fastmath::smoothstep;

pub const SURF_Y: f32 = 16.0;
pub const STONE_Y0: f32 = 206.0;
pub const STONE_Y1: f32 = 219.0;
pub const L_X0: f32 = 20.0;
pub const L_X1: f32 = 74.0;
pub const R_X0: f32 = 246.0;
pub const R_X1: f32 = 300.0;

pub const PUFF_HZ: f32 = 26.0;
pub const THROB_HZ: f32 = 2.3;
pub const THROB_AMT: f32 = 0.42;

pub const R_MIN: f32 = 0.34;
pub const R_MAX: f32 = 0.78;

pub const MAX_BUB: usize = 232;
pub const MAX_POP: usize = 32;

pub const FAN_PX: f32 = 26.0;
pub const LIFT_MAX: f32 = 62.0;
pub const LIFT_SPREAD: f32 = 34.0;
pub const LIFT_CORE: f32 = 5.0;

#[derive(Clone, Copy)]
pub struct Stone {
    pub x: f32,
    pub y: f32,
    pub emit: f32,
    pub throb: f32,
    pub lean: f32,
    pub lean_t: f32,
}

#[derive(Clone, Copy, Default)]
pub struct Bubble {
    pub x0: f32,
    pub y: f32,
    pub r0: f32,
    pub r: f32,
    pub vy: f32,
    pub fan: f32,
    pub wob_a: f32,
    pub wob_k: f32,
    pub wob_p: f32,
    pub live: bool,
}

#[derive(Clone, Copy, Default)]
pub struct Pop {
    pub x: f32,
    pub t: f32,
    pub dur: f32,
    pub r1: f32,
    pub live: bool,
}

pub struct BubblesState {
    pub stone: Stone,
    pub bub: [Bubble; MAX_BUB],
    pub pop: [Pop; MAX_POP],
    pub next_bub: usize,
    pub next_pop: usize,
    pub agit: f32,
}

impl BubblesState {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        let x = if rng.gen_bool(0.5) {
            rng.gen_range(L_X0..L_X1)
        } else {
            rng.gen_range(R_X0..R_X1)
        };
        let y = rng.gen_range(STONE_Y0..STONE_Y1);
        let stone = Stone {
            x,
            y,
            emit: rng.gen(),
            throb: rng.gen_range(0.0..PI * 2.0),
            lean: 0.0,
            lean_t: rng.gen_range(0.0..PI * 2.0),
        };
        Self {
            stone,
            bub: [Bubble::default(); MAX_BUB],
            pop: [Pop::default(); MAX_POP],
            next_bub: 0,
            next_pop: 0,
            agit: 0.55,
        }
    }

    fn spawn(&mut self) {
        let mut rng = rand::thread_rng();
        let idx = self.next_bub;
        self.next_bub = if self.next_bub + 1 == MAX_BUB { 0 } else { self.next_bub + 1 };

        let b = &mut self.bub[idx];
        b.x0 = self.stone.x + rng.gen_range(-1.6..1.6);
        b.y = self.stone.y;
        b.r0 = rng.gen_range(R_MIN..R_MAX);
        b.r = b.r0;
        b.vy = 50.0 + b.r0 * 44.0 + rng.gen_range(-5.0..5.0);

        let t: f32 = rng.gen::<f32>() + rng.gen::<f32>() - 1.0;
        b.fan = t * t * t * 0.5 + t * 0.5;
        b.wob_a = rng.gen_range(0.7..2.0) / (0.4 + b.r0);
        b.wob_k = rng.gen_range(0.070..0.150);
        b.wob_p = rng.gen_range(0.0..PI * 2.0);
        b.live = true;
    }

    fn burst(&mut self, x: f32) {
        let mut rng = rand::thread_rng();
        let idx = self.next_pop;
        self.next_pop = if self.next_pop + 1 == MAX_POP { 0 } else { self.next_pop + 1 };

        let p = &mut self.pop[idx];
        p.x = x;
        p.t = 0.0;
        p.dur = rng.gen_range(0.22..0.40);
        p.r1 = rng.gen_range(3.5..7.0);
        p.live = true;
    }

    pub fn step(&mut self, dt: f32) {
        let mut rng = rand::thread_rng();
        // Stone emit
        self.stone.throb += dt * 2.0 * PI * THROB_HZ;
        if self.stone.throb > 2.0 * PI * 64.0 {
            self.stone.throb -= 2.0 * PI * 64.0;
        }
        self.stone.lean_t += dt * 0.37;
        if self.stone.lean_t > 2.0 * PI * 64.0 {
            self.stone.lean_t -= 2.0 * PI * 64.0;
        }
        self.stone.lean = self.stone.lean_t.sin() * 3.4 + (self.stone.lean_t * 2.3 + 1.1).sin() * 1.6;

        let rate = PUFF_HZ * (1.0 + THROB_AMT * self.stone.throb.sin());
        self.stone.emit -= dt * rate;
        while self.stone.emit <= 0.0 {
            self.stone.emit += 1.0;
            self.spawn();
            if rng.gen_bool(0.70) {
                self.spawn();
            }
            if rng.gen_bool(0.40) {
                self.spawn();
            }
            if rng.gen_bool(0.15) {
                self.spawn();
            }
        }

        // Bubbles rise
        let span = 1.0 / (STONE_Y0 - SURF_Y);
        let lean = self.stone.lean;
        for i in 0..MAX_BUB {
            let b = &mut self.bub[i];
            if !b.live {
                continue;
            }
            let mut u = (self.stone.y - b.y) * span;
            if u < 0.0 {
                u = 0.0;
            } else if u > 1.0 {
                u = 1.0;
            }
            b.r = b.r0 * (1.0 + 0.42 * u);
            b.y -= b.vy * (1.0 + 0.22 * u) * dt;
            if b.y <= SURF_Y {
                b.live = false;
                let burst_x = b.x0 + b.fan * FAN_PX + lean;
                self.agit += 0.026;
                let idx = self.next_pop;
                self.next_pop = if self.next_pop + 1 == MAX_POP { 0 } else { self.next_pop + 1 };
                let p = &mut self.pop[idx];
                p.x = burst_x;
                p.t = 0.0;
                p.dur = rng.gen_range(0.22..0.40);
                p.r1 = rng.gen_range(3.5..7.0);
                p.live = true;
            }
        }

        // Pops
        for i in 0..MAX_POP {
            let p = &mut self.pop[i];
            if !p.live {
                continue;
            }
            p.t += dt;
            if p.t >= p.dur {
                p.live = false;
            }
        }

        self.agit -= self.agit * dt * 2.15;
        if self.agit < 0.0 {
            self.agit = 0.0;
        } else if self.agit > 1.0 {
            self.agit = 1.0;
        }
    }

    pub fn air_agitation(&self) -> f32 {
        self.agit
    }

    pub fn air_flow_at(&self, x: f32, y: f32) -> (f32, f32) {
        let u = (self.stone.y - y) * (1.0 / (STONE_Y0 - SURF_Y));
        if u <= 0.0 || u >= 1.06 {
            return (0.0, 0.0);
        }
        let w = LIFT_CORE + FAN_PX * u * u;
        let dx = x - (self.stone.x + self.stone.lean * u * u);
        let t = dx / w;
        if t <= -1.0 || t >= 1.0 {
            return (0.0, 0.0);
        }
        let rad = 1.0 - t * t;
        let env = smoothstep(0.0, 0.22, u) * (1.0 - 0.70 * smoothstep(0.78, 1.04, u));
        let vy = -LIFT_MAX * env * rad;
        let vx = LIFT_SPREAD * smoothstep(0.72, 1.04, u) * t * rad;
        (vx, vy)
    }
}
