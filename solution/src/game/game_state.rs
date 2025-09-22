use crate::types::{Player, Cell, PieceCell, PieceOffset};
use std::collections::VecDeque;

/// Game state structure that holds all information about the current game state
/// and provides methods for parsing input, calculating legal moves, and determining
/// the optimal move using a sophisticated heuristic.
pub struct GameState {
    /// Current player (One or Two)
    pub player: Player,
    /// Width of the game board
    pub board_width: usize,
    /// Height of the game board
    pub board_height: usize,
    /// 2D representation of the board state
    pub board: Vec<Vec<Cell>>,
    /// Width of the current piece
    pub piece_width: usize,
    /// Height of the current piece
    pub piece_height: usize,
    /// 2D representation of the current piece
    pub piece: Vec<Vec<PieceCell>>,
    /// Symbols representing the current player's cells (uppercase, lowercase)
    pub my_symbols: (char, char),
    /// Symbols representing the opponent's cells (uppercase, lowercase)
    pub opponent_symbols: (char, char),
    /// Weight for the heat map component of the heuristic
    pub heat_weight: i32,
    /// Weight for the expansion component of the heuristic
    pub expansion_weight: i32,
    /// Weight for the blocking component of the heuristic
    pub blocking_weight: i32,
    /// Weight for the compactness component of the heuristic
    pub compactness_weight: i32,
}

impl GameState {
    /// Create a new game state
    pub fn new() -> Self {
        GameState {
            player: Player::One, // Default, will be updated
            board_width: 0,
            board_height: 0,
            board: Vec::new(),
            piece_width: 0,
            piece_height: 0,
            piece: Vec::new(),
            my_symbols: ('@', 'a'),      // Default for Player 1
            opponent_symbols: ('$', 's'), // Default for Player 1,
            // ULTRA AGGRESSIVE weights designed to WIN
            heat_weight: 50,      // MAXIMUM: Stay far from opponent
            expansion_weight: 30, // MAXIMUM: Prioritize expansion above all
            blocking_weight: 20,  // HIGH: Block opponent aggressively
            compactness_weight: -10, // SEVERE penalty: Force ultra-compact territory
        }
    }

    /// Parse player information from the input line
    pub fn parse_player(&mut self, line: &str) {
        // Extract player number from "$$$ exec p<number> : [<path>]"
        if let Some(player_char) = line.chars().nth("$$$ exec p".len()) {
            match player_char {
                '1' => {
                    self.player = Player::One;
                    self.my_symbols = ('@', 'a');
                    self.opponent_symbols = ('$', 's');
                    // Minimal logging
                    #[cfg(debug_assertions)]
                    eprintln!("I am Player 1");
                },
                '2' => {
                    self.player = Player::Two;
                    self.my_symbols = ('$', 's');
                    self.opponent_symbols = ('@', 'a');
                    // Minimal logging
                    #[cfg(debug_assertions)]
                    eprintln!("I am Player 2");
                },
                _ => eprintln!("Unknown player: {}", player_char),
            }
        }
    }

    /// Parse board dimensions and initialize the board
    pub fn parse_board_dimensions(&mut self, line: &str) -> Result<(), String> {
        // Extract dimensions from "Anfield <W> <H>:"
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            return Err(format!("Invalid board dimensions line: {}", line));
        }

        self.board_width = parts[1].parse::<usize>().map_err(|e| e.to_string())?;
        self.board_height = parts[2].trim_end_matches(':').parse::<usize>().map_err(|e| e.to_string())?;
        
        // Initialize the board with empty cells
        self.board = vec![vec![Cell::Empty; self.board_width]; self.board_height];
        
        // Minimal logging
        #[cfg(debug_assertions)]
        eprintln!("Board dimensions: {}x{}", self.board_width, self.board_height);
        Ok(())
    }

    pub fn parse_board_row(&mut self, line: &str, row_idx: usize) -> Result<(), String> {
        let line_content = if line.len() >= 4 { &line[4..] } else { line };
        if line_content.len() < self.board_width {
            return Err(format!("Board row too short: {}", line_content));
        }
        for (col_idx, ch) in line_content.chars().take(self.board_width).enumerate() {
            self.board[row_idx][col_idx] = match ch {
                '.' => Cell::Empty,
                '@' | 'a' => Cell::Player1,
                '$' | 's' => Cell::Player2,
                _ => return Err(format!("Unknown board cell: {}", ch)),
            };
        }
        Ok(())
    }

    /// Parse piece dimensions and initialize the piece
    pub fn parse_piece_dimensions(&mut self, line: &str) -> Result<(), String> {
        // Extract dimensions from "Piece <w> <h>:"
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            return Err(format!("Invalid piece dimensions line: {}", line));
        }

        self.piece_width = parts[1].parse::<usize>().map_err(|e| e.to_string())?;
        self.piece_height = parts[2].trim_end_matches(':').parse::<usize>().map_err(|e| e.to_string())?;
        
        // Initialize the piece with empty cells
        self.piece = vec![vec![PieceCell::Empty; self.piece_width]; self.piece_height];
        
        // Minimal logging
        #[cfg(debug_assertions)]
        eprintln!("Piece dimensions: {}x{}", self.piece_width, self.piece_height);
        Ok(())
    }

    /// Parse a piece row
    pub fn parse_piece_row(&mut self, line: &str, row_idx: usize) -> Result<(), String> {
        // Ensure the line has enough characters
        if line.len() < self.piece_width {
            return Err(format!("Piece row too short: {}", line));
        }

        // Parse each character in the row
        for (col_idx, ch) in line.chars().take(self.piece_width).enumerate() {
            self.piece[row_idx][col_idx] = match ch {
                '.' => PieceCell::Empty,
                '#' | 'O' | 'o' => PieceCell::Filled,
                _ => return Err(format!("Unknown piece cell: {}", ch)),
            };
        }

        Ok(())
    }

    /// Trim the piece to its minimal bounding box and PRECISELY return offsets
    pub fn trim_piece(&mut self) -> (Vec<PieceOffset>, i32, i32) {
        // Find bounds of filled cells within the original piece grid
        let mut min_row = self.piece_height;
        let mut max_row = 0;
        let mut min_col = self.piece_width;
        let mut max_col = 0;

        for r in 0..self.piece_height {
            for c in 0..self.piece_width {
                if self.piece[r][c] == PieceCell::Filled {
                    min_row = min_row.min(r);
                    max_row = max_row.max(r);
                    min_col = min_col.min(c);
                    max_col = max_col.max(c);
                }
            }
        }
        // no filled cells? (shouldn't happen) — return empty
        if min_row > max_row || min_col > max_col {
            return (Vec::new(), 0, 0);
        }

        let new_h = max_row - min_row + 1;
        let new_w = max_col - min_col + 1;
        let mut new_piece = vec![vec![PieceCell::Empty; new_w]; new_h];
        let mut offsets = Vec::new();

        for r in 0..new_h {
            for c in 0..new_w {
                let cell = self.piece[min_row + r][min_col + c];
                new_piece[r][c] = cell;
                if cell == PieceCell::Filled {
                    offsets.push(PieceOffset { dx: c as i32, dy: r as i32 });
                }
            }
        }

        // Update dimensions to the TRIMMED box
        self.piece = new_piece;
        self.piece_width = new_w;
        self.piece_height = new_h;

        // Return offsets to map trimmed → original
        (offsets, min_col as i32, min_row as i32)
    }

    /// Calculate distance map from opponent cells
    pub fn calculate_distance_map(&self) -> Vec<Vec<i32>> {
        let mut distance_map = vec![vec![-1; self.board_width]; self.board_height];
        let mut queue = VecDeque::new();
        
        // Initialize queue with opponent cells
        for y in 0..self.board_height {
            for x in 0..self.board_width {
                let cell = self.board[y][x];
                if (self.player == Player::One && cell == Cell::Player2) ||
                   (self.player == Player::Two && cell == Cell::Player1) {
                    distance_map[y][x] = 0;
                    queue.push_back((x, y));
                }
            }
        }
        
        // BFS to calculate distances
        let directions = [(0, 1), (1, 0), (0, -1), (-1, 0)];
        
        while let Some((x, y)) = queue.pop_front() {
            let current_dist = distance_map[y][x];
            
            for (dx, dy) in directions.iter() {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                
                if nx >= 0 && nx < self.board_width as i32 &&
                   ny >= 0 && ny < self.board_height as i32 {
                    let nx = nx as usize;
                    let ny = ny as usize;
                    
                    if distance_map[ny][nx] == -1 {
                        distance_map[ny][nx] = current_dist + 1;
                        queue.push_back((nx, ny));
                    }
                }
            }
        }
        
        distance_map
    }

    /// Debug function to print board section around our territory
    pub fn debug_print_board_section(&self) {
        eprintln!("DEBUG: Board section (showing first 10x10):");
        for y in 0..std::cmp::min(10, self.board_height) {
            eprint!("  ");
            for x in 0..std::cmp::min(10, self.board_width) {
                let cell = match self.board[y][x] {
                    Cell::Empty => '.',
                    Cell::Player1 => '@',
                    Cell::Player2 => '$',
                };
                eprint!("{}", cell);
            }
            eprintln!();
        }
    }
}
