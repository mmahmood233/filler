// Game module - Core game logic and state management

pub mod game_state;
pub mod move_validation;
pub mod scoring;
pub mod strategy;
pub mod move_execution;

// Re-export the main GameState for easy access
pub use game_state::GameState;
