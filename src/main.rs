use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Duration;

use macroquad::{
    color::{BEIGE, BLACK, BROWN, WHITE},
    input::{KeyCode, MouseButton, is_key_pressed, is_mouse_button_pressed, mouse_position},
    math::vec2,
    shapes::{draw_circle, draw_rectangle},
    text::draw_text,
    texture::{DrawTextureParams, draw_texture_ex, load_texture},
    window::{Conf, clear_background, next_frame},
};

mod board;
mod engine;
mod moves;

use board::{Board, Color, Piece, PieceType, START_FEN, fen_to_board};
use engine::{MATE, SearchResult};
use moves::{Move, Status, game_status, is_in_check, make_move, save_king};

const SQUARE_SIZE: f32 = 100.0;
const BOARD_SIZE: f32 = SQUARE_SIZE * 8.0;
const STATUS_HEIGHT: f32 = 60.0;

/// Thinking time per engine move. The search runs in its own thread, so the
/// window keeps refreshing meanwhile.
const ENGINE_TIME: Duration = Duration::from_secs(1);
const ENGINE_MAX_DEPTH: u32 = 32;

type Rgba = macroquad::color::Color;
const MOVE_DOT: Rgba = Rgba::new(0.0, 0.0, 1.0, 0.4);
const SELECTED: Rgba = Rgba::new(0.0, 1.0, 0.0, 0.4);
const LAST_MOVE: Rgba = Rgba::new(1.0, 1.0, 0.0, 0.35);
const SHADOW: Rgba = Rgba::new(0.0, 0.0, 0.0, 0.5);

/// Piece images in `PieceType` order (King, Pawn, Knight, Bishop, Rook, Queen),
/// white then black, so a piece's image is at `kind as usize * 2 + color as usize`.
const PIECE_FILES: [&str; 12] = [
    "wK", "bK", "wP", "bP", "wN", "bN", "wB", "bB", "wR", "bR", "wQ", "bQ",
];

const PROMOTION_CHOICES: [PieceType; 4] = [
    PieceType::Queen,
    PieceType::Rook,
    PieceType::Bishop,
    PieceType::Knight,
];

fn window_conf() -> Conf {
    Conf {
        window_title: "Chess".to_owned(),
        window_width: BOARD_SIZE as i32,
        window_height: (BOARD_SIZE + STATUS_HEIGHT) as i32,
        window_resizable: false,
        ..Default::default()
    }
}

/// A search running in its own thread. Dropping it (restart, mode switch) tells
/// that search to stop instead of letting it burn a core until its time is up.
struct Thinking {
    rx: Receiver<SearchResult>,
    stop: Arc<AtomicBool>,
}

impl Drop for Thinking {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

struct Game {
    board: Board,
    /// Positions before `board`, for threefold repetition.
    history: Vec<Board>,
    status: Status,
    last_move: Option<Move>,
}

impl Game {
    fn new() -> Self {
        Game {
            board: fen_to_board(START_FEN),
            history: vec![],
            status: Status::Ongoing,
            last_move: None,
        }
    }

    fn play(&mut self, m: Move) {
        self.history.push(self.board);
        self.board = make_move(self.board, m);
        self.last_move = Some(m);
        self.status = game_status(&self.board, &self.history);
    }
}

fn color_name(c: Color) -> &'static str {
    match c {
        Color::White => "White",
        Color::Black => "Black",
    }
}

fn status_text(game: &Game) -> String {
    let side = game.board.who_play;
    match game.status {
        Status::Ongoing if is_in_check(game.board, side) => {
            format!("{} to move - check!", color_name(side))
        }
        Status::Ongoing => format!("{} to move", color_name(side)),
        Status::Checkmate => format!("Checkmate - {} wins", color_name(side.other())),
        Status::Stalemate => "Stalemate - draw".to_owned(),
        Status::FiftyMoves => "Draw - 50-move rule".to_owned(),
        Status::InsufficientMaterial => "Draw - insufficient material".to_owned(),
        Status::Repetition => "Draw - threefold repetition".to_owned(),
    }
}

/// Square of the i-th promotion choice: a column from the promotion square towards the centre.
fn promotion_square(to: usize, i: usize) -> usize {
    if to < 8 { to + 8 * i } else { to - 8 * i }
}

/// `side` is the colour the engine played; the evaluation is shown from White's side.
fn engine_text(r: &SearchResult, side: Color) -> String {
    let white_score = match side {
        Color::White => r.score,
        Color::Black => -r.score,
    };
    let score = if white_score.abs() > MATE - 1000 {
        let moves = (MATE - white_score.abs() + 1) / 2;
        format!("{}mate in {moves}", if white_score < 0 { "-" } else { "+" })
    } else {
        format!("{:+.2}", white_score as f32 / 100.0)
    };
    let best = r.best.map(|m| m.to_string()).unwrap_or_default();
    format!("Engine: {best}  depth {}  eval {score}", r.depth)
}

fn square_xy(index: usize) -> (f32, f32) {
    (
        (index % 8) as f32 * SQUARE_SIZE,
        (index / 8) as f32 * SQUARE_SIZE,
    )
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut textures = vec![];
    for name in PIECE_FILES {
        let path = format!("pieces/{name}.png");
        let texture = load_texture(&path)
            .await
            .unwrap_or_else(|e| panic!("cannot load {path} (run from the repo root): {e}"));
        textures.push(texture);
    }
    let draw_piece = |p: Piece, index: usize| {
        let (x, y) = square_xy(index);
        draw_texture_ex(
            &textures[p.kind as usize * 2 + p.color as usize],
            x,
            y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(SQUARE_SIZE, SQUARE_SIZE)),
                ..Default::default()
            },
        )
    };

    let mut game = Game::new();
    let mut selected: Option<usize> = None;
    let mut move_pieces: Vec<usize> = vec![];
    // A pawn move waiting for the player to pick the promotion piece.
    let mut promotion: Option<Move> = None;
    // Who the engine plays; None = human vs human. M cycles through the modes.
    let mut engine_color = Some(Color::Black);
    let mut thinking: Option<Thinking> = None;
    let mut engine_info = String::new();
    loop {
        if is_key_pressed(KeyCode::R) {
            game = Game::new();
            selected = None;
            move_pieces.clear();
            promotion = None;
            thinking = None; // stops a search still running for the old game
            engine_info.clear();
        }
        if is_key_pressed(KeyCode::M) {
            engine_color = match engine_color {
                Some(Color::Black) => Some(Color::White),
                Some(Color::White) => None,
                None => Some(Color::Black),
            };
            selected = None;
            move_pieces.clear();
            promotion = None;
            thinking = None;
        }

        if thinking.is_none()
            && game.status == Status::Ongoing
            && engine_color == Some(game.board.who_play)
        {
            let (tx, rx) = mpsc::channel();
            let stop = Arc::new(AtomicBool::new(false));
            let (board, history, flag) = (game.board, game.history.clone(), stop.clone());
            thread::spawn(move || {
                let result = engine::search(&board, &history, ENGINE_MAX_DEPTH, ENGINE_TIME, &flag);
                let _ = tx.send(result); // fails only if the game was restarted
            });
            thinking = Some(Thinking { rx, stop });
        }
        if let Some(t) = &thinking {
            match t.rx.try_recv() {
                Ok(result) => {
                    thinking = None;
                    engine_info = engine_text(&result, game.board.who_play);
                    if let Some(m) = result.best {
                        game.play(m);
                    }
                }
                Err(TryRecvError::Empty) => {}
                // The engine thread panicked. Leave the board to the humans instead
                // of starting a new search (that would likely panic again) every frame.
                Err(TryRecvError::Disconnected) => {
                    thinking = None;
                    engine_color = None;
                    engine_info = "Engine crashed (see terminal)".to_owned();
                }
            }
        }

        clear_background(WHITE);
        for i in 0..64 {
            let (x, y) = square_xy(i);
            let light = (i % 8 + i / 8) % 2 == 0;
            let bg = if light { BEIGE } else { BROWN };
            draw_rectangle(x, y, SQUARE_SIZE, SQUARE_SIZE, bg);
            if game.last_move.is_some_and(|m| m.from == i || m.to == i) {
                draw_rectangle(x, y, SQUARE_SIZE, SQUARE_SIZE, LAST_MOVE);
            }
            if move_pieces.contains(&i) {
                let half = SQUARE_SIZE / 2.0;
                draw_circle(x + half, y + half, SQUARE_SIZE / 4.0, MOVE_DOT);
            }
            if selected == Some(i) {
                draw_rectangle(x, y, SQUARE_SIZE, SQUARE_SIZE, SELECTED);
            }
            if let Some(p) = game.board.get(i) {
                draw_piece(p, i);
            }
        }
        if let Some(m) = promotion {
            draw_rectangle(0.0, 0.0, BOARD_SIZE, BOARD_SIZE, SHADOW);
            for (i, kind) in PROMOTION_CHOICES.into_iter().enumerate() {
                let square = promotion_square(m.to, i);
                let (x, y) = square_xy(square);
                draw_rectangle(x, y, SQUARE_SIZE, SQUARE_SIZE, BEIGE);
                let color = game.board.who_play;
                draw_piece(Piece { kind, color }, square);
            }
        }
        let bar = BOARD_SIZE + 25.0;
        let mut status = status_text(&game);
        if thinking.is_some() {
            status.push_str("  (engine thinking...)");
        }
        draw_text(status, 10.0, bar, 28.0, BLACK);
        let mode = match engine_color {
            Some(Color::Black) => "you (White) vs engine",
            Some(Color::White) => "engine vs you (Black)",
            None => "human vs human",
        };
        let help = format!("R: restart   M: {mode}   {engine_info}");
        draw_text(help, 10.0, bar + 26.0, 20.0, BLACK);

        let (mouse_x, mouse_y) = mouse_position();
        if is_mouse_button_pressed(MouseButton::Left)
            && (0.0..BOARD_SIZE).contains(&mouse_x)
            && (0.0..BOARD_SIZE).contains(&mouse_y)
        {
            let index = (mouse_x / SQUARE_SIZE) as usize + 8 * (mouse_y / SQUARE_SIZE) as usize;
            if let Some(m) = promotion.take() {
                // A click outside the choices cancels the move.
                if let Some(i) = (0..4).find(|&i| promotion_square(m.to, i) == index) {
                    game.play(Move {
                        promotion: Some(PROMOTION_CHOICES[i]),
                        ..m
                    });
                }
            } else if let Some(from) = selected
                && move_pieces.contains(&index)
            {
                let m = Move {
                    from,
                    to: index,
                    promotion: None,
                };
                let pawn = game
                    .board
                    .get(from)
                    .is_some_and(|p| p.kind == PieceType::Pawn);
                if pawn && !(8..56).contains(&index) {
                    promotion = Some(m);
                } else {
                    game.play(m);
                }
                selected = None;
                move_pieces.clear();
            } else if game.status == Status::Ongoing
                && engine_color != Some(game.board.who_play)
                && selected != Some(index)
                && game
                    .board
                    .get(index)
                    .is_some_and(|p| p.color == game.board.who_play)
            {
                selected = Some(index);
                move_pieces = save_king(game.board, index);
            } else {
                selected = None;
                move_pieces.clear();
            }
        }
        next_frame().await;
    }
}
