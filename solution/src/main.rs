// Filler Bot - Modular Structure
// A sophisticated Filler game bot with clean modular architecture

mod types;
mod game;

use crate::game::GameState;
use std::io::{self, BufRead, Write};

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
