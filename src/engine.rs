// Negamax + alpha-beta, iterative deepening, quiescence search on captures.
// Scores are in centipawns from the side to move's point of view.

use std::cmp::Reverse;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::board::{Board, Color, PieceType};
use crate::moves::{Move, insufficient_material, is_in_check, legal_moves, make_move, repetitions};

/// Score of "mated now"; a mate found `ply` half-moves from the root scores MATE - ply.
pub const MATE: i32 = 100_000;
const INFINITY: i32 = MATE + 1;

fn value(kind: PieceType) -> i32 {
    match kind {
        PieceType::Pawn => 100,
        PieceType::Knight => 320,
        PieceType::Bishop => 330,
        PieceType::Rook => 500,
        PieceType::Queen => 900,
        PieceType::King => 20_000,
    }
}

// Piece-square tables (Simplified Evaluation Function, T. Michniewski), written
// from White's point of view with a8 first: the same layout as Board::square.
#[rustfmt::skip]
const PAWN_TABLE: [i32; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,
    50, 50, 50, 50, 50, 50, 50, 50,
    10, 10, 20, 30, 30, 20, 10, 10,
     5,  5, 10, 25, 25, 10,  5,  5,
     0,  0,  0, 20, 20,  0,  0,  0,
     5, -5,-10,  0,  0,-10, -5,  5,
     5, 10, 10,-20,-20, 10, 10,  5,
     0,  0,  0,  0,  0,  0,  0,  0,
];
#[rustfmt::skip]
const KNIGHT_TABLE: [i32; 64] = [
    -50,-40,-30,-30,-30,-30,-40,-50,
    -40,-20,  0,  0,  0,  0,-20,-40,
    -30,  0, 10, 15, 15, 10,  0,-30,
    -30,  5, 15, 20, 20, 15,  5,-30,
    -30,  0, 15, 20, 20, 15,  0,-30,
    -30,  5, 10, 15, 15, 10,  5,-30,
    -40,-20,  0,  5,  5,  0,-20,-40,
    -50,-40,-30,-30,-30,-30,-40,-50,
];
#[rustfmt::skip]
const BISHOP_TABLE: [i32; 64] = [
    -20,-10,-10,-10,-10,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  0,  5, 10, 10,  5,  0,-10,
    -10,  5,  5, 10, 10,  5,  5,-10,
    -10,  0, 10, 10, 10, 10,  0,-10,
    -10, 10, 10, 10, 10, 10, 10,-10,
    -10,  5,  0,  0,  0,  0,  5,-10,
    -20,-10,-10,-10,-10,-10,-10,-20,
];
#[rustfmt::skip]
const ROOK_TABLE: [i32; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,
     5, 10, 10, 10, 10, 10, 10,  5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
    -5,  0,  0,  0,  0,  0,  0, -5,
     0,  0,  0,  5,  5,  0,  0,  0,
];
#[rustfmt::skip]
const QUEEN_TABLE: [i32; 64] = [
    -20,-10,-10, -5, -5,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  0,  5,  5,  5,  5,  0,-10,
     -5,  0,  5,  5,  5,  5,  0, -5,
      0,  0,  5,  5,  5,  5,  0, -5,
    -10,  5,  5,  5,  5,  5,  0,-10,
    -10,  0,  5,  0,  0,  0,  0,-10,
    -20,-10,-10, -5, -5,-10,-10,-20,
];
#[rustfmt::skip]
const KING_TABLE: [i32; 64] = [
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -30,-40,-40,-50,-50,-40,-40,-30,
    -20,-30,-30,-40,-40,-30,-30,-20,
    -10,-20,-20,-20,-20,-20,-20,-10,
     20, 20,  0,  0,  0,  0, 20, 20,
     20, 30, 10,  0,  0, 10, 30, 20,
];
/// In the endgame the king should walk to the centre instead of hiding.
#[rustfmt::skip]
const KING_ENDGAME_TABLE: [i32; 64] = [
    -50,-40,-30,-20,-20,-30,-40,-50,
    -30,-20,-10,  0,  0,-10,-20,-30,
    -30,-10, 20, 30, 30, 20,-10,-30,
    -30,-10, 30, 40, 40, 30,-10,-30,
    -30,-10, 30, 40, 40, 30,-10,-30,
    -30,-10, 20, 30, 30, 20,-10,-30,
    -30,-30,  0,  0,  0,  0,-30,-30,
    -50,-30,-30,-30,-30,-30,-30,-50,
];

/// Material + piece-square tables, from the side to move's point of view.
pub fn evaluate(board: &Board) -> i32 {
    // Endgame: no more than a rook and a minor piece (or one queen) left in total.
    let pieces: i32 = (0..64)
        .filter_map(|i| board.get(i))
        .filter(|p| !matches!(p.kind, PieceType::Pawn | PieceType::King))
        .map(|p| value(p.kind))
        .sum();
    let king_table = if pieces <= 900 {
        &KING_ENDGAME_TABLE
    } else {
        &KING_TABLE
    };
    let mut score = 0;
    for i in 0..64 {
        let Some(p) = board.get(i) else { continue };
        // Mirror the rank for Black: a8 (0) <-> a1 (56).
        let (square, sign) = match p.color {
            Color::White => (i, 1),
            Color::Black => (i ^ 56, -1),
        };
        let table = match p.kind {
            PieceType::Pawn => &PAWN_TABLE,
            PieceType::Knight => &KNIGHT_TABLE,
            PieceType::Bishop => &BISHOP_TABLE,
            PieceType::Rook => &ROOK_TABLE,
            PieceType::Queen => &QUEEN_TABLE,
            PieceType::King => king_table,
        };
        score += sign * (value(p.kind) + table[square]);
    }
    match board.who_play {
        Color::White => score,
        Color::Black => -score,
    }
}

/// The piece `m` captures, including a pawn taken en passant.
fn captured(board: &Board, m: Move) -> Option<PieceType> {
    match board.get(m.to) {
        Some(p) => Some(p.kind),
        None if board.en_passant == Some(m.to)
            && board.get(m.from).is_some_and(|p| p.kind == PieceType::Pawn) =>
        {
            Some(PieceType::Pawn)
        }
        None => None,
    }
}

/// Captures first, most valuable victim / least valuable attacker; then promotions.
fn order_moves(board: &Board, moves: &mut [Move]) {
    moves.sort_by_key(|&m| {
        let mut score = 0;
        if let Some(victim) = captured(board, m) {
            let attacker = board.get(m.from).map_or(0, |p| value(p.kind));
            score += 1_000_000 + 10 * value(victim) - attacker;
        }
        if let Some(kind) = m.promotion {
            score += 1_000_000 + value(kind);
        }
        Reverse(score)
    });
}

#[derive(Clone, Copy, Debug)]
pub struct SearchResult {
    /// None only when the side to move has no legal move.
    pub best: Option<Move>,
    pub score: i32,
    /// Depth of the iteration that produced `best`; 0 if time ran out before even
    /// one root move was searched (then `best` is the first move in order, score 0).
    pub depth: u32,
    pub nodes: u64,
}

struct Search<'a> {
    deadline: Instant,
    /// Set from another thread to stop the search early.
    stop: &'a AtomicBool,
    stopped: bool,
    nodes: u64,
    /// Game history followed by the current search path, for repetitions.
    history: Vec<Board>,
}

impl Search<'_> {
    fn out_of_time(&mut self) -> bool {
        if self.nodes.is_multiple_of(1024)
            && (Instant::now() >= self.deadline || self.stop.load(Ordering::Relaxed))
        {
            self.stopped = true;
        }
        self.stopped
    }

    fn child(
        &mut self,
        board: &Board,
        m: Move,
        depth: u32,
        ply: i32,
        alpha: i32,
        beta: i32,
    ) -> i32 {
        self.history.push(*board);
        let score = -self.negamax(&make_move(*board, m), depth, ply, -beta, -alpha);
        self.history.pop();
        score
    }

    fn negamax(&mut self, board: &Board, depth: u32, ply: i32, mut alpha: i32, beta: i32) -> i32 {
        self.nodes += 1;
        if self.out_of_time() {
            return 0;
        }
        // A position seen once before on the board or in the search counts as a draw.
        if insufficient_material(board) || repetitions(board, &self.history) > 1 {
            return 0;
        }
        // In check, search the evasions instead of standing pat in quiesce, so a
        // mate at the horizon is seen (one-ply check extension).
        if depth == 0 && !is_in_check(*board, board.who_play) {
            return self.quiesce(board, alpha, beta);
        }
        let mut moves = legal_moves(board);
        if moves.is_empty() {
            return if is_in_check(*board, board.who_play) {
                -MATE + ply
            } else {
                0
            };
        }
        if board.number_of_half_moves >= 100 {
            return 0;
        }
        order_moves(board, &mut moves);
        for m in moves {
            let score = self.child(board, m, depth.saturating_sub(1), ply + 1, alpha, beta);
            if self.stopped {
                return 0;
            }
            if score >= beta {
                return beta;
            }
            alpha = alpha.max(score);
        }
        alpha
    }

    /// Only captures (and queen promotions) until the position is quiet, so the
    /// evaluation never stops in the middle of an exchange.
    fn quiesce(&mut self, board: &Board, mut alpha: i32, beta: i32) -> i32 {
        self.nodes += 1;
        if self.out_of_time() {
            return 0;
        }
        let stand_pat = evaluate(board);
        if stand_pat >= beta {
            return beta;
        }
        alpha = alpha.max(stand_pat);
        let mut moves: Vec<Move> = legal_moves(board)
            .into_iter()
            .filter(|&m| match m.promotion {
                Some(kind) => kind == PieceType::Queen,
                None => captured(board, m).is_some(),
            })
            .collect();
        order_moves(board, &mut moves);
        for m in moves {
            let score = -self.quiesce(&make_move(*board, m), -beta, -alpha);
            if self.stopped {
                return 0;
            }
            if score >= beta {
                return beta;
            }
            alpha = alpha.max(score);
        }
        alpha
    }
}

/// Iterative deepening up to `max_depth` plies or until `time_limit` runs out; the
/// result comes from the last fully searched depth (or, if even depth 1 runs out of
/// time, from the depth 1 moves searched so far, if any: see `SearchResult::depth`).
/// `history` holds the positions played before `board`, for repetitions.
/// Setting `stop` from another thread ends the search like running out of time.
pub fn search(
    board: &Board,
    history: &[Board],
    max_depth: u32,
    time_limit: Duration,
    stop: &AtomicBool,
) -> SearchResult {
    let mut moves = legal_moves(board);
    order_moves(board, &mut moves);
    let mut result = SearchResult {
        best: moves.first().copied(),
        score: 0,
        depth: 0,
        nodes: 0,
    };
    let mut s = Search {
        deadline: Instant::now() + time_limit,
        stop,
        stopped: false,
        nodes: 0,
        history: history.to_vec(),
    };
    for depth in 1..=max_depth {
        if moves.is_empty() {
            break;
        }
        let mut alpha = -INFINITY;
        let mut best = 0;
        for (i, &m) in moves.iter().enumerate() {
            let score = s.child(board, m, depth - 1, 1, alpha, INFINITY);
            if s.stopped {
                break;
            }
            if score > alpha {
                alpha = score;
                best = i;
            }
        }
        // Unfinished iteration: keep the previous one. Depth 1 is kept if at least
        // one root move was searched (alpha is no longer -INFINITY).
        if s.stopped && (depth > 1 || alpha == -INFINITY) {
            break;
        }
        // The best move is searched first at the next depth.
        moves[..=best].rotate_right(1);
        result = SearchResult {
            best: Some(moves[0]),
            score: alpha,
            depth,
            nodes: s.nodes,
        };
        if s.stopped || alpha.abs() > MATE - 1000 {
            break; // out of time, or a forced mate: searching deeper changes nothing
        }
    }
    result.nodes = s.nodes;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{START_FEN, fen_to_board};
    use crate::moves::{Status, game_status, is_checkmate};

    const LONG: Duration = Duration::from_secs(60);
    static RUN: AtomicBool = AtomicBool::new(false);

    fn best(fen: &str, depth: u32) -> SearchResult {
        search(&fen_to_board(fen), &[], depth, LONG, &RUN)
    }

    fn uci(r: SearchResult) -> String {
        r.best.map(|m| m.to_string()).unwrap_or_default()
    }

    #[test]
    fn evaluation_is_symmetric() {
        assert_eq!(evaluate(&fen_to_board(START_FEN)), 0);
        let white = evaluate(&fen_to_board("4k3/8/8/8/8/8/8/3QK3 w - - 0 1"));
        let black = evaluate(&fen_to_board("3qk3/8/8/8/8/8/8/4K3 b - - 0 1"));
        assert!(white > 800);
        assert_eq!(white, black);
    }

    #[test]
    fn finds_mate_in_one() {
        let r = best("6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1", 3);
        assert_eq!(uci(r), "a1a8");
        assert_eq!(r.score, MATE - 1);
        // Fool's mate for Black.
        let r = best(
            "rnbqkbnr/pppp1ppp/8/4p3/6P1/5P2/PPPPP2P/RNBQKBNR b KQkq g3 0 2",
            3,
        );
        assert_eq!(uci(r), "d8h4");
    }

    #[test]
    fn sees_mate_at_the_horizon() {
        // At depth 1 the mated side is a depth-0 leaf: it must not stand pat in check.
        let r = best("6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1", 1);
        assert_eq!(uci(r), "a1a8");
        assert_eq!(r.score, MATE - 1);
    }

    /// Every reply to `m` allows a mate in one.
    fn forces_mate_in_two(board: Board, m: Move) -> bool {
        let after = make_move(board, m);
        let replies = legal_moves(&after);
        !replies.is_empty()
            && replies.iter().all(|&r| {
                let b = make_move(after, r);
                legal_moves(&b)
                    .iter()
                    .any(|&m2| is_checkmate(make_move(b, m2)))
            })
    }

    #[test]
    fn finds_mate_in_two() {
        // Two rooks: 1.Rb7 Kg8 2.Ra8# (or 1.Ra7 Kg8 2.Rb8#); no mate in one.
        let fen = "7k/8/8/8/8/8/R7/1R4K1 w - - 0 1";
        let r = best(fen, 5);
        assert_eq!(r.score, MATE - 3);
        assert!(
            forces_mate_in_two(fen_to_board(fen), r.best.unwrap()),
            "{}",
            uci(r)
        );
    }

    #[test]
    fn captures_a_hanging_queen() {
        assert_eq!(uci(best("4k3/8/8/3q4/8/8/3R4/4K3 w - - 0 1", 3)), "d2d5");
        assert_eq!(uci(best("4k3/3r4/8/8/3Q4/8/8/4K3 b - - 0 1", 3)), "d7d4");
    }

    #[test]
    fn quiescence_sees_the_recapture() {
        // Qxd5 wins a pawn at depth 1 but cxd5 loses the queen right after.
        let r = best("4k3/8/2p5/3p4/8/8/8/3QK3 w - - 0 1", 1);
        assert_ne!(uci(r), "d1d5");
    }

    #[test]
    fn avoids_a_repetition_when_ahead() {
        // A queen up, with the position after its favourite move already played
        // once: playing it again would be a repetition, scored as a draw.
        let board = fen_to_board("4k3/8/8/8/8/8/8/3QK3 w - - 10 40");
        let m = search(&board, &[], 3, LONG, &RUN).best.unwrap();
        let r = search(&board, &[make_move(board, m)], 3, LONG, &RUN);
        assert_ne!(r.best, Some(m));
        assert!(r.score > 500, "{}", r.score);
    }

    #[test]
    fn returns_a_legal_move() {
        let board = fen_to_board(START_FEN);
        let r = search(&board, &[], 4, LONG, &RUN);
        assert_eq!(r.depth, 4);
        assert!(legal_moves(&board).contains(&r.best.unwrap()));
    }

    #[test]
    fn time_limit_still_gives_a_move() {
        let r = search(&fen_to_board(START_FEN), &[], 64, Duration::ZERO, &RUN);
        assert!(r.best.is_some() && r.depth >= 1);
    }

    #[test]
    fn stop_flag_ends_the_search() {
        let t = Instant::now();
        let board = fen_to_board(START_FEN);
        let r = search(&board, &[], 64, LONG, &AtomicBool::new(true));
        assert!(t.elapsed() < Duration::from_secs(1));
        assert!(legal_moves(&board).contains(&r.best.unwrap()));
    }

    #[test]
    fn no_score_when_no_root_move_was_searched() {
        // Queens facing queens: the first root move alone takes over 1024 nodes,
        // so the stop flag (checked every 1024 nodes) hits before it is done.
        let board = fen_to_board("k7/8/qqqqqqqq/QQQQQQQQ/8/8/8/K7 w - - 0 1");
        let r = search(&board, &[], 64, LONG, &AtomicBool::new(true));
        assert_eq!((r.depth, r.score), (0, 0));
        assert!(legal_moves(&board).contains(&r.best.unwrap()));
    }

    #[test]
    fn no_move_at_game_end() {
        let mated = "rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3";
        assert!(best(mated, 4).best.is_none());
        assert!(best("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1", 4).best.is_none());
    }

    #[test]
    fn self_play_ends_without_crash() {
        let mut board = fen_to_board(START_FEN);
        let mut history = vec![];
        while game_status(&board, &history) == Status::Ongoing {
            let m = search(&board, &history, 2, LONG, &RUN).best.unwrap();
            assert!(legal_moves(&board).contains(&m));
            history.push(board);
            board = make_move(board, m);
            assert!(history.len() < 600, "game too long");
        }
        println!(
            "{:?} after {} plies",
            game_status(&board, &history),
            history.len()
        );
    }

    #[test]
    #[ignore = "benchmark: cargo test --release -- --ignored --nocapture"]
    fn start_position_speed() {
        let t = Instant::now();
        let r = search(
            &fen_to_board(START_FEN),
            &[],
            64,
            Duration::from_secs(1),
            &RUN,
        );
        let secs = t.elapsed().as_secs_f64();
        println!(
            "depth {} best {} score {} nodes {} in {secs:.2}s ({:.0} nodes/s)",
            r.depth,
            uci(r),
            r.score,
            r.nodes,
            r.nodes as f64 / secs
        );
    }
}
