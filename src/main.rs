#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use macroquad::prelude::*;
use metroflow::app::App;

fn window_conf() -> Conf {
    Conf {
        window_title: "MetroFlow".to_owned(),
        window_width: 1440,
        window_height: 900,
        high_dpi: true,
        sample_count: 4,
        window_resizable: true,
        fullscreen: cfg!(target_os = "android"),
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut app = App::new();
    loop {
        let dt = get_frame_time();
        app.frame(dt);
        if app.quit {
            break;
        }
        next_frame().await;
    }
}
