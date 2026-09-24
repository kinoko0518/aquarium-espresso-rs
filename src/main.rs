#![allow(dead_code)]
use ::rand::Rng;
use macroquad::prelude::*;
use image::ImageFormat;

mod fastmath;
mod bubbles;
mod light;
mod rig;
mod art;
mod sim;
mod renderer;

use bubbles::BubblesState;
use light::{prep_backdrop, LightState, SCR_W, SCR_H};
use sim::Sim;
use renderer::render_frame;

const BG_BYTES: [&[u8]; 5] = [
    include_bytes!("../assets/bg0.jpg"),
    include_bytes!("../assets/bg1.jpg"),
    include_bytes!("../assets/bg2.jpg"),
    include_bytes!("../assets/bg3.jpg"),
    include_bytes!("../assets/bg4.jpg"),
];

fn window_conf() -> Conf {
    Conf {
        window_title: "aquarium-espresso-rs".to_string(),
        window_width: 1920,
        window_height: 1080,
        fullscreen: false,
        window_resizable: true,
        platform: miniquad::conf::Platform {
            linux_wm_class: "aquarium-espresso-rs",
            ..Default::default()
        },
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    println!("Starting aquarium-espresso-rs...");

    // Decode all 5 backdrops and apply exposure prep
    let mut backdrops: Vec<Vec<u8>> = Vec::with_capacity(5);
    for (i, &bytes) in BG_BYTES.iter().enumerate() {
        let img = image::load_from_memory_with_format(bytes, ImageFormat::Jpeg)
            .expect("Failed to decode background JPEG")
            .to_rgb8();
        let mut rgb = img.into_raw();
        assert_eq!(rgb.len(), SCR_W * SCR_H * 3, "Backdrop {} dimensions mismatch", i);
        prep_backdrop(&mut rgb);
        backdrops.push(rgb);
    }
    println!("Loaded and tone-mapped {} tank backdrops.", backdrops.len());

    let mut current_bg = 0usize;
    let mut light = LightState::new();
    let mut bubbles = BubblesState::new();
    let mut sim = Sim::new();

    // Check for optional card fish
    if let Ok(card_img) = image::open("fish.png") {
        let card_rgba = card_img.resize_exact(
            rig::CARD_W as u32,
            rig::CARD_H as u32,
            image::imageops::FilterType::Lanczos3,
        ).to_rgba8();
        println!("Loaded custom card fish from fish.png ({}x{})", card_rgba.width(), card_rgba.height());
    }

    // RGBA framebuffer and texture
    let mut fb = vec![0u8; SCR_W * SCR_H * 4];
    let image = Image {
        bytes: fb.clone(),
        width: SCR_W as u16,
        height: SCR_H as u16,
    };
    let texture = Texture2D::from_image(&image);
    texture.set_filter(FilterMode::Linear);

    let mut last_tap_time = 0.0;

    loop {
        let dt = get_frame_time().min(0.06);
        let time = get_time();

        // Input handling
        if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::B) {
            current_bg = (current_bg + 1) % backdrops.len();
            println!("Switched to backdrop {}", current_bg);
        }
        if is_key_pressed(KeyCode::R) {
            sim = Sim::new();
            bubbles = BubblesState::new();
            println!("Reset tank simulation.");
        }
        if is_key_pressed(KeyCode::E) {
            sim.ebi_day = !sim.ebi_day;
            for f in sim.fish.iter_mut() {
                if f.home.key == rig::SpKey::Guppy {
                    f.cfg = if sim.ebi_day { rig::EBIFRY } else { f.home };
                }
            }
            println!("Ebi-fry mode: {}", sim.ebi_day);
        }
        if is_key_pressed(KeyCode::Q) || is_key_pressed(KeyCode::Escape) {
            break;
        }

        let sw = screen_width();
        let sh = screen_height();

        // Calculate aspect-fill scale and offsets
        let scale_x = sw / SCR_W as f32;
        let scale_y = sh / SCR_H as f32;
        let scale = scale_x.max(scale_y);
        let draw_w = SCR_W as f32 * scale;
        let draw_h = SCR_H as f32 * scale;
        let offset_x = (sw - draw_w) * 0.5;
        let offset_y = (sh - draw_h) * 0.5;

        // Mouse click or touch tap
        if is_mouse_button_pressed(MouseButton::Left) {
            let (mx, my) = mouse_position();
            let sim_x = (mx - offset_x) / scale;
            let sim_y = (my - offset_y) / scale;
            if sim_x >= 0.0 && sim_x <= SCR_W as f32 && sim_y >= 0.0 && sim_y <= SCR_H as f32 {
                sim.tap_water(sim_x, sim_y);
                last_tap_time = time;
            }
        }

        // Automatic gentle water tap if user is idle for a long time
        if time - last_tap_time > 60.0 {
            let mut rng = ::rand::thread_rng();
            sim.tap_water(rng.gen_range(100.0..540.0), rng.gen_range(100.0..380.0));
            last_tap_time = time;
        }

        // Step simulation
        bubbles.step(dt);
        light.step(dt, bubbles.air_agitation());
        sim.step(dt, &bubbles);

        // Render frame
        render_frame(&mut fb, &backdrops[current_bg], &light, &bubbles, &sim);

        // Upload to GPU texture
        texture.update(&Image {
            bytes: fb.clone(),
            width: SCR_W as u16,
            height: SCR_H as u16,
        });

        // Clear and draw aspect-fill wallpaper
        clear_background(BLACK);
        draw_texture_ex(
            &texture,
            offset_x,
            offset_y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(draw_w, draw_h)),
                ..Default::default()
            },
        );

        next_frame().await;
    }
}
