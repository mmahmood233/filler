use crate::types::{Player, Cell, PieceOffset};
use super::game_state::GameState;

impl GameState {
    /// Count empty neighbors of a cell
    pub fn count_empty_neighbors(&self, x: usize, y: usize) -> i32 {
        let mut count = 0;
        let directions = [(0, 1), (1, 0), (0, -1), (-1, 0)];
        
        for (dx, dy) in directions.iter() {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            
            if nx >= 0 && nx < self.board_width as i32 &&
               ny >= 0 && ny < self.board_height as i32 {
                let nx = nx as usize;
                let ny = ny as usize;
                
                if self.board[ny][nx] == Cell::Empty {
                    count += 1;
                }
            }
        }
        
        count
    }
    
    /// Count adjacent cells that belong to us (for connectivity scoring)
    pub fn count_my_neighbors(&self, x: usize, y: usize) -> i32 {
        let mut count = 0;
        let my_cell = if self.player == Player::One { Cell::Player1 } else { Cell::Player2 };
        
        // Check all 4 adjacent cells
        let directions = [(0, 1), (1, 0), (0, -1), (-1, 0)];
        for (dx, dy) in directions.iter() {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            
            if nx >= 0 && nx < self.board_width as i32 && ny >= 0 && ny < self.board_height as i32 {
                if self.board[ny as usize][nx as usize] == my_cell {
                    count += 1;
                }
            }
        }
        
        count
    }
    
    /// Count total empty cells on the board (for endgame detection)
    pub fn count_total_empty_cells(&self) -> i32 {
        let mut count = 0;
        for row in &self.board {
            for cell in row {
                if *cell == Cell::Empty {
                    count += 1;
                }
            }
        }
        count
    }
    
    /// Count our total territory size
    pub fn count_my_territory(&self) -> i32 {
        let mut count = 0;
        let my_cell = if self.player == Player::One { Cell::Player1 } else { Cell::Player2 };
        for row in &self.board {
            for cell in row {
                if *cell == my_cell {
                    count += 1;
                }
            }
        }
        count
    }
    
    /// Count opponent's total territory size
    pub fn count_opponent_territory(&self) -> i32 {
        let mut count = 0;
        let opponent_cell = if self.player == Player::One { Cell::Player2 } else { Cell::Player1 };
        for row in &self.board {
            for cell in row {
                if *cell == opponent_cell {
                    count += 1;
                }
            }
        }
        count
    }

    pub fn score_move(&self, x: i32, y: i32, dist: &[Vec<i32>], piece_offsets: &[PieceOffset]) -> i32 {
        let my  = if self.player == Player::One { Cell::Player1 } else { Cell::Player2 };
        let op  = if self.player == Player::One { Cell::Player2 } else { Cell::Player1 };
    
        // game phase
        let my_t = self.count_my_territory();
        let op_t = self.count_opponent_territory();
        let occ  = my_t + op_t;
        let total = (self.board_width * self.board_height) as i32;
        let p = occ as f32 / total as f32;
    
        // features
        let mut new_cells = 0;   // empty cells we'll claim
        let mut liberties = 0;   // empty-neighbor count around claimed cells
        let mut heat_sum  = 0;   // sum of distance to opponent (smaller is more pressure)
        let mut adj_op    = 0;   // adjacency to opponent (blocking)
    
        for off in piece_offsets {
            let bx = (x + off.dx) as usize;
            let by = (y + off.dy) as usize;
    
            if self.board[by][bx] == Cell::Empty {
                new_cells += 1;
    
                liberties += self.count_empty_neighbors(bx, by);
    
                let d = dist[by][bx];
                if d > 0 { heat_sum += d; }
    
                for (dx,dy) in [(1,0),(-1,0),(0,1),(0,-1)] {
                    let nx = bx as i32 + dx;
                    let ny = by as i32 + dy;
                    if nx >= 0 && ny >= 0 && nx < self.board_width as i32 && ny < self.board_height as i32 {
                        if self.board[ny as usize][nx as usize] == op { adj_op += 1; }
                    }
                }
            }
        }
        if new_cells == 0 { return i32::MIN/4; }
    
        // weights by phase
        let (w_new, w_lib, w_adj, w_heat) = if p < 0.35 {
            (150, 40, 15, -5)     // early: expansion + options
        } else if p < 0.70 {
            (120, 20, 35, -15)    // mid: balance with pressure
        } else {
            (200, 10, 50, -25)    // late: grab cells & choke
        };
    
        let mut s = 0;
        s += new_cells * w_new;
        s += liberties * w_lib;
        s += adj_op * w_adj;
        s += heat_sum * w_heat; // negative weight prefers smaller sums (closer to foe)
    
        // if behind, add aggression
        if my_t < op_t { s += adj_op * 20; }
    
        // small connectivity bias (stay near our mass)
        let mut best_conn = i32::MAX;
        for (tx, ty) in self.get_my_territory_positions() {
            let d = (x - tx as i32).abs() + (y - ty as i32).abs();
            if d < best_conn { best_conn = d; }
        }
        if my_t > 0 { s += (10 - best_conn.min(10)) * 10; }
    
        s
    }

    pub fn get_my_territory_positions(&self) -> Vec<(usize, usize)> {
        let my_cell = if self.player == Player::One { Cell::Player1 } else { Cell::Player2 };
        let mut pos = Vec::new();
        for y in 0..self.board_height {
            for x in 0..self.board_width {
                if self.board[y][x] == my_cell {
                    pos.push((x, y));
                }
            }
        }
        pos
    }
}
