use std::io::{self, BufRead, Write};
use std::collections::VecDeque;
use std::cmp::Ordering;

/// Represents a player in the game
#[derive(Debug, Clone, Copy, PartialEq)]
enum Player {
    One,
    Two,
}

/// Represents a cell on the board
#[derive(Debug, Clone, Copy, PartialEq)]
enum Cell {
    Empty,
    Player1,
    Player2,
}

/// Represents a cell in a piece
#[derive(Debug, Clone, Copy, PartialEq)]
enum PieceCell {
    Empty,
    Filled,
}

/// Represents a scored move for evaluation
#[derive(Debug, Clone)]
struct ScoredMove {
    x: i32,
    y: i32,
    score: i32,
}

impl ScoredMove {
    fn new(x: i32, y: i32, score: i32) -> Self {
        ScoredMove { x, y, score }
    }
}

impl PartialEq for ScoredMove {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

impl Eq for ScoredMove {}

impl PartialOrd for ScoredMove {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScoredMove {
    fn cmp(&self, other: &Self) -> Ordering {
        // Primary: score (higher is better)
        let score_cmp = self.score.cmp(&other.score);
        if score_cmp != Ordering::Equal {
            return score_cmp;
        }
        
        // Tiebreaker 1: lower y
        let y_cmp = other.y.cmp(&self.y); // Reversed for lower y
        if y_cmp != Ordering::Equal {
            return y_cmp;
        }
        
        // Tiebreaker 2: lower x
        other.x.cmp(&self.x) // Reversed for lower x
    }
}

/// Represents a filled cell in the piece with its relative coordinates
#[derive(Debug, Clone, Copy)]
struct PieceOffset {
    dx: i32,
    dy: i32,
}

/// Game state structure that holds all information about the current game state
/// and provides methods for parsing input, calculating legal moves, and determining
/// the optimal move using a sophisticated heuristic.
struct GameState {
    /// Current player (One or Two)
    player: Player,
    /// Width of the game board
    board_width: usize,
    /// Height of the game board
    board_height: usize,
    /// 2D representation of the board state
    board: Vec<Vec<Cell>>,
    /// Width of the current piece
    piece_width: usize,
    /// Height of the current piece
    piece_height: usize,
    /// 2D representation of the current piece
    piece: Vec<Vec<PieceCell>>,
    /// Symbols representing the current player's cells (uppercase, lowercase)
    my_symbols: (char, char),
    /// Symbols representing the opponent's cells (uppercase, lowercase)
    opponent_symbols: (char, char),
    /// Weight for the heat map component of the heuristic
    heat_weight: i32,
    /// Weight for the expansion component of the heuristic
    expansion_weight: i32,
    /// Weight for the blocking component of the heuristic
    blocking_weight: i32,
    /// Weight for the compactness component of the heuristic
    compactness_weight: i32,
}

impl GameState {
    /// Create a new game state
    fn new() -> Self {
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
    fn parse_player(&mut self, line: &str) {
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
    fn parse_board_dimensions(&mut self, line: &str) -> Result<(), String> {
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

    /// Parse a board row
    fn parse_board_row(&mut self, line: &str, row_idx: usize) -> Result<(), String> {
        // Skip the row index if present
        let line_content = if line.contains(' ') {
            line.split_whitespace().nth(1).unwrap_or(line)
        } else {
            line
        };

        // Ensure the line has enough characters
        if line_content.len() < self.board_width {
            return Err(format!("Board row too short: {}", line_content));
        }

        // Parse each character in the row
        for (col_idx, ch) in line_content.chars().take(self.board_width).enumerate() {
            self.board[row_idx][col_idx] = match ch {
                '.' => Cell::Empty,
                '@' | 'a' => Cell::Player1,  // @ and a are always Player 1
                '$' | 's' => Cell::Player2,  // $ and s are always Player 2
                _ => return Err(format!("Unknown board cell: {}", ch)),
            };
        }

        Ok(())
    }

    /// Parse piece dimensions and initialize the piece
    fn parse_piece_dimensions(&mut self, line: &str) -> Result<(), String> {
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
    fn parse_piece_row(&mut self, line: &str, row_idx: usize) -> Result<(), String> {
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

    /// Trim the piece to its minimal bounding box and precompute offsets
    fn trim_piece(&mut self) -> Vec<PieceOffset> {
        // Find the bounds of the filled cells
        let mut min_row = self.piece_height;
        let mut max_row = 0;
        let mut min_col = self.piece_width;
        let mut max_col = 0;

        // Find the bounds
        for row in 0..self.piece_height {
            for col in 0..self.piece_width {
                if self.piece[row][col] == PieceCell::Filled {
                    min_row = min_row.min(row);
                    max_row = max_row.max(row);
                    min_col = min_col.min(col);
                    max_col = max_col.max(col);
                }
            }
        }

        // If no filled cells found, return empty offsets
        if min_row > max_row || min_col > max_col {
            return Vec::new();
        }

        // Create a new trimmed piece
        let new_height = max_row - min_row + 1;
        let new_width = max_col - min_col + 1;
        let mut new_piece = vec![vec![PieceCell::Empty; new_width]; new_height];
        let mut offsets = Vec::new();

        // Copy the relevant part and collect offsets
        for row in 0..new_height {
            for col in 0..new_width {
                let orig_cell = self.piece[min_row + row][min_col + col];
                new_piece[row][col] = orig_cell;
                
                if orig_cell == PieceCell::Filled {
                    offsets.push(PieceOffset {
                        dx: col as i32,
                        dy: row as i32,
                    });
                }
            }
        }

        // Update the piece
        self.piece = new_piece;
        self.piece_width = new_width;
        self.piece_height = new_height;

        // Minimal logging
        #[cfg(debug_assertions)]
        eprintln!("Trimmed piece to {}x{} with {} filled cells", new_width, new_height, offsets.len());
        offsets
    }

    /// FAST and RELIABLE move legality check
    fn is_legal_move(&self, x: i32, y: i32, piece_offsets: &[PieceOffset]) -> bool {
        let mut own_overlaps = 0;
        let mut opponent_overlaps = 0;
        let mut valid_placements = 0;
        
        let my_cell = if self.player == Player::One { Cell::Player1 } else { Cell::Player2 };
        let opponent_cell = if self.player == Player::One { Cell::Player2 } else { Cell::Player1 };
        
        // Check each piece cell
        for offset in piece_offsets {
            let board_x = x + offset.dx;
            let board_y = y + offset.dy;
            
            // Skip out of bounds - this is OK, pieces can hang off the edge
            if board_x < 0 || board_x >= self.board_width as i32 || 
               board_y < 0 || board_y >= self.board_height as i32 {
                continue;
            }
            
            let bx = board_x as usize;
            let by = board_y as usize;
            let cell = self.board[by][bx];
            
            valid_placements += 1;
            
            // Count overlaps
            if cell == my_cell {
                own_overlaps += 1;
            } else if cell == opponent_cell {
                opponent_overlaps += 1;
            }
        }
        
        // Legal move: at least one valid placement, exactly one own overlap, no opponent overlaps
        valid_placements > 0 && own_overlaps == 1 && opponent_overlaps == 0
    }
    
    /// FAST and RELIABLE move finding - no debug output for speed
    fn find_legal_moves(&self, piece_offsets: &[PieceOffset]) -> Vec<(i32, i32)> {
        let mut legal_moves = Vec::new();
        
        // Search efficiently around the board
        let search_margin = 10;
        let start_x = -search_margin;
        let end_x = self.board_width as i32 + search_margin;
        let start_y = -search_margin;
        let end_y = self.board_height as i32 + search_margin;
        
        for y in start_y..end_y {
            for x in start_x..end_x {
                if self.is_legal_move(x, y, piece_offsets) {
                    legal_moves.push((x, y));
                }
            }
        }
        
        legal_moves
    }
    
    /// Debug function to print board section around our territory
    fn debug_print_board_section(&self) {
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

    /// Calculate distance map from opponent cells
    fn calculate_distance_map(&self) -> Vec<Vec<i32>> {
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
    
    /// Count empty neighbors of a cell
    fn count_empty_neighbors(&self, x: usize, y: usize) -> i32 {
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
    fn count_my_neighbors(&self, x: usize, y: usize) -> i32 {
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
    fn count_total_empty_cells(&self) -> i32 {
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
    fn count_my_territory(&self) -> i32 {
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
    fn count_opponent_territory(&self) -> i32 {
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

    /// SIMPLE WINNING STRATEGY: Based on breakthrough 9-point approach - maximize legal moves and territory!
    /// Focus on fundamentals: territory capture and maintaining maximum future move options
    fn score_move(&self, x: i32, y: i32, _distance_map: &[Vec<i32>], piece_offsets: &[PieceOffset]) -> i32 {
        let my_cell = if self.player == Player::One { Cell::Player1 } else { Cell::Player2 };
        
        let mut score = 0;
        let mut cells_captured = 0;
        
        // Simple but effective: count territory we can capture
        for offset in piece_offsets {
            let board_x = x + offset.dx;
            let board_y = y + offset.dy;
            
            // Skip out of bounds
            if board_x < 0 || board_x >= self.board_width as i32 || 
               board_y < 0 || board_y >= self.board_height as i32 {
                continue;
            }
            
            let bx = board_x as usize;
            let by = board_y as usize;
            
            // Skip cells we already own
            if self.board[by][bx] == my_cell {
                continue;
            }
            
            // Count empty cells we can capture
            if self.board[by][bx] == Cell::Empty {
                cells_captured += 1;
                
                // CONNECTIVITY-FIRST STRATEGY: Prioritize moves that keep us connected to large empty areas
                // This prevents the "0 0" fallbacks that are killing us
                score += 1000; // Base territory value
                
                // CRITICAL: Heavily weight moves that connect to large empty regions
                let empty_neighbors = self.count_empty_neighbors(bx, by);
                score += empty_neighbors * empty_neighbors * 1000; // Quadratic bonus for connectivity
            }
        }
        
        // HYBRID STRATEGY: Combine connectivity and piece size for maximum effectiveness
        if cells_captured > 0 {
            // CONNECTIVITY + PIECE SIZE: Prevent "0 0" fallbacks while maximizing territory
            // The connectivity scoring above prevents early game death
            // Now add piece size bonus for efficient territory capture
            score += cells_captured * cells_captured * 5000; // Quadratic piece size bonus
            
            return score;
        } else {
            return 0; // Invalid move
        }
    }
    
    /// ULTRA-COMPETITIVE MOVE GENERATION: Strategic move selection for maximum winning potential!
    fn make_move(&self, piece_offsets: &[PieceOffset]) {
        // Calculate distance map from opponent territory
        let distance_map = self.calculate_distance_map();
        
        // Find all legal moves
        let mut legal_moves = self.find_legal_moves(piece_offsets);
        
        // If no moves found, try EMERGENCY SEARCH with larger area
        if legal_moves.is_empty() {
            eprintln!("Normal search found 0 moves, trying emergency search...");
            legal_moves = self.emergency_move_search(piece_offsets);
            eprintln!("Emergency search found {} moves", legal_moves.len());
        }
        
        if legal_moves.is_empty() {
            // DEBUG: Log when no moves found
            eprintln!("ERROR: No legal moves found even after emergency search!");
            println!("0 0");
        } else {
            // DEBUG: Log move count and game state
            let my_territory = self.count_my_territory();
            let opponent_territory = self.count_opponent_territory();
            eprintln!("Found {} legal moves. Territory: Us={}, Opponent={}", 
                     legal_moves.len(), my_territory, opponent_territory);
            
            // Find the best move by scoring all legal moves
            let mut best_move = legal_moves[0];
            let mut best_score = self.score_move(best_move.0, best_move.1, &distance_map, piece_offsets);
            
            for &(x, y) in &legal_moves {
                let score = self.score_move(x, y, &distance_map, piece_offsets);
                if score > best_score {
                    best_score = score;
                    best_move = (x, y);
                }
            }
            
            eprintln!("DEBUG: Best move ({}, {}) with score {}", best_move.0, best_move.1, best_score);
            println!("{} {}", best_move.0, best_move.1);
        }
        
        io::stdout().flush().unwrap();
    }
    
    /// EMERGENCY MOVE SEARCH: Exhaustive search when normal search fails
    fn emergency_move_search(&self, piece_offsets: &[PieceOffset]) -> Vec<(i32, i32)> {
        let mut moves = Vec::new();
        
        // Exhaustive search across entire board
        for y in 0..self.board_height as i32 {
            for x in 0..self.board_width as i32 {
                if self.is_legal_move(x, y, piece_offsets) {
                    moves.push((x, y));
                }
            }
        }
        
        moves
    }
    
    /// STRATEGIC MOVE SELECTION: Advanced move selection when multiple good options exist
    fn select_strategic_move(&self, scored_moves: &[ScoredMove], distance_map: &[Vec<i32>], piece_offsets: &[PieceOffset]) -> ScoredMove {
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

/// Main function that handles the game loop for the Filler bot
/// 
/// This function implements the main game loop that:
/// 1. Reads and parses the player assignment (p1 or p2)
/// 2. Reads and parses the board dimensions and state
/// 3. Reads and parses the piece to be placed
/// 4. Calculates the optimal move using the heuristic
/// 5. Outputs the chosen move coordinates
/// 
/// The bot uses a sophisticated heuristic that considers:
/// - Heat map (distance from opponent)
/// - Expansion potential (empty neighbors)
/// - Blocking effectiveness (proximity to opponent)
/// - Compactness (adjacency to own territory)
fn main() {
    // Initialize game state
    let mut game_state = GameState::new();
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    
    // Process input until EOF
    while let Some(line_result) = lines.next() {
        // Handle potential I/O errors
        let line = match line_result {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Error reading line: {}", e);
                // Output a safe default move on error
                println!("0 0");
                io::stdout().flush().unwrap();
                continue;
            }
        };
        
        // Process the input line
        
        if line.starts_with("$$$ exec p") {
            game_state.parse_player(&line);
        }
        // Parse board dimensions
        else if line.starts_with("Anfield ") {
            if let Err(e) = game_state.parse_board_dimensions(&line) {
                eprintln!("Error parsing board dimensions: {}", e);
                // Output a safe default move on error
                println!("0 0");
                io::stdout().flush().unwrap();
                continue;
            }
            
            // Skip the column header line (e.g., "    01234567890123456789")
            match lines.next() {
                Some(Ok(_header_line)) => {
                    // Header line skipped
                },
                _ => {
                    eprintln!("Error: Expected header line after board dimensions");
                    println!("0 0");
                    io::stdout().flush().unwrap();
                    continue;
                }
            }
            
            // Read board rows
            let mut board_error = false;
            for row_idx in 0..game_state.board_height {
                match lines.next() {
                    Some(Ok(board_line)) => {
                        if let Err(e) = game_state.parse_board_row(&board_line, row_idx) {
                            eprintln!("Error parsing board row {}: {}", row_idx, e);
                            board_error = true;
                        }
                    },
                    _ => {
                        eprintln!("Unexpected end of input while reading board");
                        board_error = true;
                        break;
                    }
                }
            }
            
            if board_error {
                // Output a safe default move on error
                println!("0 0");
                io::stdout().flush().unwrap();
                continue;
            }
            
            // Look for piece info
            let mut found_piece = false;
            while let Some(line_result) = lines.next() {
                let next_line = match line_result {
                    Ok(l) => l,
                    Err(e) => {
                        eprintln!("Error reading line: {}", e);
                        break;
                    }
                };
                
                if next_line.starts_with("Piece ") {
                    found_piece = true;
                    
                    if let Err(e) = game_state.parse_piece_dimensions(&next_line) {
                        eprintln!("Error parsing piece dimensions: {}", e);
                        break;
                    }
                    
                    // Read piece rows
                    let mut piece_error = false;
                    for row_idx in 0..game_state.piece_height {
                        match lines.next() {
                            Some(Ok(piece_line)) => {
                                if let Err(e) = game_state.parse_piece_row(&piece_line, row_idx) {
                                    eprintln!("Error parsing piece row {}: {}", row_idx, e);
                                    piece_error = true;
                                }
                            },
                            _ => {
                                eprintln!("Unexpected end of input while reading piece");
                                piece_error = true;
                                break;
                            }
                        }
                    }
                    
                    if piece_error {
                        // Output a safe default move on error
                        println!("0 0");
                        io::stdout().flush().unwrap();
                        break;
                    }
                    
                    // Trim the piece to its minimal bounding box and get precomputed offsets
                    let piece_offsets = game_state.trim_piece();
                    
                    // Make a move using the precomputed offsets
                    game_state.make_move(&piece_offsets);
                    break;
                }
            }
            
            if !found_piece {
                // If we didn't find a piece, output a safe default move
                println!("0 0");
                io::stdout().flush().unwrap();
            }
        }
    }
}
