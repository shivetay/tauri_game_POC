#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass, EguiTextureHandle};
use map_tests::app::MapApp;

#[derive(Resource)]
struct MapTexture {
    image_handle: Handle<Image>,
    display_size: egui::Vec2,
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "map_tests".into(),
                    resolution: (1280, 800).into(),
                    ..default()
                }),
                ..default()
            }),
        )
        .add_plugins(EguiPlugin::default())
        .insert_non_send(MapApp::new())
        .add_systems(Startup, setup)
        .add_systems(PreUpdate, sync_bevy_map_texture)
        .add_systems(EguiPrimaryContextPass, map_ui)
        .run();
}

fn placeholder_image() -> Image {
    Image::new(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands.spawn(Camera2d);

    let handle = images.add(placeholder_image());
    commands.spawn((
        Sprite::from_image(handle.clone()),
        Transform::from_scale(Vec3::ZERO),
        Visibility::Hidden,
    ));

    commands.insert_resource(MapTexture {
        image_handle: handle,
        display_size: egui::Vec2::ZERO,
    });
}

fn image_from_rgba(width: u32, height: u32, rgba: Vec<u8>) -> Image {
    Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
}

fn upload_map_rgba(
    app: &mut MapApp,
    images: &mut Assets<Image>,
    map_tex: &mut MapTexture,
) {
    if !app.session.texture_dirty {
        return;
    }
    let Some((w, h, rgba)) = app.compose_rgba() else {
        return;
    };
    let width = w as u32;
    let height = h as u32;
    let handle = map_tex.image_handle.clone();
    if let Some(mut image) = images.get_mut(&handle) {
        let same_size = image.width() == width && image.height() == height;
        if same_size {
            if let Some(data) = image.data.as_mut() {
                data.clone_from_slice(&rgba);
            }
        } else {
            *image = image_from_rgba(width, height, rgba);
        }
    }
    app.mark_clean();
    map_tex.display_size = egui::Vec2::new(w as f32, h as f32);
}

fn sync_bevy_map_texture(
    mut app: NonSendMut<MapApp>,
    mut images: ResMut<Assets<Image>>,
    mut map_tex: ResMut<MapTexture>,
) {
    upload_map_rgba(&mut app, &mut images, &mut map_tex);
}

fn map_ui(
    mut contexts: EguiContexts,
    mut app: NonSendMut<MapApp>,
    mut images: ResMut<Assets<Image>>,
    mut map_tex: ResMut<MapTexture>,
) -> Result {
    app.poll_results();
    app.tick_simulation();
    app.tick_perf();
    // Apply gen/interaction dirty flags in the same frame before painting.
    upload_map_rgba(&mut app, &mut images, &mut map_tex);

    let map_egui_id = contexts.add_image(EguiTextureHandle::Strong(map_tex.image_handle.clone()));
    let ctx = contexts.ctx_mut()?;
    let map_size = map_tex.display_size;
    let map_texture = if map_size.x > 0.0 && map_size.y > 0.0 {
        Some(map_egui_id)
    } else {
        None
    };

    let mut viewport_ui = egui::Ui::new(
        ctx.clone(),
        "viewport".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );

    app.draw_ui(&mut viewport_ui, map_texture, map_size);
    // Clicks during draw_ui may dirty the map; push pixels before the frame ends.
    upload_map_rgba(&mut app, &mut images, &mut map_tex);
    Ok(())
}
