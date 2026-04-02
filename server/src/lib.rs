// Path of Taxation -- SpacetimeDB server module
//
// This WASM module defines all server-authoritative tables and reducers
// for the game. Tables store persistent and transient game state; reducers
// handle client actions and the 20Hz server tick loop.

pub mod tables;
pub mod reducers;
