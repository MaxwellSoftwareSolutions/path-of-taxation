// =============================================================
// Path of Taxation -- Game Constants
// Values that are fundamental to the game's architecture.
// Tunable balance values belong in content/feel/*.ron instead.
// =============================================================

// --- Rendering ---
pub const INTERNAL_WIDTH: f32 = 480.0;
pub const INTERNAL_HEIGHT: f32 = 270.0;
pub const PIXEL_SCALE: f32 = 4.0;
pub const WINDOW_WIDTH: f32 = INTERNAL_WIDTH * PIXEL_SCALE;
pub const WINDOW_HEIGHT: f32 = INTERNAL_HEIGHT * PIXEL_SCALE;

// --- Isometric ---
pub const TILE_WIDTH: f32 = 64.0;
pub const TILE_HEIGHT: f32 = 32.0;
pub const ISO_RATIO: f32 = 2.0; // 2:1 dimetric

// --- Server ---
pub const SERVER_TICK_RATE_MS: u64 = 50; // 20Hz
pub const CLIENT_FRAME_RATE: f32 = 60.0;

// --- Combat (structural, not balance) ---
pub const MAX_ABILITY_SLOTS: usize = 6;
pub const MAX_ACTIVE_ATTACKERS: usize = 4;
pub const MAX_LEGISLATIVE_AMENDMENTS_PER_RUN: usize = 2;

// --- Runs ---
pub const MIN_ROOMS_PER_RUN: u32 = 5;
pub const MAX_ROOMS_PER_RUN: u32 = 7;
pub const ROOM_CHOICES_SHOWN: usize = 3;

// --- Meta ---
pub const MAX_NPC_RELATIONSHIP_RANK: u32 = 5;
