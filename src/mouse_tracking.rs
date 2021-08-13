use bevy::prelude::*;

pub struct MainCamera;
pub struct MousePos(pub Vec2);

fn track_mouse(
    // need to get window dimensions
    wnds: Res<Windows>,
    // query to get camera transform
    q_camera: Query<&Transform, With<MainCamera>>,
    mut mouse_pos: ResMut<MousePos>,
) {
    // get the primary window
    let wnd = wnds.get_primary().unwrap();

    // check if the cursor is in the primary window
    if let Some(pos) = wnd.cursor_position() {
        // get the size of the window
        let size = Vec2::new(wnd.width() as f32, wnd.height() as f32);

        // the default orthographic projection is in pixels from the center;
        // just undo the translation
        let p = pos - size / 2.0;

        // assuming there is exactly one main camera entity, so this is OK
        let camera_transform = q_camera.single().unwrap();

        // apply the camera transform
        let pos_wld = camera_transform.compute_matrix() * p.extend(0.0).extend(1.0);
        *mouse_pos = MousePos(Vec2::new(pos_wld.x, pos_wld.y));
    }
}

pub struct MouseTrackingPlugin;

impl Plugin for MouseTrackingPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(MousePos(Vec2::new(0.0, 0.0)))
            .add_system(track_mouse.system());
    }
}
