use std::f32::consts::PI;

#[inline]
pub fn fwrap(mut a: f32) -> f32 {
    while a < -PI {
        a += 2.0 * PI;
    }
    while a > PI {
        a -= 2.0 * PI;
    }
    a
}

#[inline]
pub fn clampf(v: f32, a: f32, b: f32) -> f32 {
    if v < a {
        a
    } else if v > b {
        b
    } else {
        v
    }
}

#[inline]
pub fn smoothstep(a: f32, b: f32, v: f32) -> f32 {
    let t = (v - a) / (b - a);
    if t <= 0.0 {
        0.0
    } else if t >= 1.0 {
        1.0
    } else {
        t * t * (3.0 - 2.0 * t)
    }
}
