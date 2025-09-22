use crate::types::{ScoredMove, Cell, PieceOffset};
use super::game_state::GameState;

impl GameState {
    /// STRATEGIC MOVE SELECTION: Advanced move selection when multiple good options exist
    pub fn select_strategic_move(&self, scored_moves: &[ScoredMove], distance_map: &[Vec<i32>], piece_offsets: &[PieceOffset]) -> ScoredMove {
        // Get game state context
        let my_territory = self.count_my_territory();
        let opponent_territory = self.count_opponent_territory();
        let total_cells = self.board_width * self.board_height;
        let game_progress = (my_territory + opponent_territory) as f32 / total_cells as f32;
        
        // Consider top 5 moves for strategic analysis
        let top_moves = &scored_moves[0..scored_moves.len().min(5)];
        
        // Early game (< 30% filled): Focus on expansion and positioning
        if game_progress < 0.3 {
            // Prefer moves that maximize future expansion potential
            let mut best_expansion_move = &top_moves[0];
            let mut best_expansion_score = 0;
            
            for move_candidate in top_moves {
                let mut expansion_potential = 0;
                
                // Calculate expansion potential for this move
                for offset in piece_offsets {
                    let board_x = move_candidate.x + offset.dx;
                    let board_y = move_candidate.y + offset.dy;
                    
                    if board_x >= 0 && board_x < self.board_width as i32 && 
                       board_y >= 0 && board_y < self.board_height as i32 {
                        let bx = board_x as usize;
                        let by = board_y as usize;
                        
                        if self.board[by][bx] == Cell::Empty {
                            expansion_potential += self.count_empty_neighbors(bx, by);
                        }
                    }
                }
                
                if expansion_potential > best_expansion_score {
                    best_expansion_score = expansion_potential;
                    best_expansion_move = move_candidate;
                }
            }
            
            return best_expansion_move.clone();
        }
        
        // Mid game (30-70% filled): Balance between expansion and blocking
        else if game_progress < 0.7 {
            // If we're behind, prioritize aggressive expansion
            if my_territory < opponent_territory {
                // Find move that captures the most territory
                let mut best_territory_move = &top_moves[0];
                let mut best_territory_count = 0;
                
                for move_candidate in top_moves {
                    let mut territory_captured = 0;
                    
                    for offset in piece_offsets {
                        let board_x = move_candidate.x + offset.dx;
                        let board_y = move_candidate.y + offset.dy;
                        
                        if board_x >= 0 && board_x < self.board_width as i32 && 
                           board_y >= 0 && board_y < self.board_height as i32 {
                            let bx = board_x as usize;
                            let by = board_y as usize;
                            
                            if self.board[by][bx] == Cell::Empty {
                                territory_captured += 1;
                            }
                        }
                    }
                    
                    if territory_captured > best_territory_count {
                        best_territory_count = territory_captured;
                        best_territory_move = move_candidate;
                    }
                }
                
                return best_territory_move.clone();
            }
            // If we're ahead or equal, balance expansion with blocking
            else {
                // Prefer moves that are close to opponent but still expand our territory
                let mut best_balanced_move = &top_moves[0];
                let mut best_balance_score = 0;
                
                for move_candidate in top_moves {
                    let mut balance_score = 0;
                    let mut territory_captured = 0;
                    let mut blocking_value = 0;
                    
                    for offset in piece_offsets {
                        let board_x = move_candidate.x + offset.dx;
                        let board_y = move_candidate.y + offset.dy;
                        
                        if board_x >= 0 && board_x < self.board_width as i32 && 
                           board_y >= 0 && board_y < self.board_height as i32 {
                            let bx = board_x as usize;
                            let by = board_y as usize;
                            
                            if self.board[by][bx] == Cell::Empty {
                                territory_captured += 1;
                                let opponent_distance = distance_map[by][bx];
                                if opponent_distance != -1 && opponent_distance <= 4 {
                                    blocking_value += 5 - opponent_distance;
                                }
                            }
                        }
                    }
                    
                    balance_score = territory_captured * 100 + blocking_value * 50;
                    
                    if balance_score > best_balance_score {
                        best_balance_score = balance_score;
                        best_balanced_move = move_candidate;
                    }
                }
                
                return best_balanced_move.clone();
            }
        }
        
        // End game (> 70% filled): Focus on maximum territory capture
        else {
            // In endgame, every empty cell matters - pick move with highest territory capture
            let mut best_endgame_move = &top_moves[0];
            let mut best_endgame_score = 0;
            
            for move_candidate in top_moves {
                let mut endgame_score = 0;
                
                for offset in piece_offsets {
                    let board_x = move_candidate.x + offset.dx;
                    let board_y = move_candidate.y + offset.dy;
                    
                    if board_x >= 0 && board_x < self.board_width as i32 && 
                       board_y >= 0 && board_y < self.board_height as i32 {
                        let bx = board_x as usize;
                        let by = board_y as usize;
                        
                        if self.board[by][bx] == Cell::Empty {
                            endgame_score += 1000; // High value for each cell
                            // Extra bonus for cells that deny opponent future moves
                            let empty_neighbors = self.count_empty_neighbors(bx, by);
                            endgame_score += empty_neighbors * 100;
                        }
                    }
                }
                
                if endgame_score > best_endgame_score {
                    best_endgame_score = endgame_score;
                    best_endgame_move = move_candidate;
                }
            }
            
            return best_endgame_move.clone();
        }
    }
}
