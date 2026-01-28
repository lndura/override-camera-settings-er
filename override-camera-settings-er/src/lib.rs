use std::time::Duration;

use eldenring::{
    cs::{CSCamExt, CSCamera, CSTaskGroupIndex, CSTaskImp},
    fd4::FD4TaskData,
    util::system::wait_for_system_init,
};
use fromsoftware_shared::{FromStatic, Program, SharedTaskImpExt};
use serde::Deserialize;

const CAMERA_CONFIG_TOML: &str = "camera_config.toml";

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct CameraSettings {
    field_of_view: f32,
    distance_multiplier: f32,
    render_distance_start: f32,
    render_distance_end: f32,
    aspect_width: f32,
    aspect_height: f32,
}

struct CameraConfig {
    field_of_view: f32,
    distance_multiplier: f32,
    render_distance_start: f32,
    render_distance_end: f32,
    aspect_ratio: f32,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            field_of_view: 48.0f32.to_radians(),
            distance_multiplier: 1.0,
            render_distance_start: 0.05,
            render_distance_end: 10_000.0,
            aspect_ratio: 16.0 / 9.0,
        }
    }
}

fn read_camera_config() -> Option<CameraConfig> {

    let config = std::env::current_exe()
        .ok()?
        .parent()?
        .join(CAMERA_CONFIG_TOML);

    let config_file = std::fs::read_to_string(config.as_path()).ok()?;

    let camera_settings = toml::from_str::<CameraSettings>(&config_file).ok()?;

    let mut camera_config = CameraConfig::default();

    camera_config.field_of_view = camera_settings.field_of_view.to_radians();
    camera_config.render_distance_end = camera_settings.render_distance_end;
    camera_config.render_distance_start = camera_settings.render_distance_start;
    camera_config.aspect_ratio = camera_settings.aspect_width / camera_settings.aspect_height;

    Some(camera_config)
}

fn adjust_camera_task() {
    let Some(cs_task) = unsafe { CSTaskImp::instance() }.ok() else {
        return;
    };

    let Some(mut camera_config) = read_camera_config() else {
        return;
    };

    // This code will run every frame, after the CameraStep task.
    cs_task.run_recurring(
        move |_pre_render_task: &FD4TaskData| {
            let Ok(cam) =
                (unsafe { CSCamera::instance() }).map(|camera| camera.pers_cam_1.as_mut())
            else {
                return;
            };

            let forward = cam.forward();
            let position = &mut cam.matrix.3;
            position.0 += forward.0 * -camera_config.distance_multiplier;
            position.1 += forward.1 * -camera_config.distance_multiplier;
            position.2 += forward.2 * -camera_config.distance_multiplier;

            cam.fov = camera_config.field_of_view;
            cam.aspect_ratio = camera_config.aspect_ratio;
            cam.near_plane = camera_config.render_distance_start;
            cam.far_plane = camera_config.render_distance_end;

        },
        CSTaskGroupIndex::CameraStep,
    );
}

// Exposed for dll loaders, a.e ModEngine 3.
#[unsafe(no_mangle)]
unsafe extern "C" fn DllMain(_hmodule: usize, reason: u32) -> bool {
    if reason == 1 {
        std::thread::spawn(move || {
            // Wait for the game to initialize. Panic if it doesn't.
            wait_for_system_init(&Program::current(), Duration::MAX)
                .expect("Could not await system init.");

            adjust_camera_task();
        });
    }
    true
}
