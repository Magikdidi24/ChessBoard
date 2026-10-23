use std::fmt;

use crate::board::{Board, Color, Piece, PieceType, square_name};

#[rustfmt::skip]
const KING_MOVES: [[(isize, isize); 1]; 8] = [
    [(-1, -1)], [(-1, 0)], [(-1, 1)],
    [( 0, -1)],            [( 0, 1)],
    [( 1, -1)], [( 1, 0)], [( 1, 1)],
];
#[rustfmt::skip]
const ROOK_MOVES: [[(isize, isize); 7]; 4] = [
    [(-1, 0), (-2, 0), (-3, 0), (-4, 0), (-5, 0), (-6, 0), (-7, 0)], // up
    [( 1, 0), ( 2, 0), ( 3, 0), ( 4, 0), ( 5, 0), ( 6, 0), ( 7, 0)], // down
    [(0, -1), (0, -2), (0, -3), (0, -4), (0, -5), (0, -6), (0, -7)], // left
    [(0,  1), (0,  2), (0,  3), (0,  4), (0,  5), (0,  6), (0,  7)], // right
];
#[rustfmt::skip]
const BISHOP_MOVES: [[(isize, isize); 7]; 4] = [
    [(-1, -1), (-2, -2), (-3, -3), (-4, -4), (-5, -5), (-6, -6), (-7, -7)], // up-left
    [( 1, -1), ( 2, -2), ( 3, -3), ( 4, -4), ( 5, -5), ( 6, -6), ( 7, -7)], // up-right
    [(-1,  1), (-2,  2), (-3,  3), (-4,  4), (-5,  5), (-6,  6), (-7,  7)], // down-left
    [( 1,  1), ( 2,  2), ( 3,  3), ( 4,  4), ( 5,  5), ( 6,  6), ( 7,  7)], // down-right
];

#[rustfmt::skip]
const KNIGHT_MOVES: [[(isize, isize); 1]; 8] = [
    [(-1, -2)],[(-2, -1)],[(1, -2)],[(-2, 1)],[(2, -1)],[(-1, 2)],[(1, 2)],[(2, 1)],
];

#[rustfmt::skip]
const QUEEN_MOVES: [[(isize, isize); 7]; 8] = [
    [(-1, 0), (-2, 0), (-3, 0), (-4, 0), (-5, 0), (-6, 0), (-7, 0)],  // up
    [( 1, 0), ( 2, 0), ( 3, 0), ( 4, 0), ( 5, 0), ( 6, 0), ( 7, 0)],  // down
    [(0, -1), (0, -2), (0, -3), (0, -4), (0, -5), (0, -6), (0, -7)],  // left
    [(0,  1), (0,  2), (0,  3), (0,  4), (0,  5), (0,  6), (0,  7)],  // right
    [(-1, -1), (-2, -2), (-3, -3), (-4, -4), (-5, -5), (-6, -6), (-7, -7)],  // up-left
    [( 1, -1), ( 2, -2), ( 3, -3), ( 4, -4), ( 5, -5), ( 6, -6), ( 7, -7)],  // up-right
    [(-1,  1), (-2,  2), (-3,  3), (-4,  4), (-5,  5), (-6,  6), (-7,  7)],  // down-left
    [( 1,  1), ( 2,  2), ( 3,  3), ( 4,  4), ( 5,  5), ( 6,  6), ( 7,  7)],  // down-right
];

fn helper_check_legal_move<const T: usize, const N: usize>(
    index: usize,
    board: Board,
    mut mov: Vec<usize>,
    moves_pieces: [[(isize, isize); T]; N],
    p: Piece,
) -> Vec<usize> {
    let file = (index % 8) as isize;
    let rank = (index / 8) as isize;
    for directions in moves_pieces {
        for (df, dr) in directions {
            let nf = file + df;
            let nr = rank + dr;
            if !(0..=7).contains(&nf) || !(0..=7).contains(&nr) {
                continue;
            }
            let n_idx = (nr * 8 + nf) as usize;
            match board.get(n_idx) {
                Some(other) if other.color != p.color => {
                    mov.push(n_idx);
                    break;
                }
                None => mov.push(n_idx),
                _ => break,
            }
        }
    }
    mov
}

fn pawn_moves(index: usize, board: Board, mut mov: Vec<usize>, p: Piece) -> Vec<usize> {
    let file = (index % 8) as isize;
    let rank = (index / 8) as isize;
    let (dir, starting_rank): (isize, isize) = match p.color {
        Color::White => (-1, 6),
        Color::Black => (1, 1),
    };

    let nr = rank + dir;
    if (0..=7).contains(&nr) {
        let one_ahead = (nr * 8 + file) as usize;
        if board.get(one_ahead).is_none() {
            mov.push(one_ahead);
            if rank == starting_rank {
                let nr2 = rank + 2 * dir;
                if (0..=7).contains(&nr2) {
                    let two_ahead = (nr2 * 8 + file) as usize;
                    if board.get(two_ahead).is_none() {
                        mov.push(two_ahead);
                    }
                }
            }
        }
    }
    for nf in [file - 1, file + 1] {
        if !(0..=7).contains(&nr) || !(0..=7).contains(&nf) {
            continue;
        }
        let diag = (nr * 8 + nf) as usize;
        let capture = board.get(diag).is_some_and(|other| other.color != p.color);
        let en_passant = board.en_passant == Some(diag) && board.who_play == p.color;
        if capture || en_passant {
            mov.push(diag);
        }
    }
    mov
}

/// Castling is encoded as the king moving two squares (e1g1, e1c1, e8g8, e8c8).
fn castling_moves(board: Board, index: usize, p: Piece, mov: &mut Vec<usize>) {
    // (right, rook square, squares that must be empty, squares the king crosses, king target)
    let options = match p.color {
        Color::White if index == 60 => [
            (
                board.can_rook_white_small,
                63,
                &[61, 62][..],
                [60, 61, 62],
                62,
            ),
            (
                board.can_rook_white_big,
                56,
                &[57, 58, 59][..],
                [60, 59, 58],
                58,
            ),
        ],
        Color::Black if index == 4 => [
            (board.can_rook_black_small, 7, &[5, 6][..], [4, 5, 6], 6),
            (board.can_rook_black_big, 0, &[1, 2, 3][..], [4, 3, 2], 2),
        ],
        _ => return,
    };
    let rook = Some(Piece {
        kind: PieceType::Rook,
        color: p.color,
    });
    for (right, rook_square, empty, crossed, target) in options {
        // The rook, and b1/b8 for the long castle, may be attacked: only the king's path matters.
        if right
            && board.square[rook_square] == rook
            && empty.iter().all(|&s| board.square[s].is_none())
            && crossed.iter().all(|&s| !is_case_attack(s, board, p.color))
        {
            mov.push(target);
        }
    }
}

/// Pseudo-legal target squares of the piece on `index` (may leave the own king in check).
pub fn check_legal_move(board: Board, index: usize) -> Vec<usize> {
    let mut mov: Vec<usize> = vec![];
    if let Some(p) = board.get(index) {
        mov = match p.kind {
            PieceType::Knight => helper_check_legal_move(index, board, mov, KNIGHT_MOVES, p),
            PieceType::King => {
                let mut m = helper_check_legal_move(index, board, mov, KING_MOVES, p);
                castling_moves(board, index, p, &mut m);
                m
            }
            PieceType::Rook => helper_check_legal_move(index, board, mov, ROOK_MOVES, p),
            PieceType::Bishop => helper_check_legal_move(index, board, mov, BISHOP_MOVES, p),
            PieceType::Queen => helper_check_legal_move(index, board, mov, QUEEN_MOVES, p),
            PieceType::Pawn => pawn_moves(index, board, mov, p),
        };
    }
    mov
}

/// True when the first piece met along one of `rays` from `index` is an
/// `attacker` piece of one of `kinds`.
fn ray_hits<const T: usize, const N: usize>(
    index: usize,
    board: Board,
    rays: [[(isize, isize); T]; N],
    attacker: Color,
    kinds: &[PieceType],
) -> bool {
    let file = (index % 8) as isize;
    let rank = (index / 8) as isize;
    for ray in rays {
        for (df, dr) in ray {
            let nf = file + df;
            let nr = rank + dr;
            if !(0..=7).contains(&nf) || !(0..=7).contains(&nr) {
                break;
            }
            if let Some(p) = board.get((nr * 8 + nf) as usize) {
                if p.color == attacker && kinds.contains(&p.kind) {
                    return true;
                }
                break;
            }
        }
    }
    false
}

/// Is square `index` attacked by the opponent of `color`?
/// Looks outwards from the square, as every move is symmetric except the pawn's.
pub fn is_case_attack(index: usize, board: Board, color: Color) -> bool {
    use PieceType::*;
    let enemy = color.other();
    // An enemy pawn attacks us from one rank "behind" its direction of travel.
    let back = match enemy {
        Color::White => 1,
        Color::Black => -1,
    };
    ray_hits(index, board, ROOK_MOVES, enemy, &[Rook, Queen])
        || ray_hits(index, board, BISHOP_MOVES, enemy, &[Bishop, Queen])
        || ray_hits(index, board, KNIGHT_MOVES, enemy, &[Knight])
        || ray_hits(index, board, KING_MOVES, enemy, &[King])
        || ray_hits(index, board, [[(-1, back)], [(1, back)]], enemy, &[Pawn])
}

pub fn is_in_check(board: Board, color: Color) -> bool {
    for i in 0..64 {
        if let Some(piece) = board.get(i)
            && piece.kind == PieceType::King
            && piece.color == color
        {
            return is_case_attack(i, board, color);
        }
    }
    false
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Move {
    pub from: usize,
    pub to: usize,
    pub promotion: Option<PieceType>,
}

impl fmt::Display for Move {
    /// Coordinate notation: "e2e4", "e7e8q".
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let promo = match self.promotion {
            Some(PieceType::Queen) => "q",
            Some(PieceType::Rook) => "r",
            Some(PieceType::Bishop) => "b",
            Some(PieceType::Knight) => "n",
            _ => "",
        };
        write!(
            f,
            "{}{}{promo}",
            square_name(self.from),
            square_name(self.to)
        )
    }
}

/// Copy-make: returns the position after `m`, which must come from `legal_moves`
/// (or at least be pseudo-legal).
pub fn make_move(mut board: Board, m: Move) -> Board {
    let piece = board.square[m.from].expect("make_move: empty from square");
    let captured = board.square[m.to];
    board.square[m.from] = None;
    board.square[m.to] = Some(Piece {
        kind: m.promotion.unwrap_or(piece.kind),
        color: piece.color,
    });
    let pawn = piece.kind == PieceType::Pawn;
    if pawn && board.en_passant == Some(m.to) && m.from % 8 != m.to % 8 {
        // The captured pawn sits next to the capturing one, not on the target square.
        board.square[m.from / 8 * 8 + m.to % 8] = None;
    }
    if piece.kind == PieceType::King && m.from.abs_diff(m.to) == 2 {
        let (rook_from, rook_to) = if m.to > m.from {
            (m.from + 3, m.from + 1)
        } else {
            (m.from - 4, m.from - 1)
        };
        board.square[rook_to] = board.square[rook_from];
        board.square[rook_from] = None;
    }
    // Only when an enemy pawn stands beside the pushed one: otherwise no capture is
    // possible and, for repetitions (FIDE), the position is the same as without it.
    // shortcut: a pinned neighbour still sets it; test the capture's legality if that matters.
    let enemy_pawn = Some(Piece {
        kind: PieceType::Pawn,
        color: piece.color.other(),
    });
    let enemy_pawn_on = |s: usize| s / 8 == m.to / 8 && board.square[s] == enemy_pawn;
    board.en_passant = if pawn
        && m.from.abs_diff(m.to) == 16
        && (enemy_pawn_on(m.to - 1) || enemy_pawn_on(m.to + 1))
    {
        Some((m.from + m.to) / 2)
    } else {
        None
    };
    // Moving from or onto a king/rook home square loses the matching rights.
    for s in [m.from, m.to] {
        match s {
            60 => {
                board.can_rook_white_small = false;
                board.can_rook_white_big = false;
            }
            63 => board.can_rook_white_small = false,
            56 => board.can_rook_white_big = false,
            4 => {
                board.can_rook_black_small = false;
                board.can_rook_black_big = false;
            }
            7 => board.can_rook_black_small = false,
            0 => board.can_rook_black_big = false,
            _ => {}
        }
    }
    if pawn || captured.is_some() {
        board.number_of_half_moves = 0;
    } else {
        board.number_of_half_moves += 1;
    }
    if board.who_play == Color::Black {
        board.number_of_moves += 1;
    }
    board.who_play = board.who_play.other();
    board
}

/// Legal target squares of the piece on `index`: the pseudo-legal ones that do not
/// leave its own king in check.
pub fn save_king(board: Board, index: usize) -> Vec<usize> {
    let me = match board.get(index) {
        Some(p) => p.color,
        None => return vec![],
    };
    check_legal_move(board, index)
        .into_iter()
        .filter(|&to| {
            let m = Move {
                from: index,
                to,
                promotion: None,
            };
            !is_in_check(make_move(board, m), me)
        })
        .collect()
}

/// All legal moves of the side to move; a promotion gives four moves.
pub fn legal_moves(board: &Board) -> Vec<Move> {
    let mut moves = vec![];
    for from in 0..64 {
        let Some(p) = board.get(from) else { continue };
        if p.color != board.who_play {
            continue;
        }
        for to in save_king(*board, from) {
            if p.kind == PieceType::Pawn && !(8..56).contains(&to) {
                for kind in [
                    PieceType::Queen,
                    PieceType::Rook,
                    PieceType::Bishop,
                    PieceType::Knight,
                ] {
                    moves.push(Move {
                        from,
                        to,
                        promotion: Some(kind),
                    });
                }
            } else {
                moves.push(Move {
                    from,
                    to,
                    promotion: None,
                });
            }
        }
    }
    moves
}

pub fn is_checkmate(board: Board) -> bool {
    is_in_check(board, board.who_play) && legal_moves(&board).is_empty()
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Status {
    Ongoing,
    /// The side to move is mated.
    Checkmate,
    Stalemate,
    FiftyMoves,
    InsufficientMaterial,
    Repetition,
}

/// `history` holds the positions played before `board`, oldest first.
pub fn game_status(board: &Board, history: &[Board]) -> Status {
    if is_checkmate(*board) {
        Status::Checkmate
    } else if legal_moves(board).is_empty() {
        Status::Stalemate
    } else if board.number_of_half_moves >= 100 {
        Status::FiftyMoves
    } else if insufficient_material(board) {
        Status::InsufficientMaterial
    } else if repetitions(board, history) >= 3 {
        Status::Repetition
    } else {
        Status::Ongoing
    }
}

/// How many times `board` has occurred, counting itself.
pub fn repetitions(board: &Board, history: &[Board]) -> usize {
    // Nothing before the last capture or pawn move can repeat.
    let reversible = history
        .iter()
        .rev()
        .take(board.number_of_half_moves as usize);
    1 + reversible.filter(|b| b.same_position(board)).count()
}

/// Basic cases only: K v K, K + minor v K, and bishops that all stand on one colour.
pub fn insufficient_material(board: &Board) -> bool {
    let mut minors = vec![]; // (kind, square colour)
    for i in 0..64 {
        if let Some(p) = board.get(i) {
            match p.kind {
                PieceType::King => {}
                PieceType::Knight | PieceType::Bishop => minors.push((p.kind, (i / 8 + i % 8) % 2)),
                _ => return false,
            }
        }
    }
    minors.len() <= 1
        || minors
            .iter()
            .all(|&(kind, colour)| kind == PieceType::Bishop && colour == minors[0].1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{START_FEN, fen_to_board, to_fen};

    fn perft(board: &Board, depth: u32) -> u64 {
        if depth == 0 {
            return 1;
        }
        let moves = legal_moves(board);
        if depth == 1 {
            return moves.len() as u64;
        }
        moves
            .iter()
            .map(|&m| perft(&make_move(*board, m), depth - 1))
            .sum()
    }

    fn check_perft(fen: &str, expected: &[u64]) {
        let board = fen_to_board(fen);
        for (depth, &nodes) in (1..).zip(expected) {
            assert_eq!(perft(&board, depth), nodes, "{fen} depth {depth}");
        }
    }

    const KIWIPETE: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";

    #[test]
    fn perft_start_position() {
        check_perft(START_FEN, &[20, 400, 8902, 197281]);
    }

    #[test]
    #[ignore = "slow: run with cargo test --release -- --ignored"]
    fn perft_start_position_depth_5() {
        assert_eq!(perft(&fen_to_board(START_FEN), 5), 4865609);
    }

    #[test]
    fn perft_kiwipete() {
        check_perft(KIWIPETE, &[48, 2039, 97862]);
    }

    #[test]
    fn perft_position_3() {
        check_perft(
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
            &[14, 191, 2812, 43238],
        );
    }

    #[test]
    fn perft_position_4() {
        check_perft(
            "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
            &[6, 264, 9467],
        );
    }

    fn play(fen: &str, uci: &str) -> String {
        let board = fen_to_board(fen);
        let m = legal_moves(&board)
            .into_iter()
            .find(|m| m.to_string() == uci)
            .unwrap_or_else(|| panic!("{uci} is not legal in {fen}"));
        to_fen(&make_move(board, m))
    }

    fn is_legal(fen: &str, uci: &str) -> bool {
        legal_moves(&fen_to_board(fen))
            .iter()
            .any(|m| m.to_string() == uci)
    }

    #[test]
    fn castling_moves_the_rook_and_clears_rights() {
        assert_eq!(
            play(KIWIPETE, "e1g1"),
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R4RK1 b kq - 1 1"
        );
        assert_eq!(
            play("r3k2r/8/8/8/8/8/8/R3K2R b KQkq - 0 1", "e8c8"),
            "2kr3r/8/8/8/8/8/8/R3K2R w KQ - 1 2"
        );
    }

    #[test]
    fn no_castling_out_of_or_through_check() {
        // e1 attacked by the e8 rook.
        assert!(!is_legal("4r1k1/8/8/8/8/8/8/R3K2R w KQ - 0 1", "e1c1"));
        assert!(!is_legal("4r1k1/8/8/8/8/8/8/R3K2R w KQ - 0 1", "e1g1"));
        // d1 attacked: no long castle, short castle still fine.
        assert!(!is_legal("3r2k1/8/8/8/8/8/8/R3K2R w KQ - 0 1", "e1c1"));
        assert!(is_legal("3r2k1/8/8/8/8/8/8/R3K2R w KQ - 0 1", "e1g1"));
        // Only b1 attacked: long castle allowed.
        assert!(is_legal("1r4k1/8/8/8/8/8/8/R3K2R w KQ - 0 1", "e1c1"));
    }

    #[test]
    fn capturing_a_rook_clears_its_castling_right() {
        assert_eq!(
            play("r3k2r/8/8/8/8/8/6p1/R3K2R b KQkq - 0 1", "g2h1q"),
            "r3k2r/8/8/8/8/8/8/R3K2q w Qkq - 0 2"
        );
    }

    #[test]
    fn en_passant_capture_and_square() {
        // No black pawn beside e4 (or across the board edge from a4/h4): no square.
        assert_eq!(
            play(START_FEN, "e2e4"),
            "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1"
        );
        assert_eq!(
            play("4k3/8/8/8/8/p7/7P/4K3 w - - 0 1", "h2h4"),
            "4k3/8/8/8/7P/p7/8/4K3 b - - 0 1"
        );
        assert_eq!(
            play("4k3/8/8/8/3p4/8/4P3/4K3 w - - 0 1", "e2e4"),
            "4k3/8/8/8/3pP3/8/8/4K3 b - e3 0 1"
        );
        assert_eq!(
            play(
                "rnbqkbnr/ppp1p1pp/8/3pPp2/8/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 3",
                "e5f6"
            ),
            "rnbqkbnr/ppp1p1pp/5P2/3p4/8/8/PPPP1PPP/RNBQKBNR b KQkq - 0 3"
        );
    }

    #[test]
    fn make_move_clears_en_passant_and_updates_the_clock() {
        // Any other move drops the en passant square; a quiet piece move adds one.
        assert_eq!(
            play("4k3/8/8/8/3pP3/8/8/4K3 b - e3 0 1", "e8d7"),
            "8/3k4/8/8/3pP3/8/8/4K3 w - - 1 2"
        );
        // A pawn move or a capture resets the half-move clock.
        assert_eq!(
            play("4k3/8/8/8/3p4/8/8/4K3 b - - 7 30", "d4d3"),
            "4k3/8/8/8/8/3p4/8/4K3 w - - 0 31"
        );
        assert_eq!(
            play("4k3/8/8/8/8/8/3r4/4K3 w - - 7 30", "e1d2"),
            "4k3/8/8/8/8/8/3K4/8 b - - 0 30"
        );
    }

    #[test]
    fn en_passant_that_exposes_the_king_is_illegal() {
        // Taking on d6 would empty the 5th rank between the h5 rook and the a5 king.
        assert!(!is_legal("8/8/8/K2pP2r/8/8/8/7k w - d6 0 1", "e5d6"));
        assert!(is_legal("8/8/8/K2pP3/8/8/8/7k w - d6 0 1", "e5d6"));
    }

    #[test]
    fn all_four_promotions() {
        let moves: Vec<String> = legal_moves(&fen_to_board("8/P6k/8/8/8/8/8/K7 w - - 0 1"))
            .iter()
            .map(|m| m.to_string())
            .filter(|s| s.starts_with("a7"))
            .collect();
        assert_eq!(moves, ["a7a8q", "a7a8r", "a7a8b", "a7a8n"]);
    }

    fn status(fen: &str) -> Status {
        game_status(&fen_to_board(fen), &[])
    }

    #[test]
    fn game_end_detection() {
        let fools_mate = "rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3";
        assert_eq!(status(fools_mate), Status::Checkmate);
        assert_eq!(status("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1"), Status::Stalemate);
        assert_eq!(
            status("4k3/8/8/8/8/8/8/R3K3 w - - 100 80"),
            Status::FiftyMoves
        );
        assert_eq!(status("4k3/8/8/8/8/8/8/R3K3 w - - 99 80"), Status::Ongoing);
        assert_eq!(status(START_FEN), Status::Ongoing);
    }

    #[test]
    fn mate_on_the_hundredth_half_move_is_mate() {
        // FIDE 9.3: a checkmate given by the 50th move wins, it is not a draw.
        let fools_mate = "rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 100 3";
        assert_eq!(status(fools_mate), Status::Checkmate);
    }

    #[test]
    fn insufficient_material_cases() {
        let draw = Status::InsufficientMaterial;
        assert_eq!(status("4k3/8/8/8/8/8/8/4K3 w - - 0 1"), draw);
        assert_eq!(status("4k3/8/8/8/8/8/8/2B1K3 w - - 0 1"), draw);
        assert_eq!(status("4k3/8/8/8/8/8/8/1N2K3 w - - 0 1"), draw);
        // Bishops on c1 and f8: both dark squares.
        assert_eq!(status("4kb2/8/8/8/8/8/8/2B1K3 w - - 0 1"), draw);
        // Opposite coloured bishops, two knights, a pawn: mate still possible.
        assert_eq!(status("2b1k3/8/8/8/8/8/8/2B1K3 w - - 0 1"), Status::Ongoing);
        assert_eq!(status("4k3/8/8/8/8/8/8/1N2KN2 w - - 0 1"), Status::Ongoing);
        assert_eq!(status("4k3/8/8/8/8/8/P7/4K3 w - - 0 1"), Status::Ongoing);
    }

    #[test]
    fn threefold_repetition() {
        let mut board = fen_to_board(START_FEN);
        let mut history = vec![];
        let knights = ["g1f3", "g8f6", "f3g1", "f6g8"];
        for (ply, uci) in knights.iter().chain(&knights).enumerate() {
            assert_eq!(game_status(&board, &history), Status::Ongoing, "ply {ply}");
            let m = legal_moves(&board)
                .into_iter()
                .find(|m| m.to_string() == *uci);
            history.push(board);
            board = make_move(board, m.unwrap());
        }
        assert_eq!(repetitions(&board, &history), 3);
        assert_eq!(game_status(&board, &history), Status::Repetition);
    }

    #[test]
    fn useless_en_passant_square_does_not_hide_a_repetition() {
        let mut board = fen_to_board(START_FEN);
        let mut history = vec![];
        for uci in ["e2e4", "g8f6", "b1c3", "f6g8", "c3b1"] {
            let m = legal_moves(&board)
                .into_iter()
                .find(|m| m.to_string() == uci);
            history.push(board);
            board = make_move(board, m.unwrap());
        }
        // Same position as after 1.e4: no black pawn could ever take on e3.
        assert_eq!(repetitions(&board, &history), 2);
    }

    #[test]
    fn checkmate_needs_every_piece_stuck() {
        // Fool's mate.
        let mated = fen_to_board("rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3");
        assert!(is_checkmate(mated));
        // Check that the bishop can block.
        let check = fen_to_board("4k3/8/8/8/7q/8/7B/4K3 w - - 0 1");
        assert!(is_in_check(check, Color::White) && !is_checkmate(check));
    }
}
