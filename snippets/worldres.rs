#[derive(Resource)]
struct WorldRes {
    tiles: HashMap<Position, (Box<dyn Tile>, u32)>,
    terrain: HashMap<Position, TerrainTileType>,
    resources: HashMap<u32, u32>,
    world_seed: u32,
    tick_timer: Timer,
    tick_count: i32,
    actions: Vec<Action>,
}
