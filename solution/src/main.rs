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

    fn parse_board_row(&mut self, line: &str, row_idx: usize) -> Result<(), String> {
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

/// Trim the piece to its minimal bounding box and PRECISELY return offsets
fn trim_piece(&mut self) -> (Vec<PieceOffset>, i32, i32) {
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


fn is_legal_move(&self, x: i32, y: i32, piece_offsets: &[PieceOffset]) -> bool {
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

    
    
    
fn find_legal_moves(&self, piece_offsets: &[PieceOffset], trim_off_x: i32, trim_off_y: i32) -> Vec<(i32, i32)> {
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

    fn score_move(&self, x: i32, y: i32, dist: &[Vec<i32>], piece_offsets: &[PieceOffset]) -> i32 {
        let my  = if self.player == Player::One { Cell::Player1 } else { Cell::Player2 };
        let op  = if self.player == Player::One { Cell::Player2 } else { Cell::Player1 };
    
        // game phase
        let my_t = self.count_my_territory();
        let op_t = self.count_opponent_territory();
        let occ  = my_t + op_t;
        let total = (self.board_width * self.board_height) as i32;
        let p = occ as f32 / total as f32;
    
        // features
        let mut new_cells = 0;   // empty cells we’ll claim
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

    fn get_my_territory_positions(&self) -> Vec<(usize, usize)> {
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
    
    
    
    
    fn make_move(&self, piece_offsets: &[PieceOffset], trim_off_x: i32, trim_off_y: i32) {
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
    
    
    /// EMERGENCY MOVE SEARCH: Exhaustive search when normal search fails
    fn emergency_move_search(&self, piece_offsets: &[PieceOffset], trim_off_x: i32, trim_off_y: i32) -> Vec<(i32, i32)> {
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
                    let (piece_offsets, trim_off_x, trim_off_y) = game_state.trim_piece();
                    
                    // Make a move using the precomputed offsets
                    game_state.make_move(&piece_offsets, trim_off_x, trim_off_y);
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
