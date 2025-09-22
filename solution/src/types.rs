use std::cmp::Ordering;

/// Represents a player in the game
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Player {
    One,
    Two,
}

/// Represents a cell on the board
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Cell {
    Empty,
    Player1,
    Player2,
}

/// Represents a cell in a piece
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PieceCell {
    Empty,
    Filled,
}

/// Represents a scored move for evaluation
#[derive(Debug, Clone)]
pub struct ScoredMove {
    pub x: i32,
    pub y: i32,
    pub score: i32,
}

impl ScoredMove {
    pub fn new(x: i32, y: i32, score: i32) -> Self {
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
pub struct PieceOffset {
    pub dx: i32,
    pub dy: i32,
}
