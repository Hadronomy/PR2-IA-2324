use bevy::math::*;
use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bevy_ecs_tilemap::helpers::square_grid::SquarePos;
use bevy_ecs_tilemap::{helpers::square_grid::neighbors::*, prelude::*};
use rand::seq::IteratorRandom;
use rand::Rng;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: String::from("A*"),
                        ..Default::default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
            // LogDiagnosticsPlugin::default(),
            FrameTimeDiagnosticsPlugin::default(),
        ))
        .init_resource::<CursorPos>()
        .add_event::<GenerateMazeEvent>()
        .add_plugins(TilemapPlugin)
        .add_systems(Startup, startup)
        .add_systems(Startup, init_maze.in_set(ConfigureMaze))
        .add_systems(
            Update,
            (
                update_cursor_pos,
                text_update_system,
                generate_maze.in_set(ConfigureMaze),
                regenerate_on_click.after(ConfigureMaze),
            ),
        )
        .run();
}

#[derive(SystemSet, Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct ConfigureMaze;

#[derive(Component)]
struct Maze;

#[derive(Event)]
struct GenerateMazeEvent;

#[derive(Resource)]
struct ActiveMaze(Entity);

#[derive(Component)]
struct FpsText;

#[derive(Component)]
struct CursorPosText;

#[derive(Component)]
enum TileType {
    Wall,
    Floor,
}

fn startup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    // let neighbor_positions =
    //      Neighbors::get_square_neighboring_positions(&TilePos { x: 0, y: 0 }, &map_size, true);
    // let neighbor_entities = neighbor_positions.entities(&tile_storage);

    // We can access tiles using:
    // assert!(tile_storage.get(&TilePos { x: 0, y: 0 }).is_some());
    // assert_eq!(neighbor_entities.iter().count(), 3); // Only 3 neighbors since negative is outside of map.

    // This changes some of our tiles by looking at neighbors.

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

fn init_maze(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut ev_generate_maze: EventWriter<GenerateMazeEvent>,
    #[cfg(all(not(feature = "atlas"), feature = "render"))] array_texture_loader: Res<
        ArrayTextureLoader,
    >,
) {
    let texture_handle: Handle<Image> = asset_server.load("tiles.png");

    let map_size = TilemapSize { x: 79, y: 45 };

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
                    texture_index: TileTextureIndex(5),
                    color: TileColor(Color::BLACK),
                    ..Default::default()
                })
                .insert(TileType::Wall)
                .id();
            tile_storage.set(&tile_pos, tile_entity);
        }
    }

    let tile_size = TilemapTileSize { x: 16.0, y: 16.0 };
    let grid_size = tile_size.into();
    let map_type = TilemapType::default();

    commands
        .entity(tilemap_entity)
        .insert(Maze)
        .insert(TilemapBundle {
            grid_size,
            map_type,
            size: map_size,
            storage: tile_storage,
            texture: TilemapTexture::Single(texture_handle),
            tile_size,
            transform: get_tilemap_center_transform(&map_size, &grid_size, &map_type, 0.0),
            ..Default::default()
        });

    commands.insert_resource(ActiveMaze(tilemap_entity));
    ev_generate_maze.send(GenerateMazeEvent {});
}

fn neighbors(tile_pos: &TilePos, map_size: &TilemapSize) -> Neighbors<TilePos> {
    let square_pos = SquarePos::from(tile_pos);
    let f = |direction: SquareDirection| {
        if direction.is_cardinal() {
            square_pos
                .offset(&direction)
                .offset(&direction)
                .as_tile_pos(map_size)
        } else {
            None
        }
    };
    Neighbors::from_directional_closure(f)
}

fn find_wall(current: &TilePos, next: &TilePos, map_size: &TilemapSize) -> Option<TilePos> {
    let current = SquarePos::from(current);
    let next = SquarePos::from(next);
    let offset = SquarePos {
        x: (current.x - next.x) / 2,
        y: (current.y - next.y) / 2,
    };
    let wall = current - offset;
    wall.as_tile_pos(map_size)
}

fn generate_maze(
    mut commands: Commands,
    storage_query: Query<&TileStorage, With<Maze>>,
    active_maze: Res<ActiveMaze>,
    mut ev_generate_maze: EventReader<GenerateMazeEvent>,
) {
    let active_maze = active_maze.0;

    for _ in ev_generate_maze.iter() {
        let tile_storage = storage_query.get(active_maze).unwrap();
        let grid_size = tile_storage.size;
        println!("grid_sizes: {:?}", grid_size);
        for x in 0..grid_size.x {
            for y in 0..grid_size.y {
                let tile_pos = TilePos { x, y };
                commands
                    .entity(tile_storage.get(&tile_pos).unwrap())
                    .insert(TileColor(Color::BLACK))
                    .insert(TileType::Wall);
            }
        }

        // Recursive backtracking maze generation algorithm.
        // https://en.wikipedia.org/wiki/Maze_generation_algorithm#Recursive_backtracker
        let mut rng = rand::thread_rng();
        let first = {
            let mut first = TilePos {
                x: rng.gen_range(1..grid_size.x - 1),
                y: rng.gen_range(1..grid_size.y - 1),
            };
            while first.x % 2 != 0 || first.y % 2 != 0 {
                first = TilePos {
                    x: rng.gen_range(1..grid_size.x),
                    y: rng.gen_range(1..grid_size.y),
                };
            }
            first
        };
        let mut stack: Vec<TilePos> = vec![first];
        let mut visited: Vec<TilePos> = vec![first];
        commands
            .entity(tile_storage.get(&first).unwrap())
            .insert(TileColor(Color::YELLOW))
            .insert(TileType::Floor);
        while let Some(current) = stack.pop() {
            let neighbors = neighbors(&current, &grid_size);
            let unvisited = neighbors.iter().filter(|n| !visited.contains(n));
            let next = unvisited.choose(&mut rng);

            if let Some(next) = next {
                stack.push(current);
                if let Some(wall) = find_wall(&current, next, &grid_size) {
                    commands
                        .entity(tile_storage.get(&wall).unwrap())
                        .insert(TileColor(Color::WHITE))
                        .insert(TileType::Floor);
                    commands
                        .entity(tile_storage.get(next).unwrap())
                        .insert(TileColor(Color::WHITE))
                        .insert(TileType::Floor);
                    stack.push(*next);
                }
                visited.push(*next);
            }
        }
    }
}

fn regenerate_on_click(
    maze_query: Query<(&TilemapSize, &TilemapGridSize, &TilemapType, &Transform), With<Maze>>,
    mouse_button: Res<Input<MouseButton>>,
    cursor_pos: Res<CursorPos>,
    active_maze: Res<ActiveMaze>,
    mut ev_generate_maze: EventWriter<GenerateMazeEvent>,
) {
    if mouse_button.just_pressed(MouseButton::Left) {
        let cursor_pos = cursor_pos.0;
        let (map_size, grid_size, map_type, map_transform) = maze_query.get(active_maze.0).unwrap();
        let cursor_in_map_pos: Vec2 = {
            // Extend the cursor_pos vec3 by 0.0 and 1.0
            let cursor_pos = Vec4::from((cursor_pos, 0.0, 1.0));
            let cursor_in_map_pos = map_transform.compute_matrix().inverse() * cursor_pos;
            cursor_in_map_pos.xy()
        };
        println!("cursor_pos: {:?}", cursor_pos);
        if let Some(tile_pos) =
            TilePos::from_world_pos(&cursor_in_map_pos, map_size, grid_size, map_type)
        {
            println!("tile_pos: {:?}", tile_pos);
            ev_generate_maze.send(GenerateMazeEvent {});
        }
    }
}

fn text_update_system(
    diagnostics: Res<DiagnosticsStore>,
    cursor_pos: Res<CursorPos>,
    mut fps_query: Query<&mut Text, With<FpsText>>,
    mut cursor_query: Query<&mut Text, (With<CursorPosText>, Without<FpsText>)>,
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
