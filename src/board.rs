// Square indexes: 0 = a8, 7 = h8, 56 = a1, 63 = h1 (rank 8 at the top, like the window).

pub const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PieceType {
    King,
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub fn other(self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Piece {
    pub kind: PieceType,
    pub color: Color,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Board {
    pub square: [Option<Piece>; 64],
    pub who_play: Color,
    pub can_rook_black_small: bool,
    pub can_rook_black_big: bool,
    pub can_rook_white_small: bool,
    pub can_rook_white_big: bool,
    /// Square a pawn can capture en passant onto (the square it jumped over).
    pub en_passant: Option<usize>,
    /// Half-move clock for the 50-move rule.
    pub number_of_half_moves: u64,
    pub number_of_moves: u64,
}

impl Board {
    /// Empty board, White to move, no castling rights.
    pub fn new() -> Self {
        Board {
            square: [const { None }; 64],
            who_play: Color::White,
            can_rook_black_big: false,
            can_rook_white_small: false,
            can_rook_white_big: false,
            can_rook_black_small: false,
            en_passant: None,
            number_of_half_moves: 0,
            number_of_moves: 1,
        }
    }
    pub fn set(&mut self, index: usize, piece: Piece) {
        self.square[index] = Some(piece);
    }
    pub fn get(&self, index: usize) -> Option<Piece> {
        self.square[index]
    }
    /// Same pieces, side to move, castling rights and en passant square (clocks ignored).
    pub fn same_position(&self, other: &Board) -> bool {
        let no_clocks = |b: &Board| Board {
            number_of_half_moves: 0,
            number_of_moves: 0,
            ..*b
        };
        no_clocks(self) == no_clocks(other)
    }
}

fn piece_from_char(c: char) -> Option<Piece> {
    let kind = match c.to_ascii_lowercase() {
        'k' => PieceType::King,
        'p' => PieceType::Pawn,
        'n' => PieceType::Knight,
        'b' => PieceType::Bishop,
        'r' => PieceType::Rook,
        'q' => PieceType::Queen,
        _ => return None,
    };
    let color = if c.is_ascii_uppercase() {
        Color::White
    } else {
        Color::Black
    };
    Some(Piece { kind, color })
}

#[cfg(test)]
fn piece_to_char(p: Piece) -> char {
    let c = match p.kind {
        PieceType::King => 'k',
        PieceType::Pawn => 'p',
        PieceType::Knight => 'n',
        PieceType::Bishop => 'b',
        PieceType::Rook => 'r',
        PieceType::Queen => 'q',
    };
    match p.color {
        Color::White => c.to_ascii_uppercase(),
        Color::Black => c,
    }
}

/// "e4" for index 36.
pub fn square_name(index: usize) -> String {
    let file = (b'a' + (index % 8) as u8) as char;
    let rank = 8 - index / 8;
    format!("{file}{rank}")
}

pub fn square_from_name(name: &str) -> Option<usize> {
    let b = name.as_bytes();
    if b.len() != 2 || !(b'a'..=b'h').contains(&b[0]) || !(b'1'..=b'8').contains(&b[1]) {
        return None;
    }
    Some((b'8' - b[1]) as usize * 8 + (b[0] - b'a') as usize)
}

/// Lenient parser: unknown characters are ignored and missing fields get defaults.
pub fn fen_to_board(fen: &str) -> Board {
    let mut board = Board::new();
    let mut fields = fen.split_whitespace();
    let mut i = 0;
    for c in fields.next().unwrap_or("").chars() {
        if let Some(d) = c.to_digit(10) {
            i += d as usize;
        } else if let Some(piece) = piece_from_char(c)
            && i < 64
        {
            board.set(i, piece);
            i += 1;
        }
    }
    if fields.next() == Some("b") {
        board.who_play = Color::Black;
    }
    let castling = fields.next().unwrap_or("-");
    board.can_rook_white_small = castling.contains('K');
    board.can_rook_white_big = castling.contains('Q');
    board.can_rook_black_small = castling.contains('k');
    board.can_rook_black_big = castling.contains('q');
    board.en_passant = fields.next().and_then(square_from_name);
    board.number_of_half_moves = fields.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    board.number_of_moves = fields.next().and_then(|s| s.parse().ok()).unwrap_or(1);
    board
}

#[cfg(test)]
pub fn to_fen(board: &Board) -> String {
    let mut fen = String::new();
    for rank in 0..8 {
        let mut empty = 0;
        for file in 0..8 {
            match board.get(rank * 8 + file) {
                Some(p) => {
                    if empty > 0 {
                        fen.push_str(&empty.to_string());
                        empty = 0;
                    }
                    fen.push(piece_to_char(p));
                }
                None => empty += 1,
            }
        }
        if empty > 0 {
            fen.push_str(&empty.to_string());
        }
        if rank < 7 {
            fen.push('/');
        }
    }
    fen.push_str(match board.who_play {
        Color::White => " w ",
        Color::Black => " b ",
    });
    let rights = [
        (board.can_rook_white_small, 'K'),
        (board.can_rook_white_big, 'Q'),
        (board.can_rook_black_small, 'k'),
        (board.can_rook_black_big, 'q'),
    ];
    let castling: String = rights.iter().filter(|r| r.0).map(|r| r.1).collect();
    fen.push_str(if castling.is_empty() { "-" } else { &castling });
    let ep = board.en_passant.map_or("-".to_string(), square_name);
    fen.push_str(&format!(
        " {ep} {} {}",
        board.number_of_half_moves, board.number_of_moves
    ));
    fen
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fen_round_trip() {
        for fen in [
            START_FEN,
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
            "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
            "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq e6 0 2",
            "4k3/8/8/8/8/8/8/4K3 b - - 37 112",
        ] {
            assert_eq!(to_fen(&fen_to_board(fen)), fen);
        }
    }

    #[test]
    fn fen_fields_are_parsed() {
        let b = fen_to_board("4k3/8/8/3pP3/8/8/8/4K3 b Kq d6 12 34");
        assert_eq!(b.who_play, Color::Black);
        assert!(b.can_rook_white_small && !b.can_rook_white_big);
        assert!(!b.can_rook_black_small && b.can_rook_black_big);
        assert_eq!(b.en_passant, square_from_name("d6"));
        assert_eq!(b.number_of_half_moves, 12);
        assert_eq!(b.number_of_moves, 34);
        assert_eq!(
            b.get(28),
            Some(Piece {
                kind: PieceType::Pawn,
                color: Color::White
            })
        );
    }

    #[test]
    fn square_names() {
        assert_eq!(square_name(0), "a8");
        assert_eq!(square_name(63), "h1");
        assert_eq!(square_from_name("e4"), Some(36));
        assert_eq!(square_from_name("i9"), None);
    }
}
