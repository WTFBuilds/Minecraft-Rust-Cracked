//! MELON shared world model.
//!
//! Used by both the client (rendering) and the server (simulation), so the
//! block registry and chunk layout can never drift between the two sides.
//!
//! Layout: a [`World`] is a map of [`Chunk`]s (16×256×16) keyed by chunk
//! position; every [`Section`] is a flat 16×16×16 block array. Simple, dense,
//! and allocation-light — palettes and compression come later, once M3 tells
//! us what the real access patterns look like.

pub mod chunk;
pub mod collision;
pub mod flat;
pub mod raycast;

pub use chunk::{Chunk, Section, WireError, World, CHUNK_H, CHUNK_W, SECTION_SIZE};
pub use collision::{
    is_solid, step, GRAVITY, JUMP_VELOCITY, PLAYER_EYE_HEIGHT, PLAYER_HEIGHT, PLAYER_WIDTH,
    TERMINAL_VELOCITY, WALK_SPEED,
};
pub use flat::FlatGenerator;
pub use raycast::{raycast, RayHit};

/// Block IDs are the platform's own — no vanilla mapping table lives here.
/// Blocks the BedWars entity set actually touches for Phase 1–3.
pub mod block {
    pub const AIR: u16 = 0;
    pub const BEDROCK: u16 = 1;
    pub const STONE: u16 = 2;
    pub const DIRT: u16 = 3;
    pub const GRASS: u16 = 4;
    pub const WOOL_RED: u16 = 5;
    pub const WOOL_BLUE: u16 = 6;
    pub const WOOL_YELLOW: u16 = 7;
    pub const WOOL_GREEN: u16 = 8;
    pub const IRON_BLOCK: u16 = 9;
    pub const GOLD_BLOCK: u16 = 10;
    pub const DIAMOND_BLOCK: u16 = 11;
    pub const BED_HEAD: u16 = 12; // two-part bed, block 1 of 2
    pub const BED_FOOT: u16 = 13; // two-part bed, block 2 of 2

    /// Human-readable names for logging/debugging.
    pub fn name(id: u16) -> &'static str {
        match id {
            AIR => "air",
            BEDROCK => "bedrock",
            STONE => "stone",
            DIRT => "dirt",
            GRASS => "grass",
            WOOL_RED => "wool:red",
            WOOL_BLUE => "wool:blue",
            WOOL_YELLOW => "wool:yellow",
            WOOL_GREEN => "wool:green",
            IRON_BLOCK => "iron_block",
            GOLD_BLOCK => "gold_block",
            DIAMOND_BLOCK => "diamond_block",
            BED_HEAD => "bed:head",
            BED_FOOT => "bed:foot",
            _ => "unknown",
        }
    }
}
