use bevy::math::*;
use bevy::{prelude::*, diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin}, window::PrimaryWindow};
use bevy_ecs_tilemap::prelude::*;

use bevy_screen_diagnostics::{ScreenDiagnosticsPlugin, ScreenFrameDiagnosticsPlugin};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins.set(WindowPlugin{
            primary_window: Some(Window {
                title: String::from(
                    "Basic Example - Press Space to change Texture and H to show/hide tilemap.",
                ),
                ..Default::default()
            }),
            ..default()
        }).set(ImagePlugin::default_nearest()), 
            FrameTimeDiagnosticsPlugin
        ))
        .init_resource::<CursorPos>()
        .add_plugins(TilemapPlugin)
        .add_systems(Startup, startup.in_set(SpawnTilemapSet))
        .add_systems(Update, (update_cursor_pos, text_update_system))
        .run();
}

#[derive(SystemSet, Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct SpawnTilemapSet;

#[derive(Component)]
struct FpsText;

#[derive(Component)]
struct CursorPosText;

fn startup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    #[cfg(all(not(feature = "atlas"), feature = "render"))] array_texture_loader: Res<
        ArrayTextureLoader,
    >,
) {
    commands.spawn(Camera2dBundle::default());

    let texture_handle: Handle<Image> = asset_server.load("tiles.png");

    let map_size = TilemapSize { x: 64, y: 32 };

    // Create a tilemap entity a little early.
    // We want this entity early because we need to tell each tile which tilemap entity
    // it is associated with. This is done with the TilemapId component on each tile.
    // Eventually, we will insert the `TilemapBundle` bundle on the entity, which
    // will contain various necessary components, such as `TileStorage`.
    let tilemap_entity = commands.spawn_empty().id();

    // To begin creating the map we will need a `TileStorage` component.
    // This component is a grid of tile entities and is used to help keep track of individual
    // tiles in the world. If you have multiple layers of tiles you would have a tilemap entity
    // per layer, each with their own `TileStorage` component.
    let mut tile_storage = TileStorage::empty(map_size);

    // Spawn the elements of the tilemap.
    // Alternatively, you can use helpers::filling::fill_tilemap.
    for x in 0..map_size.x {
        for y in 0..map_size.y {
            let tile_pos = TilePos { x, y };
            let tile_entity = commands
                .spawn(TileBundle {
                    position: tile_pos,
                    tilemap_id: TilemapId(tilemap_entity),
                    ..Default::default()
                })
                .id();
            tile_storage.set(&tile_pos, tile_entity);
        }
    }

    let tile_size = TilemapTileSize { x: 16.0, y: 16.0 };
    let grid_size = tile_size.into();
    let map_type = TilemapType::default();

    commands.entity(tilemap_entity).insert(TilemapBundle {
        grid_size,
        map_type,
        size: map_size,
        storage: tile_storage,
        texture: TilemapTexture::Single(texture_handle),
        tile_size,
        transform: get_tilemap_center_transform(&map_size, &grid_size, &map_type, 0.0),
        ..Default::default()
    });

    commands.spawn((
        // Create a TextBundle that has a Text with a list of sections.
        TextBundle::from_sections([
            TextSection::new(
                "FPS: ",
                TextStyle {
                    // This font is loaded and will be used instead of the default font.
                    font_size: 25.0,
                    color: Color::WHITE,
                    ..default()
                },
            ),
            TextSection::from_style(TextStyle {
                font_size: 30.0,
                color: Color::GOLD,
                // If no font is specified, it will use the default font.
                ..default()
            }),
            TextSection::new(
                " ",
                TextStyle {
                    // This font is loaded and will be used instead of the default font.
                    font_size: 25.0,
                    color: Color::WHITE,
                    ..default()
                },
            ),
        ]),
        FpsText,
    ));

    commands.spawn((
        // Create a TextBundle that has a Text with a list of sections.
        TextBundle::from_sections([
            TextSection::new(
                "Cursor Pos: ",
                TextStyle {
                    // This font is loaded and will be used instead of the default font.
                    font_size: 25.0,
                    color: Color::WHITE,
                    ..default()
                },
            ),
            TextSection::from_style(TextStyle {
                font_size: 30.0,
                color: Color::GOLD,
                // If no font is specified, it will use the default font.
                ..default()
            }),
            
        ]),
        CursorPosText,
    ));

    // Add atlas to array texture loader so it's preprocessed before we need to use it.
    // Only used when the atlas feature is off and we are using array textures.
    #[cfg(all(not(feature = "atlas"), feature = "render"))]
    {
        array_texture_loader.add(TilemapArrayTexture {
            texture: TilemapTexture::Single(asset_server.load("tiles.png")),
            tile_size,
            ..Default::default()
        });
    }
}

fn text_update_system(
    diagnostics: Res<DiagnosticsStore>,
    cursor_pos: Res<CursorPos>,
    mut fps_query: Query<&mut Text, With<FpsText>>,
    mut cursor_query: Query<&mut Text, (With<CursorPosText>, Without<FpsText>)>
) {
    for mut text in &mut fps_query {
        if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(value) = fps.smoothed() {
                // Update the value of the second section
                text.sections[1].value = format!("{value:.1}");
            }
        }
    }

    for mut text in &mut cursor_query {
        let pos = cursor_pos.0;
        text.sections[1].value = format!("[{:.2}, {:.2}]", pos.x, pos.y);
    }
}

#[derive(Resource)]
pub struct CursorPos(Vec2);
impl Default for CursorPos {
    fn default() -> Self {
        // Initialize the cursor pos at some far away place. It will get updated
        // correctly when the cursor moves.
        Self(Vec2::new(-1000.0, -1000.0))
    }
}

// We need to keep the cursor position updated based on any `CursorMoved` events.
pub fn update_cursor_pos(
    camera_q: Query<(&GlobalTransform, &Camera)>,
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut cursor_pos: ResMut<CursorPos>,
) {
    for cursor_moved in cursor_moved_events.iter() {
        // To get the mouse's world position, we have to transform its window position by
        // any transforms on the camera. This is done by projecting the cursor position into
        // camera space (world space).
        for (cam_t, cam) in camera_q.iter() {
            if let Some(pos) = cam.viewport_to_world_2d(cam_t, cursor_moved.position) {
                *cursor_pos = CursorPos(pos);
            }
        }
    }
}

