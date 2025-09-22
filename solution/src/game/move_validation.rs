use crate::types::{Player, Cell, PieceOffset};
use super::game_state::GameState;

impl GameState {
    pub fn is_legal_move(&self, x: i32, y: i32, piece_offsets: &[PieceOffset]) -> bool {
        let mut own_overlaps = 0;
        let my = if self.player == Player::One { Cell::Player1 } else { Cell::Player2 };
        let op = if self.player == Player::One { Cell::Player2 } else { Cell::Player1 };

        for off in piece_offsets {
            let bx = x + off.dx;
            let by = y + off.dy;

            // MUST be fully inside board
            if bx < 0 || by < 0 || bx >= self.board_width as i32 || by >= self.board_height as i32 {
                return false;
            }
            match self.board[by as usize][bx as usize] {
                c if c == op => return false,
                c if c == my => own_overlaps += 1,
                _ => {}
            }
        }
        own_overlaps == 1
    }

    pub fn find_legal_moves(&self, piece_offsets: &[PieceOffset], trim_off_x: i32, trim_off_y: i32) -> Vec<(i32, i32)> {
        let mut legal = Vec::new();
        if self.piece_width > self.board_width || self.piece_height > self.board_height {
            return legal;
        }

        // x,y here are **TRIMMED** top-lefts. We later print (x - trim_off_x, y - trim_off_y).
        // To guarantee non-negative printed coords, start scan at those offsets.
        let start_x = trim_off_x;
        let start_y = trim_off_y;
        let end_x = self.board_width as i32 - self.piece_width as i32 + trim_off_x;
        let end_y = self.board_height as i32 - self.piece_height as i32 + trim_off_y;

        for y in start_y..=end_y {
            for x in start_x..=end_x {
                if self.is_legal_move(x, y, piece_offsets) {
                    legal.push((x, y));
                }
            }
        }
        legal
    }

    /// EMERGENCY MOVE SEARCH: Exhaustive search when normal search fails
    pub fn emergency_move_search(&self, piece_offsets: &[PieceOffset], trim_off_x: i32, trim_off_y: i32) -> Vec<(i32, i32)> {
        let mut moves = Vec::new();
        let start_x = trim_off_x;
        let start_y = trim_off_y;
        let end_x = self.board_width as i32 - self.piece_width as i32 + trim_off_x;
        let end_y = self.board_height as i32 - self.piece_height as i32 + trim_off_y;

        for y in start_y..=end_y {
            for x in start_x..=end_x {
                if self.is_legal_move(x, y, piece_offsets) {
                    moves.push((x, y));
                }
            }
        }
        moves
    }
}
