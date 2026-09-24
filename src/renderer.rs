use std::f32::consts::PI;
use rayon::prelude::*;
use crate::fastmath::clampf;
use crate::rig::*;
use crate::sim::*;
use crate::light::*;
use crate::bubbles::*;

pub const FB_W: usize = 640;
pub const FB_H: usize = 480;

pub struct PolyPaint {
    pub grad: bool,
    pub additive: bool,
    pub gx0: f32,
    pub gy0: f32,
    pub gx1: f32,
    pub gy1: f32,
    pub r0: f32,
    pub g0: f32,
    pub b0: f32,
    pub a0: f32,
    pub r1: f32,
    pub g1: f32,
    pub b1: f32,
    pub a1: f32,
}

#[inline]
pub fn blend_pixel(fb: &mut [u8], idx: usize, r: f32, g: f32, b: f32, a: f32) {
    if a <= 0.001 { return; }
    let a = a.min(1.0);
    let inv_a = 1.0 - a;
    let dr = fb[idx] as f32;
    let dg = fb[idx + 1] as f32;
    let db = fb[idx + 2] as f32;
    fb[idx] = (r * a + dr * inv_a).clamp(0.0, 255.0) as u8;
    fb[idx + 1] = (g * a + dg * inv_a).clamp(0.0, 255.0) as u8;
    fb[idx + 2] = (b * a + db * inv_a).clamp(0.0, 255.0) as u8;
}

#[inline]
pub fn add_pixel(fb: &mut [u8], idx: usize, r: f32, g: f32, b: f32) {
    fb[idx] = (fb[idx] as f32 + r).min(255.0) as u8;
    fb[idx + 1] = (fb[idx + 1] as f32 + g).min(255.0) as u8;
    fb[idx + 2] = (fb[idx + 2] as f32 + b).min(255.0) as u8;
}

pub fn fill_poly_aa(fb: &mut [u8], xs: &[f32], ys: &[f32], p: &PolyPaint) {
    let n = xs.len();
    if n < 3 { return; }
    let mut ymin = ys[0]; let mut ymax = ys[0];
    let mut xmin = xs[0]; let mut xmax = xs[0];
    for i in 1..n {
        if ys[i] < ymin { ymin = ys[i]; }
        if ys[i] > ymax { ymax = ys[i]; }
        if xs[i] < xmin { xmin = xs[i]; }
        if xs[i] > xmax { xmax = xs[i]; }
    }
    let y0 = ymin.floor().clamp(0.0, FB_H as f32) as usize;
    let y1 = ymax.ceil().clamp(0.0, FB_H as f32) as usize;
    let x0 = xmin.floor().clamp(0.0, FB_W as f32) as usize;
    let x1 = xmax.ceil().clamp(0.0, FB_W as f32) as usize;
    if y0 >= y1 || x0 >= x1 { return; }

    let gdx = p.gx1 - p.gx0;
    let gdy = p.gy1 - p.gy0;
    let mut glen2 = gdx * gdx + gdy * gdy;
    if glen2 < 1e-9 { glen2 = 1e-9; }
    let inv_glen2 = 1.0 / glen2;

    const SUB: usize = 2;
    const INV_SUB: f32 = 1.0 / SUB as f32;
    let mut cx = [0.0f32; 16];

    for y in y0..y1 {
        for s in 0..SUB {
            let sy = y as f32 + (s as f32 + 0.5) * INV_SUB;
            let mut nc = 0;
            for i in 0..n {
                let j = (i + 1) % n;
                let ay = ys[i]; let by = ys[j];
                if ay == by { continue; }
                if (sy >= ay && sy < by) || (sy >= by && sy < ay) {
                    let t = (sy - ay) / (by - ay);
                    if nc < 16 {
                        cx[nc] = xs[i] + (xs[j] - xs[i]) * t;
                        nc += 1;
                    }
                }
            }
            if nc < 2 { continue; }
            cx[..nc].sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            let mut a_idx = 0;
            while a_idx + 1 < nc {
                let mut xa = cx[a_idx];
                let mut xb = cx[a_idx + 1];
                a_idx += 2;
                if xb <= x0 as f32 || xa >= x1 as f32 { continue; }
                if xa < x0 as f32 { xa = x0 as f32; }
                if xb > x1 as f32 { xb = x1 as f32; }
                let ia = xa.floor().max(x0 as f32) as usize;
                let ib = xb.ceil().min(x1 as f32) as usize;
                for x in ia..ib {
                    let l = xa.max(x as f32);
                    let r = xb.min(x as f32 + 1.0);
                    if r <= l { continue; }
                    let cov = (r - l) * INV_SUB;
                    if cov <= 0.002 { continue; }

                    let (rr, gg, bb, aa) = if p.grad {
                        let t = (((x as f32 + 0.5 - p.gx0) * gdx + (y as f32 + 0.5 - p.gy0) * gdy) * inv_glen2).clamp(0.0, 1.0);
                        (
                            p.r0 + (p.r1 - p.r0) * t,
                            p.g0 + (p.g1 - p.g0) * t,
                            p.b0 + (p.b1 - p.b0) * t,
                            p.a0 + (p.a1 - p.a0) * t,
                        )
                    } else {
                        (p.r0, p.g0, p.b0, p.a0)
                    };
                    let final_a = (aa * cov).min(1.0);
                    let idx = (y * FB_W + x) * 4;
                    if p.additive {
                        add_pixel(fb, idx, rr * final_a, gg * final_a, bb * final_a);
                    } else {
                        blend_pixel(fb, idx, rr, gg, bb, final_a);
                    }
                }
            }
        }
    }
}

pub fn line_aa(fb: &mut [u8], x0: f32, y0: f32, x1: f32, y1: f32, w: f32, r: f32, g: f32, b: f32, a: f32) {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let l = (dx * dx + dy * dy).sqrt();
    if l < 1e-4 { return; }
    let nx = -dy / l * w * 0.5;
    let ny = dx / l * w * 0.5;
    let xs = [x0 + nx, x1 + nx, x1 - nx, x0 - nx];
    let ys = [y0 + ny, y1 + ny, y1 - ny, y0 - ny];
    let p = PolyPaint {
        grad: false,
        additive: false,
        gx0: 0.0, gy0: 0.0, gx1: 0.0, gy1: 0.0,
        r0: r, g0: g, b0: b, a0: a,
        r1: r, g1: g, b1: b, a1: a,
    };
    fill_poly_aa(fb, &xs, &ys, &p);
}

pub fn fill_rect_aa(fb: &mut [u8], x: f32, y: f32, w: f32, h: f32, r: f32, g: f32, b: f32, a: f32) {
    if w <= 0.0 || h <= 0.0 || a <= 0.0 { return; }
    let ix0 = x.floor().clamp(0.0, FB_W as f32) as usize;
    let ix1 = (x + w).ceil().clamp(0.0, FB_W as f32) as usize;
    let iy0 = y.floor().clamp(0.0, FB_H as f32) as usize;
    let iy1 = (y + h).ceil().clamp(0.0, FB_H as f32) as usize;
    for j in iy0..iy1 {
        let cy = (y + h).min(j as f32 + 1.0) - y.max(j as f32);
        if cy <= 0.0 { continue; }
        for i in ix0..ix1 {
            let cx = (x + w).min(i as f32 + 1.0) - x.max(i as f32);
            if cx <= 0.0 { continue; }
            let al = a * cx * cy;
            blend_pixel(fb, (j * FB_W + i) * 4, r, g, b, al);
        }
    }
}

pub fn circle_outline_aa(fb: &mut [u8], cx: f32, cy: f32, r: f32, lw: f32, red: f32, green: f32, blue: f32, a: f32) {
    let ro = r + lw * 0.5;
    let x0 = (cx - ro - 1.0).floor().clamp(0.0, FB_W as f32) as usize;
    let x1 = (cx + ro + 1.0).ceil().clamp(0.0, FB_W as f32) as usize;
    let y0 = (cy - ro - 1.0).floor().clamp(0.0, FB_H as f32) as usize;
    let y1 = (cy + ro + 1.0).ceil().clamp(0.0, FB_H as f32) as usize;
    let half = lw * 0.5;
    for y in y0..y1 {
        for x in x0..x1 {
            let mut cov = 0.0f32;
            for sy in 0..2 {
                for sx in 0..2 {
                    let px = x as f32 + 0.25 + sx as f32 * 0.5;
                    let py = y as f32 + 0.25 + sy as f32 * 0.5;
                    let d = ((px - cx) * (px - cx) + (py - cy) * (py - cy)).sqrt();
                    if (d - r).abs() <= half {
                        cov += 0.25;
                    }
                }
            }
            if cov > 0.0 {
                blend_pixel(fb, (y * FB_W + x) * 4, red, green, blue, a * cov);
            }
        }
    }
}

fn draw_bubble(fb: &mut [u8], cx: f32, cy: f32, r: f32, red: f32, green: f32, blue: f32, hi_r: f32, hi_g: f32, hi_b: f32) {
    let ix0 = (cx - r - 1.0).floor().clamp(0.0, FB_W as f32) as usize;
    let ix1 = (cx + r + 1.0).ceil().clamp(0.0, FB_W as f32) as usize;
    let iy0 = (cy - r - 1.0).floor().clamp(0.0, FB_H as f32) as usize;
    let iy1 = (cy + r + 1.0).ceil().clamp(0.0, FB_H as f32) as usize;
    let rim = r - 0.28;
    for y in iy0..iy1 {
        let dy = y as f32 + 0.5 - cy;
        for x in ix0..ix1 {
            let dx = x as f32 + 0.5 - cx;
            let d = (dx * dx + dy * dy).sqrt();
            if d > r + 0.65 { continue; }
            let cov = (1.0 - (d - rim).abs()).clamp(0.0, 1.0);
            if cov <= 0.0 { continue; }
            let hi = ((-dx - dy) * (1.0 / 1.414)).clamp(0.0, 1.0);
            let rr = red * (1.0 - hi) + hi_r * hi;
            let gg = green * (1.0 - hi) + hi_g * hi;
            let bb = blue * (1.0 - hi) + hi_b * hi;
            blend_pixel(fb, (y * FB_W + x) * 4, rr, gg, bb, cov * 0.75);
        }
    }
}

pub fn render_frame(
    fb: &mut [u8],
    backdrop_rgb: &[u8],
    light: &LightState,
    bubbles: &BubblesState,
    sim: &Sim,
) {
    // 1. Parallel Backdrop + Light Application
    let to_idx = LUT_N as f32 / (PI * 2.0);
    let g_top = (light.gain * 256.0 + 0.5) as i32;

    fb.par_chunks_exact_mut(FB_W * 4).enumerate().for_each(|(y, row)| {
        let amp = (light.row_amp[y] as i32 * light.amp_scale) >> 8;
        let d_a = light.row_step_a[y];
        let d_b = light.row_step_b[y];
        let d_d = light.row_step_d[y];
        let mut ua = light.row_ph_a[y];
        let mut ub = light.row_ph_b[y];
        let mut ud = light.row_ph_d[y];
        let uc = ((KCY * y as f32 + light.phase_c) * to_idx * 65536.0) as i32;
        let d_c = (KCX * to_idx * 65536.0) as i32;

        let src_row = &backdrop_rgb[y * FB_W * 3..(y + 1) * FB_W * 3];

        let mut x = 0;
        while x < FB_W {
            let pool_idx = (((uc + d_c * x as i32) >> 16) as usize) & (LUT_N - 1);
            let pool_val = ((255 - light.pool_lut[pool_idx] as i32) * POOL_AMP as i32) >> 9;

            let wa = 255 - light.caustic_lut[((ua >> 16) as usize) & (LUT_N - 1)] as i32;
            let wb = 255 - light.caustic_lut[((ub >> 16) as usize) & (LUT_N - 1)] as i32;
            let wd = 255 - light.caustic_lut[((ud >> 16) as usize) & (LUT_N - 1)] as i32;
            ua += d_a * 2;
            ub += d_b * 2;
            ud += d_d * 2;

            let g = (g_top - (((wa + wb + wd) * amp) >> 9) - pool_val).clamp(32, 256);

            for pair in 0..2 {
                let px = x + pair;
                if px >= FB_W { break; }
                let s_idx = px * 3;
                let d_idx = px * 4;
                let sr = src_row[s_idx] as i32;
                let sg = src_row[s_idx + 1] as i32;
                let sb = src_row[s_idx + 2] as i32;
                row[d_idx] = ((sr * g) >> 8).min(255) as u8;
                row[d_idx + 1] = ((sg * g) >> 8).min(255) as u8;
                row[d_idx + 2] = ((sb * g) >> 8).min(255) as u8;
                row[d_idx + 3] = 255;
            }
            x += 2;
        }
    });

    // 2. Air Stone Bubbles
    for b in &bubbles.bub {
        if !b.live { continue; }
        let cx = (b.x0 + b.fan * FAN_PX + bubbles.stone.lean) * 2.0;
        let cy = b.y * 2.0;
        let rad = b.r * 2.0;
        draw_bubble(fb, cx, cy, rad, 160.0, 220.0, 240.0, 240.0, 250.0, 255.0);
    }
    for p in &bubbles.pop {
        if !p.live { continue; }
        let cx = p.x * 2.0;
        let cy = SURF_Y * 2.0;
        let frac = p.t / p.dur;
        let r = p.r1 * 2.0 * frac;
        let a = (1.0 - frac) * 0.6;
        circle_outline_aa(fb, cx, cy, r, 1.2, 200.0, 240.0, 255.0, a);
    }

    // 3. Motes
    for m in &sim.motes {
        let alpha = (m.a * 1.5).min(0.65);
        fill_rect_aa(fb, m.x - 0.75, m.y - 0.75, 1.5, 1.5, 210.0, 245.0, 235.0, alpha);
    }

    // 4. Fish sorted back-to-front by sf
    let mut order: Vec<usize> = (0..sim.fish.len()).collect();
    order.sort_by(|&a, &b| {
        sim.fish[a].sf.partial_cmp(&sim.fish[b].sf).unwrap_or(std::cmp::Ordering::Equal)
    });

    for &idx in &order {
        let f = &sim.fish[idx];
        let off_x = sim.sway * (0.55 + 0.45 * (f.sf - 0.74) / 0.54);
        if f.cfg.key == SpKey::Card {
            draw_card(fb, f, off_x, light);
        } else {
            draw_fish(fb, f, off_x, light);
        }
    }
}

fn draw_fish(fb: &mut [u8], f: &Fish, off_x: f32, light: &LightState) {
    let c = &f.cfg;
    let d = &f.depth;
    let ch = f.heading.cos();
    let pitch_body = (f.heading.sin().clamp(-0.9, 0.9)) * 0.2 * ch.abs();
    let sgn = if ch >= 0.0 { 1.0 } else { -1.0 };
    let pitch = d.bell * d.sign * c.pitch_max + pitch_body * -sgn;
    let s_y = (1.0 - 0.5 * pitch.sin().abs()).clamp(0.78, 1.0);
    let sf = f.sf;
    let facing = f.facing;
    let drift = (1.0 - sf) * 16.0 * facing;
    let mir = f.mirror;

    let gag = c.key == SpKey::EbiFry;
    let lg = if gag { 1.0 } else { light.fish_light((f.x + off_x) as i32, f.y as i32) };
    let h = if gag { 0.0 } else { LightState::haze_for(f.sf) };
    let k = 1.0 - h;

    // Guppy caudal veil
    if c.key == SpKey::Guppy {
        let flare = f.flare;
        let mut px = [0.0f32; 5];
        let mut py = [0.0f32; 5];
        for k_idx in 0..=4 {
            let back = c.spacing * (BONES - 1) as f32 - 3.0 + k_idx as f32 * 3.0;
            let (ox, oy) = trail_at(&f.trail, back);
            px[k_idx] = ox + off_x;
            py[k_idx] = oy;
        }

        let mut lx = [0.0f32; 5];
        let mut ly = [0.0f32; 5];
        let mut rx = [0.0f32; 5];
        let mut ry = [0.0f32; 5];

        for k_idx in 0..=4 {
            let qx = px[if k_idx < 4 { k_idx + 1 } else { 4 }];
            let qy = py[if k_idx < 4 { k_idx + 1 } else { 4 }];
            let ang = (py[k_idx] - qy).atan2(px[k_idx] - qx);
            let wave = (f.beat * 1.15 - k_idx as f32 * 1.25).sin() * (0.5 + k_idx as f32 * 0.6) * (0.35 + flare);
            let spread = (k_idx as f32 * 1.1 + 0.4) * (1.0 + flare * 1.7) * s_y;
            let nx = (ang + PI * 0.5).cos();
            let ny = (ang + PI * 0.5).sin();
            lx[k_idx] = px[k_idx] + nx * (spread + wave * 0.85);
            ly[k_idx] = py[k_idx] + ny * (spread + wave * 0.85);
            rx[k_idx] = px[k_idx] - nx * (spread - wave * 0.85);
            ry[k_idx] = py[k_idx] - ny * (spread - wave * 0.85);
        }

        let mut xs = [0.0f32; 10];
        let mut ys = [0.0f32; 10];
        for k_idx in 0..5 {
            xs[k_idx] = lx[k_idx]; ys[k_idx] = ly[k_idx];
            xs[5 + k_idx] = rx[4 - k_idx]; ys[5 + k_idx] = ry[4 - k_idx];
        }

        let (mut r0, mut g0, mut b0, a0, mut r1, mut g1, mut b1, a1) = match c.strain {
            1 => (235.0, 210.0, 90.0, 0.06 + flare * 0.30, 200.0, 170.0, 50.0, 0.03),
            2 => (250.0, 95.0, 25.0, 0.06 + flare * 0.32, 210.0, 50.0, 15.0, 0.03),
            _ => (250.0, 105.0, 60.0, 0.06 + flare * 0.32, 210.0, 40.0, 40.0, 0.03),
        };

        r0 = r0 * lg * k + HAZE_R * lg * h;
        g0 = g0 * lg * k + HAZE_G * lg * h;
        b0 = b0 * lg * k + HAZE_B * lg * h;
        r1 = r1 * lg * k + HAZE_R * lg * h;
        g1 = g1 * lg * k + HAZE_G * lg * h;
        b1 = b1 * lg * k + HAZE_B * lg * h;

        let vp = PolyPaint {
            grad: true,
            additive: false,
            gx0: lx[0], gy0: ly[0],
            gx1: lx[4], gy1: ly[4],
            r0, g0, b0, a0,
            r1, g1, b1, a1,
        };
        fill_poly_aa(fb, &xs, &ys, &vp);
    }

    // Body segments
    for b in 0..BONES - 1 {
        draw_segment(fb, f, b, s_y, sf, drift, off_x, mir, lg, h);
    }

    // Neon iridescent stripe glow
    if c.key == SpKey::Neon {
        let m = &f.bones[1];
        let glow = 0.6 + 0.4 * (f.beat * 0.5 + f.phase).sin() + (1.0 - sf) * 0.8;
        let ca = m.a.cos();
        let sa = m.a.sin();
        let ox = m.x + off_x;
        let oy = m.y;
        let w = c.w as f32;
        let lx0 = -w * 0.4;
        let lx1 = w * 0.45;
        let lxs = [lx0, lx1, lx1, lx0];
        let lys = [-1.4f32, -1.4, 1.4, 1.4];
        let mut xs = [0.0f32; 4];
        let mut ys = [0.0f32; 4];
        for i in 0..4 {
            xs[i] = ox + lxs[i] * ca - lys[i] * sa;
            ys[i] = oy + lxs[i] * sa + lys[i] * ca;
        }
        let gp = PolyPaint {
            grad: true,
            additive: true,
            gx0: ox + lx0 * ca, gy0: oy + lx0 * sa,
            gx1: ox + lx1 * ca, gy1: oy + lx1 * sa,
            r0: 90.0 * lg * k, g0: 240.0 * lg * k, b0: 255.0 * lg * k,
            a0: (0.55 * glow).min(1.0),
            r1: 60.0 * lg * k, g1: 200.0 * lg * k, b1: 235.0 * lg * k,
            a1: (0.25 * glow).min(1.0),
        };
        fill_poly_aa(fb, &xs, &ys, &gp);
    }

    let head = &f.bones[0];
    let hx = head.a.cos();
    let hy = head.a.sin();

    if c.key == SpKey::Yamato {
        // Antennae
        let bx = head.x + off_x + hx * c.spacing * 0.55 * sf;
        let by = head.y + hy * c.spacing * 0.55 * sf;
        let len_tot = c.w as f32 * 0.40 * sf;
        let (ar, ag, ab) = (176.0 * lg * k + HAZE_R * lg * h, 192.0 * lg * k + HAZE_G * lg * h, 180.0 * lg * k + HAZE_B * lg * h);

        for side in 0..2 {
            let sgn_side = if side == 1 { 1.0 } else { -1.0 };
            let w = (f.beat * 0.55 + f.phase + side as f32 * 2.1).sin() * (0.15 + 0.55 * f.thrash);
            let mut a = head.a + sgn_side * (0.30 + 1.05 * f.speed_norm + w);
            let mut px = bx;
            let mut py = by;
            let mut al = 0.22;
            for k_seg in 0..3 {
                let seg_len = len_tot * (0.42 - 0.09 * k_seg as f32);
                let nx = px + a.cos() * seg_len;
                let ny = py + a.sin() * seg_len;
                line_aa(fb, px, py, nx, ny, 1.2, ar, ag, ab, al);
                px = nx; py = ny;
                a += sgn_side * (0.24 + w * 0.45);
                al *= 0.60;
            }
        }
        // Swimmerets
        let ab_bone = &f.bones[2];
        let beat_sw = (f.beat * 3.1 + f.phase).sin() * 0.5;
        let pax = ab_bone.a + mir * (1.9 + beat_sw * 0.5);
        let pl = 4.2 * (0.6 + 0.9 * f.effort.clamp(0.0, 1.0));
        let (sr, sg, sb) = (196.0 * lg * k + HAZE_R * lg * h, 206.0 * lg * k + HAZE_G * lg * h, 194.0 * lg * k + HAZE_B * lg * h);
        line_aa(fb, ab_bone.x + off_x, ab_bone.y, ab_bone.x + off_x + pax.cos() * pl, ab_bone.y + pax.sin() * pl, 1.2, sr, sg, sb, 0.18);
    } else {
        // Eye
        let ex = head.x + off_x + hx * 2.0 * sf + mir * hy * 1.8 * s_y;
        let ey = head.y + hy * 2.0 * sf - mir * hx * 1.8 * s_y;
        let (er, eg, eb) = (6.0 * lg * k + HAZE_R * lg * h, 14.0 * lg * k + HAZE_G * lg * h, 16.0 * lg * k + HAZE_B * lg * h);
        fill_rect_aa(fb, ex - 0.9, ey - 0.9, 1.8, 1.8, er, eg, eb, 0.85);

        // Pectoral fin
        let flap = (f.beat * 2.1 + f.phase).sin() * (0.5 + d.bell + f.flare * 0.4);
        let pb = &f.bones[1];
        let pa = pb.a + mir * (PI * 0.5 + flap * 0.55);
        let pl = (if c.key == SpKey::Neon { 4.4 } else { 6.4 }) * (1.0 + d.bell * 0.4);
        let bx = pb.x + off_x + pb.a.cos() * 3.0;
        let by = pb.y + pb.a.sin() * 3.0;

        let (fr, fg, fb_col) = if c.key == SpKey::Neon {
            (170.0 * lg * k + HAZE_R * lg * h, 235.0 * lg * k + HAZE_G * lg * h, 228.0 * lg * k + HAZE_B * lg * h)
        } else {
            (235.0 * lg * k + HAZE_R * lg * h, 150.0 * lg * k + HAZE_G * lg * h, 110.0 * lg * k + HAZE_B * lg * h)
        };
        line_aa(fb, bx, by, bx + pa.cos() * pl, by + pa.sin() * pl, 1.4, fr, fg, fb_col, 0.60);
    }
}

fn draw_segment(
    fb: &mut [u8],
    f: &Fish,
    b: usize,
    s_y: f32,
    sf: f32,
    drift: f32,
    off_x: f32,
    mirror: f32,
    lg: f32,
    h: f32,
) {
    let c = &f.cfg;
    let w = c.w;
    let h_dim = c.h;
    let segs = BONES - 1;
    let seg_w = w as f32 / segs as f32;
    let seg_wi = seg_w as usize;
    let kappa = (PI * 2.0) / (2.6 * w as f32);

    let b0 = &f.bones[b];
    let b1 = &f.bones[b + 1];
    let mx = (b0.x + b1.x) * 0.5 + off_x;
    let my = (b0.y + b1.y) * 0.5;
    let ang = (b0.y - b1.y).atan2(b0.x - b1.x);
    let src_x = (w as f32 - (b + 1) as f32 * seg_w) as usize;

    let sweep = (f.beat * 0.37 + f.phase).sin() * 0.25;
    let spd = (0.7 + 0.3 * f.speed_norm + f.thrash * 1.4) * s_y;
    let open = f.flare * 0.55;
    let fan_sweep = 0.35 + 1.1 * f.flare;
    let fin_span = (w - 1) as f32 - c.fin_from;

    let mut lat_col = [0.0f32; 16];
    let mut shrink_col = [1.0f32; 16];
    let mut lat_min = 1e9f32;
    let mut lat_max = -1e9f32;
    let mut exp_max = 1.0f32;

    for j in 0..seg_wi.min(16) {
        let s_arc = (w - 1 - (src_x + j)) as f32;
        let mut fan_w = (s_arc - c.fin_from) / if fin_span > 0.1 { fin_span } else { 1.0 };
        fan_w = clampf(fan_w, 0.0, 1.0);

        let t = s_arc / c.spacing;
        let mut ti = t as usize;
        if ti > BONES - 2 { ti = BONES - 2; }
        let mut amp = c.a[ti] + (c.a[ti + 1] - c.a[ti]) * (t - ti as f32);
        amp *= (1.0 - fan_w) + fan_w * fan_sweep;

        let v = amp * (f.beat - kappa * s_arc + sweep).sin() * spd;
        lat_col[j] = v;
        if v < lat_min { lat_min = v; }
        if v > lat_max { lat_max = v; }

        let expand = 1.0 + open * fan_w;
        shrink_col[j] = 1.0 / expand;
        if expand > exp_max { exp_max = expand; }
    }

    let ca = ang.cos();
    let sa = ang.sin();
    let tx = mx + drift * ca;
    let ty = my + drift * sa;

    let dw = seg_w * sf + 2.4;
    let lx0 = -seg_w * sf * 0.5 - 0.6;
    let rip_a = (c.a[b] + c.a[b + 1]) * 0.5 * c.fan_ripple * s_y * (0.25 + 0.9 * f.flare);
    let mid = (h_dim - 1) as f32 * 0.5;

    let mut dyv = [0.0f32; 32];
    let mut dy_lo = 1e9f32;
    let mut dy_hi = -1e9f32;
    for r in 0..h_dim.min(32) {
        let val = (r as f32 - mid) * s_y + rip_a * (f.beat * 0.53 + r as f32 * 0.55 + f.phase).sin() - 0.5;
        dyv[r] = val;
        if val < dy_lo { dy_lo = val; }
        if val > dy_hi { dy_hi = val; }
    }
    dy_hi += 2.36;

    let slack = (exp_max - 1.0) * 1.2 + 0.3;
    let mut ly_top = dy_lo * exp_max + lat_min - slack;
    let mut ly_bot = dy_hi * exp_max + lat_max + slack;
    if mirror < 0.0 {
        let tmp = ly_top;
        ly_top = -ly_bot;
        ly_bot = -tmp;
    }

    let cx = [lx0, lx0 + dw, lx0 + dw, lx0];
    let cy = [ly_top, ly_top, ly_bot, ly_bot];
    let mut xmin = 1e9f32; let mut xmax = -1e9f32;
    let mut ymin = 1e9f32; let mut ymax = -1e9f32;
    for i in 0..4 {
        let wx = tx + cx[i] * ca - cy[i] * sa;
        let wy = ty + cx[i] * sa + cy[i] * ca;
        if wx < xmin { xmin = wx; }
        if wx > xmax { xmax = wx; }
        if wy < ymin { ymin = wy; }
        if wy > ymax { ymax = wy; }
    }

    let ix0 = xmin.floor().clamp(0.0, FB_W as f32) as usize;
    let ix1 = (xmax.ceil() + 1.0).clamp(0.0, FB_W as f32) as usize;
    let iy0 = ymin.floor().clamp(0.0, FB_H as f32) as usize;
    let iy1 = (ymax.ceil() + 1.0).clamp(0.0, FB_H as f32) as usize;
    if ix0 >= ix1 || iy0 >= iy1 { return; }

    let inv_dw = 1.0 / dw;
    let inv_sy = 1.0 / s_y;
    let u_scale = inv_dw * seg_w;
    let lx_end = lx0 + dw;
    let inv_ca = if ca.abs() > 1e-4 { 1.0 / ca } else { 0.0 };
    let inv_sa = if sa.abs() > 1e-4 { 1.0 / sa } else { 0.0 };

    let k_blend = 1.0 - h;
    let spr = c.art;

    for y in iy0..iy1 {
        let dxc = (ix0 as f32 + 0.5) - tx;
        let dyc = (y as f32 + 0.5) - ty;
        let lxc = dxc * ca + dyc * sa;
        let lyc = -dxc * sa + dyc * ca;
        let mut u0 = 0.0f32;
        let mut u1 = (ix1 - ix0) as f32;

        if inv_ca != 0.0 {
            let mut a0 = (lx0 - lxc) * inv_ca;
            let mut a1 = (lx_end - lxc) * inv_ca;
            if a0 > a1 { std::mem::swap(&mut a0, &mut a1); }
            if a0 > u0 { u0 = a0; }
            if a1 < u1 { u1 = a1; }
        } else if lxc < lx0 || lxc >= lx_end {
            continue;
        }

        if inv_sa != 0.0 {
            let mut a0 = (lyc - ly_bot) * inv_sa;
            let mut a1 = (lyc - ly_top) * inv_sa;
            if a0 > a1 { std::mem::swap(&mut a0, &mut a1); }
            if a0 > u0 { u0 = a0; }
            if a1 < u1 { u1 = a1; }
        } else if lyc < ly_top || lyc >= ly_bot {
            continue;
        }

        if u1 < u0 { continue; }
        let xs = (ix0 as f32 + u0).floor().max(ix0 as f32) as usize;
        let xe = (ix0 as f32 + u1).ceil().min(ix1 as f32) as usize;
        if xs >= xe { continue; }

        for x in xs..xe {
            let dxw = x as f32 + 0.5 - tx;
            let dyw = y as f32 + 0.5 - ty;
            let lx = dxw * ca + dyw * sa;
            let ly = -dxw * sa + dyw * ca;
            if lx < lx0 || lx >= lx_end { continue; }

            let col = ((lx - lx0) * u_scale).floor().clamp(0.0, seg_wi as f32 - 1.0) as usize;
            let src_col = src_x + col;
            if src_col >= w { continue; }

            let mut ly_adj = if mirror < 0.0 { -ly } else { ly };
            ly_adj -= lat_col[col.min(15)];
            ly_adj *= shrink_col[col.min(15)];

            let row_f = mid + ly_adj * inv_sy;
            let r_i = row_f.floor() as i32;
            let fv = row_f - r_i as f32;

            if r_i < -1 || r_i >= h_dim as i32 { continue; }

            let sample = |ri: i32| -> (f32, f32, f32, f32) {
                if ri < 0 || ri >= h_dim as i32 {
                    (0.0, 0.0, 0.0, 0.0)
                } else {
                    let idx = (ri as usize * w + src_col) * 4;
                    if idx + 3 < spr.len() {
                        (spr[idx] as f32, spr[idx + 1] as f32, spr[idx + 2] as f32, spr[idx + 3] as f32 / 255.0)
                    } else {
                        (0.0, 0.0, 0.0, 0.0)
                    }
                }
            };

            let p0 = sample(r_i);
            let p1 = sample(r_i + 1);
            let a = p0.3 * (1.0 - fv) + p1.3 * fv;
            if a < 0.01 { continue; }

            let r = (p0.0 * (1.0 - fv) + p1.0 * fv) * lg * k_blend + HAZE_R * lg * h;
            let g = (p0.1 * (1.0 - fv) + p1.1 * fv) * lg * k_blend + HAZE_G * lg * h;
            let b = (p0.2 * (1.0 - fv) + p1.2 * fv) * lg * k_blend + HAZE_B * lg * h;

            blend_pixel(fb, (y * FB_W + x) * 4, r, g, b, a);
        }
    }
}

fn draw_card(fb: &mut [u8], f: &Fish, off_x: f32, light: &LightState) {
    if f.cfg.art.is_empty() { return; }
    let sf = f.sf;
    let flip = f.turn.cos();
    let st = f.turn.sin();
    let mir = if flip >= 0.0 { 1.0 } else { -1.0 };
    let ax = f.x + off_x;
    let cy = f.y;
    let wide = CARD_W as f32 * sf * flip.abs();
    let half_h = CARD_H as f32 * 0.5 * sf;
    if wide < 1.4 { return; }

    let lean = f.heading.sin().clamp(-0.85, 0.85) * 0.42 * mir;
    let lg = light.fish_light(ax as i32, cy as i32);
    let h = LightState::haze_for(sf);
    let k = 1.0 - h;

    let ix0 = if mir > 0.0 { (ax - wide).floor().max(0.0) as usize } else { ax.floor().max(0.0) as usize };
    let ix1 = if mir > 0.0 { (ax + 1.0).ceil().min(FB_W as f32) as usize } else { (ax + wide + 1.0).ceil().min(FB_W as f32) as usize };
    let inv_wide = 1.0 / wide;

    for x in ix0..ix1 {
        let d = if mir > 0.0 { ax - (x as f32 + 0.5) } else { (x as f32 + 0.5) - ax };
        let q = d * inv_wide;
        if q < 0.0 || q >= 1.0 { continue; }

        let ph = f.beat - q * 3.8;
        let wsin = ph.sin();
        let dy = 0.28 * q * q * wsin * CARD_H as f32 * sf + (x as f32 + 0.5 - ax) * lean;
        let dn = mir * (1.0 - 2.0 * q);
        let sy = (1.0 - 0.25 * q * ph.cos().abs()) * (1.0 + 0.22 * st * dn);

        let top = cy + dy - half_h * sy;
        let col_h = half_h * 2.0 * sy;
        if col_h < 1.0 { continue; }
        let inv_h = CARD_H as f32 / col_h;

        let iy0 = top.floor().clamp(0.0, FB_H as f32) as usize;
        let iy1 = (top + col_h + 1.0).ceil().clamp(0.0, FB_H as f32) as usize;

        let u = ((1.0 - q) * CARD_W as f32).clamp(0.0, CARD_W as f32 - 1.0) as usize;
        let spr = f.cfg.art;

        for y in iy0..iy1 {
            let v_f = (y as f32 + 0.5 - top) * inv_h;
            if v_f < 0.0 || v_f >= CARD_H as f32 { continue; }
            let v = v_f as usize;
            let idx = (v * CARD_W + u) * 4;
            if idx + 3 >= spr.len() { continue; }
            let a = spr[idx + 3] as f32 / 255.0;
            if a < 0.02 { continue; }
            let r = spr[idx] as f32 * lg * k + HAZE_R * lg * h;
            let g = spr[idx + 1] as f32 * lg * k + HAZE_G * lg * h;
            let b = spr[idx + 2] as f32 * lg * k + HAZE_B * lg * h;
            blend_pixel(fb, (y * FB_W + x) * 4, r, g, b, a);
        }
    }
}
