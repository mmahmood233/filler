use crate::types::PieceOffset;
use super::game_state::GameState;
use std::io::{self, Write};

impl GameState {
    pub fn make_move(&self, piece_offsets: &[PieceOffset], trim_off_x: i32, trim_off_y: i32) {
        let distance_map = self.calculate_distance_map();
    
        // Find legal moves with offset-aware scan
        let mut legal_moves = self.find_legal_moves(piece_offsets, trim_off_x, trim_off_y);
    
        if legal_moves.is_empty() {
            legal_moves = self.emergency_move_search(piece_offsets, trim_off_x, trim_off_y);
        }
    
        if legal_moves.is_empty() {
            println!("0 0");
        } else {
            let mut best = legal_moves[0];
            let mut best_score = self.score_move(best.0, best.1, &distance_map, piece_offsets);
    
            for &(x, y) in &legal_moves {
                let s = self.score_move(x, y, &distance_map, piece_offsets);
                if s > best_score {
                    best_score = s;
                    best = (x, y);
                }
            }
    
            // Convert TRIMMED anchor → ORIGINAL top-left for the engine
            let out_x = best.0 - trim_off_x;
            let out_y = best.1 - trim_off_y;
            // Safety (should already be ≥0 and within board)
            let ox = out_x.max(0);
            let oy = out_y.max(0);
    
            println!("{} {}", ox, oy);
        }
        io::stdout().flush().unwrap();
    }
}
