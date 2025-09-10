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
            // Default weights based on the prompt
            heat_weight: 10,
            expansion_weight: 3,
            blocking_weight: 2,
            compactness_weight: -1,
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
                '@' | 'a' => if self.player == Player::One { Cell::Player1 } else { Cell::Player2 },
                '$' | 's' => if self.player == Player::One { Cell::Player2 } else { Cell::Player1 },
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

    /// Check if placing the piece at position (x, y) is legal using precomputed offsets
    fn is_legal_move(&self, x: i32, y: i32, piece_offsets: &[PieceOffset]) -> bool {
        let mut overlap_count = 0;
        
        // Check each filled cell of the piece using precomputed offsets
        for offset in piece_offsets {
            // Calculate board coordinates
            let board_x = x + offset.dx;
            let board_y = y + offset.dy;
            
            // Check if out of bounds
            if board_x < 0 || board_x >= self.board_width as i32 || 
               board_y < 0 || board_y >= self.board_height as i32 {
                return false;
            }
            
            // Convert to usize for indexing
            let bx = board_x as usize;
            let by = board_y as usize;
            
            // Check if overlapping with opponent
            if self.player == Player::One && self.board[by][bx] == Cell::Player2 ||
               self.player == Player::Two && self.board[by][bx] == Cell::Player1 {
                return false;
            }
            
            // Count overlaps with own territory
            if self.player == Player::One && self.board[by][bx] == Cell::Player1 ||
               self.player == Player::Two && self.board[by][bx] == Cell::Player2 {
                overlap_count += 1;
            }
        }
        
        // Legal if exactly one overlap with own territory
        overlap_count == 1
    }
    
    /// Find all legal moves using precomputed offsets
    fn find_legal_moves(&self, piece_offsets: &[PieceOffset]) -> Vec<(i32, i32)> {
        let mut legal_moves = Vec::new();
        
        // Try all possible placements
        for y in -(self.piece_height as i32)..self.board_height as i32 {
            for x in -(self.piece_width as i32)..self.board_width as i32 {
                if self.is_legal_move(x, y, piece_offsets) {
                    legal_moves.push((x, y));
                }
            }
        }
        
        // Minimal logging
        #[cfg(debug_assertions)]
        eprintln!("Found {} legal moves", legal_moves.len());
        legal_moves
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
    
    /// Count adjacent cells that are part of my territory
    fn count_my_adjacencies(&self, x: usize, y: usize) -> i32 {
        let mut count = 0;
        let directions = [(0, 1), (1, 0), (0, -1), (-1, 0)];
        let my_cell = if self.player == Player::One { Cell::Player1 } else { Cell::Player2 };
        
        for (dx, dy) in directions.iter() {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            
            if nx >= 0 && nx < self.board_width as i32 &&
               ny >= 0 && ny < self.board_height as i32 {
                let nx = nx as usize;
                let ny = ny as usize;
                
                if self.board[ny][nx] == my_cell {
                    count += 1;
                }
            }
        }
        
        count
    }

    /// Score a move based on the improved heuristic using precomputed offsets
    fn score_move(&self, x: i32, y: i32, distance_map: &[Vec<i32>], piece_offsets: &[PieceOffset]) -> i32 {
        let mut heat_score = 0;
        let mut expansion_score = 0;
        let mut blocking_score = 0;
        let mut compactness_score = 0;
        let mut new_cells = 0;
        
        // Check each filled cell of the piece using precomputed offsets
        for offset in piece_offsets {
            // Calculate board coordinates
            let board_x = x + offset.dx;
            let board_y = y + offset.dy;
            
            // Skip out of bounds or cells that overlap with existing territory
            if board_x < 0 || board_x >= self.board_width as i32 || 
               board_y < 0 || board_y >= self.board_height as i32 {
                continue;
            }
            
            let bx = board_x as usize;
            let by = board_y as usize;
            
            // Skip cells that overlap with existing territory
            if (self.player == Player::One && self.board[by][bx] == Cell::Player1) ||
               (self.player == Player::Two && self.board[by][bx] == Cell::Player2) {
                continue;
            }
            
            // Heat: distance from opponent (farther is better)
            if distance_map[by][bx] != -1 {
                heat_score += distance_map[by][bx];
            } else {
                // If no distance calculated, use a high value
                heat_score += self.board_width as i32 + self.board_height as i32;
            }
            
            // Expansion: count empty neighbors (more is better)
            expansion_score += self.count_empty_neighbors(bx, by);
            
            // Blocking: negative distance to opponent (closer is better for blocking)
            if distance_map[by][bx] != -1 {
                blocking_score += 10 - distance_map[by][bx].min(10); // Cap at 10, invert so closer is higher
            }
            
            // Compactness: count adjacencies to own territory (more is better)
            compactness_score += self.count_my_adjacencies(bx, by);
            
            new_cells += 1;
        }
        
        
        // Add a small bonus for the number of new cells covered
        final_score + new_cells
    }
    
    /// Find the best move using the improved heuristic approach with precomputed offsets
    fn find_best_move(&self, piece_offsets: &[PieceOffset]) -> Option<(i32, i32)> {
        let legal_moves = self.find_legal_moves(piece_offsets);
        if legal_moves.is_empty() {
            return None;
        }
        
        // Calculate distance map
        let distance_map = self.calculate_distance_map();
        
        // Score all legal moves
        let mut scored_moves: Vec<ScoredMove> = legal_moves.iter()
            .map(|&(x, y)| {
                let score = self.score_move(x, y, &distance_map, piece_offsets);
                ScoredMove::new(x, y, score)
            })
            .collect();
        
        // Sort moves by score (highest first)
        scored_moves.sort_by(|a, b| b.cmp(a));
        
        // Return the best move
        scored_moves.first().map(|m| (m.x, m.y))
    }

    /// Output the best move or 0 0 if no legal moves
    /// Uses the improved heuristic to select the optimal move
    fn make_move(&self, piece_offsets: &[PieceOffset]) {
        if let Some((x, y)) = self.find_best_move(piece_offsets) {
            // Output the selected move to stdout
            println!("{} {}", x, y);
            
            // Minimal logging of the selected move
            #[cfg(debug_assertions)]
            eprintln!("Selected move: {} {}", x, y);
        } else {
            // No legal moves, output 0 0 as required by the protocol
            println!("0 0");
            
            // Minimal logging when no legal moves are found
            #[cfg(debug_assertions)]
            eprintln!("No legal moves found, outputting 0 0");
        }
        
        // Ensure output is flushed immediately
        io::stdout().flush().unwrap();
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
    
    // Read from stdin line by line
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
        
        // Parse player number from the first line
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
