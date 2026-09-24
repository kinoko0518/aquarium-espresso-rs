use std::f32::consts::PI;
use rand::Rng;

pub const SCR_W: usize = 640;
pub const SCR_H: usize = 480;

pub const FLICK_HZ: f32 = 8.3;
pub const FLICK_DEPTH: f32 = 0.022;
pub const DIP_DEPTH: f32 = 0.09;
pub const CAUSTIC_AMP: f32 = 52.0;

pub const FLOOR_AMP: f32 = 116.0;
pub const FLOOR_TOP: f32 = 300.0; // Scaled to 640x480 (150 * 2)
pub const FLOOR_BOT: f32 = 480.0; // 240 * 2
pub const FLOOR_FADE: f32 = 48.0; // 24 * 2
pub const FLOOR_PERSP: f32 = 0.45;
pub const FLOOR_NEAR: f32 = 0.75;
pub const POOL_AMP: f32 = 30.0;

pub const AGIT_SPEED: f32 = 1.45;
pub const AGIT_SWAY: f32 = 0.90;
pub const AGIT_MORPH: f32 = 1.70;
pub const AGIT_BITE: f32 = 78.0;

pub const EXPO_GAMMA: f32 = 1.35;
pub const EXPO_GAIN: f32 = 3.8;

pub const LUT_N: usize = 512;

// Wavenumbers adjusted for 640x480 (divided by 2)
pub const A_ANG: f32 = 0.36;
pub const A_K: f32 = 0.0589 * 0.5;
pub const B_ANG: f32 = -0.945;
pub const B_K: f32 = 0.0580 * 0.5;
pub const D_ANG: f32 = 1.396;
pub const D_K: f32 = 0.0533 * 0.5;
pub const KCX: f32 = 0.013 * 0.5;
pub const KCY: f32 = 0.009 * 0.5;

pub const HAZE_MAX: f32 = 0.10;
pub const HAZE_R: f32 = 46.0;
pub const HAZE_G: f32 = 86.0;
pub const HAZE_B: f32 = 92.0;
pub const FISH_LIGHT_MIX: f32 = 0.30;

pub struct LightState {
    pub caustic_lut: [u8; LUT_N],
    pub pool_lut: [u8; LUT_N],
    pub row_amp: [u8; SCR_H],
    pub floor_mix: [f32; SCR_H],
    pub row_ph_a: [i32; SCR_H],
    pub row_ph_b: [i32; SCR_H],
    pub row_ph_d: [i32; SCR_H],
    pub row_step_a: [i32; SCR_H],
    pub row_step_b: [i32; SCR_H],
    pub row_step_d: [i32; SCR_H],

    pub phase_a: f32,
    pub phase_b: f32,
    pub phase_c: f32,
    pub phase_d: f32,
    pub morph_t: f32,
    pub sway_t: f32,
    pub flick_phase: f32,
    pub dip_timer: f32,
    pub dip_t: f32,
    pub dip_len: f32,
    pub gain: f32,
    pub amp_scale: i32,
}

impl LightState {
    pub fn new() -> Self {
        let mut caustic_lut = [0u8; LUT_N];
        let mut pool_lut = [0u8; LUT_N];

        for i in 0..LUT_N {
            let u = i as f32 * (PI * 2.0 / LUT_N as f32);
            let v = u.sin() * 0.60 + (u * 2.3 + 1.1).sin() * 0.30 + (u * 3.7 + 2.4).sin() * 0.18;
            let s = if v > 0.0 {
                v.powf(2.2)
            } else {
                -(-v).powf(1.6) * 0.35
            };
            let q = 128 + (s * 127.0) as i32;
            caustic_lut[i] = q.clamp(0, 255) as u8;
            pool_lut[i] = (128 + (u.sin() * 126.0) as i32).clamp(0, 255) as u8;
        }

        let mut row_amp = [0u8; SCR_H];
        let mut floor_mix = [0.0f32; SCR_H];

        for y in 0..SCR_H {
            let d = y as f32 / (SCR_H - 1) as f32;
            let water = CAUSTIC_AMP * (1.0 - 0.55 * d * d);

            let mut t = (y as f32 - FLOOR_TOP) / FLOOR_FADE;
            t = t.clamp(0.0, 1.0);
            let m = t * t * (3.0 - 2.0 * t);
            floor_mix[y] = m;

            let a = (water * (1.0 - m) + FLOOR_AMP * m) * (2.0 / 3.0);
            row_amp[y] = a.clamp(0.0, 255.0) as u8;
        }

        Self {
            caustic_lut,
            pool_lut,
            row_amp,
            floor_mix,
            row_ph_a: [0; SCR_H],
            row_ph_b: [0; SCR_H],
            row_ph_d: [0; SCR_H],
            row_step_a: [0; SCR_H],
            row_step_b: [0; SCR_H],
            row_step_d: [0; SCR_H],
            phase_a: 0.0,
            phase_b: 0.0,
            phase_c: 0.0,
            phase_d: 0.0,
            morph_t: 0.0,
            sway_t: 0.0,
            flick_phase: 0.0,
            dip_timer: 3.0,
            dip_t: 0.0,
            dip_len: 0.1,
            gain: 1.0,
            amp_scale: 256,
        }
    }

    pub fn step(&mut self, dt: f32, ag: f32) {
        let mut rng = rand::thread_rng();
        let spd = 1.0 + AGIT_SPEED * ag;
        self.amp_scale = 256 + (AGIT_BITE * ag) as i32;

        self.sway_t += dt * 0.21 * (1.0 + AGIT_SWAY * ag);
        let sw = self.sway_t.sin();
        self.phase_a += dt * (0.75 + 0.55 * sw) * spd;
        self.phase_b -= dt * (0.45 + 0.35 * (self.sway_t * 0.7 + 1.3).sin()) * spd;
        self.phase_d += dt * (0.58 + 0.30 * (self.sway_t * 1.3 + 2.6).sin()) * spd;
        self.phase_c += dt * (0.16 + 0.13 * (self.sway_t * 0.5 - 0.6).sin()) * (1.0 + 0.35 * ag);
        self.morph_t += dt * (0.115 + AGIT_MORPH * 0.115 * ag);

        let tau = PI * 2.0;
        while self.phase_a >= tau { self.phase_a -= tau; }
        while self.phase_a < 0.0 { self.phase_a += tau; }
        while self.phase_b >= tau { self.phase_b -= tau; }
        while self.phase_b < 0.0 { self.phase_b += tau; }
        while self.phase_c >= tau { self.phase_c -= tau; }
        while self.phase_c < 0.0 { self.phase_c += tau; }
        while self.phase_d >= tau { self.phase_d -= tau; }
        while self.phase_d < 0.0 { self.phase_d += tau; }
        while self.sway_t >= tau * 64.0 { self.sway_t -= tau * 64.0; }
        while self.morph_t >= tau * 64.0 { self.morph_t -= tau * 64.0; }

        self.flick_phase += dt * tau * FLICK_HZ;
        if self.flick_phase > tau * 1024.0 {
            self.flick_phase -= tau * 1024.0;
        }
        let rip = 0.6 * self.flick_phase.sin() + 0.4 * (self.flick_phase * 2.0 + 0.7).sin();
        let mut g = 1.0 - FLICK_DEPTH * (1.0 - rip);

        self.dip_timer -= dt;
        if self.dip_timer <= 0.0 && self.dip_t <= 0.0 {
            self.dip_timer = 2.5 + rng.gen::<f32>() * 7.0;
            self.dip_len = 0.05 + rng.gen::<f32>() * 0.12;
            self.dip_t = self.dip_len;
        }
        if self.dip_t > 0.0 {
            self.dip_t -= dt;
            let p = 1.0 - if self.dip_t > 0.0 { self.dip_t / self.dip_len } else { 0.0 };
            g *= 1.0 - DIP_DEPTH * (PI * p).sin();
        }
        self.gain = g;

        let a_a = A_ANG + 0.34 * (self.morph_t * 0.83).sin();
        let a_b = B_ANG + 0.29 * (self.morph_t * 0.61 + 2.1).sin();
        let a_d = D_ANG + 0.31 * (self.morph_t * 1.07 + 4.4).sin();
        let k_a = A_K * (1.0 + 0.17 * (self.morph_t * 0.47 + 1.7).sin());
        let k_b = B_K * (1.0 + 0.15 * (self.morph_t * 0.71 - 0.9).sin());
        let k_d = D_K * (1.0 + 0.19 * (self.morph_t * 0.55 + 3.3).sin());

        let kax = k_a * a_a.cos();
        let kay = k_a * a_a.sin();
        let kbx = k_b * a_b.cos();
        let kby = k_b * a_b.sin();
        let kdx = k_d * a_d.cos();
        let kdy = k_d * a_d.sin();

        let to_idx = LUT_N as f32 / (PI * 2.0);
        let base_a = kax * to_idx * 65536.0;
        let base_b = kbx * to_idx * 65536.0;
        let base_d = kdx * to_idx * 65536.0;

        for y in 0..SCR_H {
            let m = self.floor_mix[y];
            let mut persp = 1.0f32;
            if m > 0.0 {
                let mut u = (y as f32 - FLOOR_TOP) / (FLOOR_BOT - FLOOR_TOP);
                u = u.clamp(0.0, 1.0);
                let z = 1.0 / (FLOOR_PERSP + u * (1.0 - FLOOR_PERSP));
                persp = 1.0 + m * (FLOOR_NEAR * z - 1.0);
            }
            self.row_ph_a[y] = ((kay * y as f32 + self.phase_a) * to_idx * 65536.0) as i32;
            self.row_ph_b[y] = ((kby * y as f32 + self.phase_b) * to_idx * 65536.0) as i32;
            self.row_ph_d[y] = ((kdy * y as f32 + self.phase_d) * to_idx * 65536.0) as i32;
            self.row_step_a[y] = (base_a * persp) as i32;
            self.row_step_b[y] = (base_b * persp) as i32;
            self.row_step_d[y] = (base_d * persp) as i32;
        }
    }

    pub fn light_gain_at(&self, x: i32, y: i32) -> i32 {
        let y = y.clamp(0, SCR_H as i32 - 1) as usize;
        let x = x.clamp(0, SCR_W as i32 - 1) as i64;
        let to_idx = LUT_N as f32 / (PI * 2.0);

        let ia = (((self.row_ph_a[y] as i64 + self.row_step_a[y] as i64 * x) >> 16) as usize) & (LUT_N - 1);
        let ib = (((self.row_ph_b[y] as i64 + self.row_step_b[y] as i64 * x) >> 16) as usize) & (LUT_N - 1);
        let id = (((self.row_ph_d[y] as i64 + self.row_step_d[y] as i64 * x) >> 16) as usize) & (LUT_N - 1);
        let ic = ((KCX * x as f32 + KCY * y as f32 + self.phase_c) * to_idx) as usize & (LUT_N - 1);

        let wa = 255 - self.caustic_lut[ia] as i32;
        let wb = 255 - self.caustic_lut[ib] as i32;
        let wd = 255 - self.caustic_lut[id] as i32;
        let wc = 255 - self.pool_lut[ic] as i32;

        let amp = (self.row_amp[y] as i32 * self.amp_scale) >> 8;
        let g = (self.gain * 256.0 + 0.5) as i32 - (((wa + wb + wd) * amp) >> 9) - ((wc * POOL_AMP as i32) >> 9);
        g.clamp(32, 256)
    }

    pub fn fish_light(&self, x: i32, y: i32) -> f32 {
        let g = self.light_gain_at(x, y) as f32 * (1.0 / 256.0);
        1.0 - (1.0 - g) * FISH_LIGHT_MIX
    }

    pub fn haze_for(sf: f32) -> f32 {
        let h = (1.28 - sf) * (HAZE_MAX / 0.54);
        h.clamp(0.0, HAZE_MAX)
    }
}

pub fn expose(v: f32) -> f32 {
    let y = v.powf(EXPO_GAMMA) * EXPO_GAIN;
    y * (1.0 + y / (EXPO_GAIN * EXPO_GAIN)) / (1.0 + y)
}

pub fn prep_backdrop(rgb_data: &mut [u8]) {
    let mut lut = [0u8; 256];
    for i in 0..256 {
        let v = i as f32 / 255.0;
        let exp = (expose(v) * 255.0 + 0.5) as i32;
        lut[i] = exp.clamp(0, 255) as u8;
    }
    for p in rgb_data.iter_mut() {
        *p = lut[*p as usize];
    }
}
