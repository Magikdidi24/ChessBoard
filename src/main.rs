use macroquad::{
    color::{BEIGE, BROWN, WHITE}, input::{MouseButton, is_mouse_button_pressed, mouse_position}, math::vec2, miniquad::window, shapes::{draw_circle, draw_rectangle}, texture::{DrawTextureParams, draw_texture_ex, load_texture}, window::{clear_background, next_frame}
};

const SQUARE_SIZE: f32 = 100.0;
const KING_MOVES: [[(isize, isize); 1]; 8] = [
    [(-1, -1)], [(-1, 0)], [(-1, 1)],
    [( 0, -1)],            [( 0, 1)],
    [( 1, -1)], [( 1, 0)], [( 1, 1)],
];
const ROOK_MOVES: [[(isize, isize); 7]; 4] = [
    [(-1, 0), (-2, 0), (-3, 0), (-4, 0), (-5, 0), (-6, 0), (-7, 0)], // up
    [( 1, 0), ( 2, 0), ( 3, 0), ( 4, 0), ( 5, 0), ( 6, 0), ( 7, 0)], // down
    [(0, -1), (0, -2), (0, -3), (0, -4), (0, -5), (0, -6), (0, -7)], // left
    [(0,  1), (0,  2), (0,  3), (0,  4), (0,  5), (0,  6), (0,  7)], // right
];
const BISHOP_MOVES: [[(isize, isize); 7]; 4] = [
    [(-1, -1), (-2, -2), (-3, -3), (-4, -4), (-5, -5), (-6, -6), (-7, -7)], // up-left
    [( 1, -1), ( 2, -2), ( 3, -3), ( 4, -4), ( 5, -5), ( 6, -6), ( 7, -7)], // up-right
    [(-1,  1), (-2,  2), (-3,  3), (-4,  4), (-5,  5), (-6,  6), (-7,  7)], // down-left
    [( 1,  1), ( 2,  2), ( 3,  3), ( 4,  4), ( 5,  5), ( 6,  6), ( 7,  7)], // down-right
];

const KNIGHT_MOVES: [[(isize, isize); 1]; 8] = [
    [(-1, -2)],[(-2, -1)],[(1, -2)],[(-2, 1)],[(2, -1)],[(-1, 2)],[(1, 2)],[(2, 1)],
];

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


#[derive(Clone, Copy, PartialEq)]
enum PieceType { King, Pawn, Knight, Bishop, Rook, Queen }

#[derive(Clone, Copy, PartialEq)]
enum Color { White, Black }

#[derive(Clone, Copy, PartialEq)]
struct Piece {
    kind: PieceType,
    color: Color,
}
#[derive(Clone,Copy)]
struct Board {
    square: [Option<Piece>; 64],
    black_play : bool,
    white_play : bool,
    can_rook_black_small : bool,
    can_rook_black_big : bool,
    can_rook_white_small : bool,
    can_rook_white_big : bool,
    number_of_half_moves : u64,
    number_of_moves : u64,
}

impl Board {
    fn new() -> Self {
        Board { 
            square: [const { None }; 64], 
            black_play: false,
            white_play: true,
            can_rook_black_big:true,
            can_rook_white_small:true,
            can_rook_white_big:true,
            can_rook_black_small:true,
            number_of_half_moves:0,
            number_of_moves:1,
        }
    }
    fn set(&mut self, index: usize, piece: Piece) {
        self.square[index] = Some(piece);
    }
    fn get(&self, index: usize) -> Option<Piece> {
        self.square[index]
    }
}

fn fen_to_board(mut board: Board, s: String) -> Board{
    let mut i: u8=0;
    let mut space: i32 = 0;
    board.can_rook_black_big=false; 
    board.can_rook_white_small=false;
    board.can_rook_white_big=false;
    board.can_rook_black_small=false; 
    for c in s.chars(){
        if c == ' '{ space+=1; continue; }
        if space==1 {
            if c == 'b' { board.black_play = true;  board.white_play = false; }
            if c == 'w' { board.black_play = false; board.white_play = true;  }
            continue;
        }
        if space==2{
            if c == 'K'{board.can_rook_white_small=true;}
            if c == 'Q'{board.can_rook_white_big=true;}
            if c == 'k'{board.can_rook_black_small=true;}
            if c == 'q'{board.can_rook_black_big=true;}
            continue;
        }
        if space==3 { /*en passant*/ continue; }
        if space==4 { board.number_of_half_moves=c.to_ascii_lowercase() as u64; continue; }
        if space==5 { board.number_of_moves=c.to_ascii_lowercase() as u64; continue; }
        

        if c.is_ascii_digit() { i+= c as u8 - b'0'; }
        
        match c {
            'r' => { board.set(i.into(), Piece { kind: PieceType::Rook,   color: Color::Black }); i += 1; }
            'R' => { board.set(i.into(), Piece { kind: PieceType::Rook,   color: Color::White }); i += 1; }
            'b' => { board.set(i.into(), Piece { kind: PieceType::Bishop, color: Color::Black }); i += 1; }
            'B' => { board.set(i.into(), Piece { kind: PieceType::Bishop, color: Color::White }); i += 1; }
            'n' => { board.set(i.into(), Piece { kind: PieceType::Knight, color: Color::Black }); i += 1; }
            'N' => { board.set(i.into(), Piece { kind: PieceType::Knight, color: Color::White }); i += 1; }
            'q' => { board.set(i.into(), Piece { kind: PieceType::Queen,  color: Color::Black }); i += 1; }
            'Q' => { board.set(i.into(), Piece { kind: PieceType::Queen,  color: Color::White }); i += 1; }
            'k' => { board.set(i.into(), Piece { kind: PieceType::King,   color: Color::Black }); i += 1; }
            'K' => { board.set(i.into(), Piece { kind: PieceType::King,   color: Color::White }); i += 1; }
            'p' => { board.set(i.into(), Piece { kind: PieceType::Pawn,   color: Color::Black }); i += 1; }
            'P' => { board.set(i.into(), Piece { kind: PieceType::Pawn,   color: Color::White }); i += 1; }
            _ => {}
        }
    }
    board
}

fn helper_check_legal_move<const T: usize,const N: usize>(index:usize, board:Board, mut mov: Vec<usize>, moves_pieces: [[(isize, isize); T]; N],p :Piece) -> Vec<usize>{
    let file = (index % 8) as isize;
    let rank = (index / 8) as isize;
    for directions in moves_pieces{
        for (df, dr) in directions {
            let nf = file + df;
            let nr = rank + dr;
            if !(0..=7).contains(&nf) || !(0..=7).contains(&nr) {
                continue;
            }
            let n_idx = (nr * 8 + nf) as usize;
            match board.get(n_idx) {
                Some(other) if other.color != p.color => { mov.push(n_idx); break; }
                None => mov.push(n_idx),
                _ => break
            }
        }
    }
    mov
}

fn pawn_moves(index:usize, board:Board, mut mov: Vec<usize>,p :Piece) -> Vec<usize>{
    let file = (index % 8) as isize;
    let rank = (index / 8) as isize;
    let (dir, starting_rank): (isize, isize) = match p.color {
        Color::White => (-1, 6),
        Color::Black => ( 1, 1),
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
    if (0..=7).contains(&nr) && file > 0 {
        let left_diag = (nr * 8 + (file - 1)) as usize;
        if let Some(other_p) = board.get(left_diag) && other_p.color != p.color{
            mov.push(left_diag);
        }
    }
    if (0..=7).contains(&nr) && file < 7 {
        let right_diag = (nr * 8 + (file + 1)) as usize;
        if let Some(other_p) = board.get(right_diag) && other_p.color != p.color{
            mov.push(right_diag);
        }
    }
    mov
}


fn check_legal_move(board :Board, index:usize) -> Vec<usize>{
    let mut mov: Vec<usize> = vec![];
    if let Some(p) = board.get(index) {
        mov = match p.kind {
            PieceType::Knight => helper_check_legal_move(index, board, mov,KNIGHT_MOVES, p),
            PieceType::King   => {
                let mut m = helper_check_legal_move(index, board, mov,KING_MOVES,   p);
                if p.color == Color::White && index == 60 { 
                    if board.can_rook_white_small && 
                        board.square[61].is_none() &&
                        board.square[62].is_none() &&
                        !is_case_attack(61, board, Color::White) &&
                        !is_case_attack(62, board, Color::White) &&
                        board.square[63] == (Some(Piece { kind: PieceType::Rook, color: Color::White }))
                    {
                        m.push(63);
                    }
                    if board.can_rook_white_big &&
                        board.square[59].is_none() &&
                        board.square[58].is_none() &&
                        board.square[57].is_none() &&
                        !is_case_attack(60, board, Color::Black) &&
                        !is_case_attack(59, board, Color::White) &&
                        !is_case_attack(58, board, Color::White) && 
                        // En revanche la tour, ainsi que sa case adjacente dans le cas du grand roque, peuvent être menacées
                        board.square[56] == (Some(Piece { kind: PieceType::Rook, color: Color::White })){
                        m.push(56);
                    }
                }
                if p.color == Color::Black && index == 4 {
                    if board.can_rook_black_small &&
                        board.square[5].is_none() &&
                        board.square[6].is_none() &&
                        !is_case_attack(5, board, Color::Black) &&
                        !is_case_attack(6, board, Color::Black) &&
                        board.square[7] == (Some(Piece { kind: PieceType::Rook, color: Color::Black })){
                        m.push(7);
                    }
                    if board.can_rook_black_big &&
                        board.square[3].is_none() &&
                        board.square[2].is_none() &&
                        board.square[1].is_none() &&
                        !is_case_attack(4, board, Color::Black) &&
                        !is_case_attack(3, board, Color::Black) &&
                        !is_case_attack(2, board, Color::Black) &&
                        // En revanche la tour, ainsi que sa case adjacente dans le cas du grand roque, peuvent être menacées
                        board.square[0] == (Some(Piece { kind: PieceType::Rook, color: Color::Black })){
                        m.push(0);
                    }
                }
                m
            },
            PieceType::Rook   => helper_check_legal_move(index, board, mov,ROOK_MOVES,   p),
            PieceType::Bishop => helper_check_legal_move(index, board, mov,BISHOP_MOVES, p),
            PieceType::Queen  => helper_check_legal_move(index, board, mov,QUEEN_MOVES,  p),
            PieceType::Pawn => pawn_moves(index, board, mov, p),
        };
    }
    mov
}

fn attacks_from(index: usize, board: Board) -> Vec<usize> {
    let mut mov: Vec<usize> = vec![];
    if let Some(p) = board.get(index) {
        mov = match p.kind {
            PieceType::Knight => helper_check_legal_move(index, board, mov, KNIGHT_MOVES, p),
            PieceType::King   => helper_check_legal_move(index, board, mov, KING_MOVES,   p),
            PieceType::Rook   => helper_check_legal_move(index, board, mov, ROOK_MOVES,   p),
            PieceType::Bishop => helper_check_legal_move(index, board, mov, BISHOP_MOVES, p),
            PieceType::Queen  => helper_check_legal_move(index, board, mov, QUEEN_MOVES,  p),
            PieceType::Pawn   => pawn_attacks(index, mov, p),
        };
    }
    mov
}

fn pawn_attacks(index: usize, mut mov: Vec<usize>, p: Piece) -> Vec<usize> {
    let file = (index % 8) as isize;
    let rank = (index / 8) as isize;
    let dir: isize = match p.color {
        Color::White => -1,
        Color::Black =>  1,
    };
    let nr = rank + dir;
    if !(0..=7).contains(&nr) {
        return mov;
    }

    if file > 0 {
        let left = (nr * 8 + (file - 1)) as usize;
        mov.push(left);
    }
    if file < 7 {
        let right = (nr * 8 + (file + 1)) as usize;
        mov.push(right);
    }
    mov
}

fn is_case_attack(index: usize, board: Board, color: Color) -> bool {
    for i in 0..64 {
        if let Some(piece) = board.get(i)
            && piece.color != color
            && attacks_from(i, board).contains(&index)
        {
            return true;
        }
    }
    false
}

fn is_in_check(board :Board,color :Color) -> bool{
    for i in 0..64 {
        if let Some(piece) = board.get(i)&& piece.kind == PieceType::King && piece.color == color{
            return is_case_attack(i, board, color)
        }
    }
    false
}

fn save_king(board :Board,index: usize) -> Vec<usize> {
    let pseudo: Vec<usize> = check_legal_move(board, index);
    let mut legal: Vec<usize> = vec![];
    let me = match board.get(index) {
        Some(p) => p.color,
        None => return legal,
    };
    for to in pseudo{
        let mut tmp: Board = board;
        tmp.square[to] = tmp.square[index];
        tmp.square[index] = None; 
        if !is_in_check(tmp, me) {
            legal.push(to);
        }
    }
    legal
}

#[macroquad::main("Chess")]
async fn main() {
    let b_b = load_texture("pieces/bB.png").await.unwrap();
    let b_k = load_texture("pieces/bK.png").await.unwrap();
    let b_n = load_texture("pieces/bN.png").await.unwrap();
    let b_p = load_texture("pieces/bP.png").await.unwrap();
    let b_q = load_texture("pieces/bQ.png").await.unwrap();
    let b_r = load_texture("pieces/bR.png").await.unwrap();
    let w_b = load_texture("pieces/wB.png").await.unwrap();
    let w_k = load_texture("pieces/wK.png").await.unwrap();
    let w_n = load_texture("pieces/wN.png").await.unwrap();
    let w_p = load_texture("pieces/wP.png").await.unwrap();
    let w_q = load_texture("pieces/wQ.png").await.unwrap();
    let w_r = load_texture("pieces/wR.png").await.unwrap();

    let mut board = Board::new();
    let mut who_play: Color = Color::White;
    let s = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_string();
    board = fen_to_board(board,s);
    let mut selected: Option<usize> = None;
    let mut move_pieces: Option<Vec<usize>> = None;
    loop {
        clear_background(WHITE);
        window::set_window_size((SQUARE_SIZE*8.0) as u32, (SQUARE_SIZE*8.0) as u32);

        for i in 0..64 {
            let file = i % 8;
            let rank = i / 8;
            let x = (i % 8) as f32 * SQUARE_SIZE;
            let y = rank as f32 * SQUARE_SIZE;

            let light = (file + rank) % 2 == 0;
            let bg = if light { BEIGE } else { BROWN };
            draw_rectangle(x, y, SQUARE_SIZE, SQUARE_SIZE, bg);
            if let Some(ref arr) = move_pieces{
                for j in arr{
                    if *j == i {draw_circle(x+SQUARE_SIZE/2.0, y+SQUARE_SIZE/2.0, SQUARE_SIZE/4.0, macroquad::color::Color::new(0.0, 0.0, 1.0, 0.4));}
                }
                
            }
            if selected == Some(i) {
                draw_rectangle(x, y, SQUARE_SIZE, SQUARE_SIZE,
                    macroquad::color::Color::new(0.0, 1.0, 0.0, 0.4));
            }
            if let Some(p) = board.get(i) {
                let texture = match (p.kind, p.color) {
                    (PieceType::King,   Color::White) => &w_k,
                    (PieceType::King,   Color::Black) => &b_k,
                    (PieceType::Pawn,   Color::White) => &w_p,
                    (PieceType::Pawn,   Color::Black) => &b_p,
                    (PieceType::Knight, Color::White) => &w_n,
                    (PieceType::Knight, Color::Black) => &b_n,
                    (PieceType::Bishop, Color::White) => &w_b,
                    (PieceType::Bishop, Color::Black) => &b_b,
                    (PieceType::Rook,   Color::White) => &w_r,
                    (PieceType::Rook,   Color::Black) => &b_r,
                    (PieceType::Queen,  Color::White) => &w_q,
                    (PieceType::Queen,  Color::Black) => &b_q,
                };
                draw_texture_ex(
                    texture, x, y, WHITE,
                    DrawTextureParams {
                        dest_size: Some(vec2(SQUARE_SIZE, SQUARE_SIZE)),
                        ..Default::default()
                    },
                )
            }
        }
        if is_mouse_button_pressed(MouseButton::Left){
            let mouse_pos_start = mouse_position();
            let a_to_x = (mouse_pos_start.0 / SQUARE_SIZE) as usize;
            let b_to_y = (mouse_pos_start.1 / SQUARE_SIZE) as usize;
            let index_start = a_to_x+8*b_to_y;
            if selected.is_none() && board.square[index_start].is_some() && let Some(a) = board.get(index_start) && a.color == who_play{
                selected = Some(index_start);
                if move_pieces.is_none(){
                    move_pieces = Some(save_king(board, index_start));
                }
            }
            else{
                if let Some(index_start2) = selected{
                    let mouse_pos_end = mouse_position();
                    let a_to_x_end = (mouse_pos_end.0 / SQUARE_SIZE) as usize;
                    let b_to_y_end = (mouse_pos_end.1 / SQUARE_SIZE) as usize;
                    let index_end = a_to_x_end+8*b_to_y_end;
                    if index_end == index_start2{selected=None;move_pieces=None;}
                    else{
                        let piece = board.square[index_start2];
                        if piece == Some(Piece { kind: PieceType::King, color: Color::White }) && matches!(&move_pieces, Some(c) if c.contains(&index_end))
                        {
                            if index_end == 63 {
                                board.square[62] = Some(Piece { kind: PieceType::King, color: Color::White });
                                board.square[61] = Some(Piece { kind: PieceType::Rook, color: Color::White });
                                board.square[index_end] = None;
                            }
                            else if index_end == 56 {
                                board.square[58] = Some(Piece { kind: PieceType::King, color: Color::White });
                                board.square[59] = Some(Piece { kind: PieceType::Rook, color: Color::White });
                                board.square[index_end] = None;
                            }
                            else {board.square[index_end] = piece;}

                            board.square[index_start2] = None;
                            selected=None;
                            move_pieces=None;
                            board.can_rook_white_big=false;
                            board.can_rook_white_small=false;
                        }
                        else if piece == Some(Piece { kind: PieceType::King, color: Color::Black }) && matches!(&move_pieces, Some(c) if c.contains(&index_end))
                        {
                            if index_end == 7 {
                                board.square[6] = Some(Piece { kind: PieceType::King, color: Color::Black });
                                board.square[5] = Some(Piece { kind: PieceType::Rook, color: Color::Black });
                                board.square[index_end] = None;
                            }
                            else if index_end == 0 {
                                board.square[2] = Some(Piece { kind: PieceType::King, color: Color::Black });
                                board.square[3] = Some(Piece { kind: PieceType::Rook, color: Color::Black });
                                board.square[index_end] = None;
                            }
                            else {board.square[index_end] = piece;}

                            board.square[index_start2] = None;
                            selected=None;
                            move_pieces=None;
                            board.can_rook_black_big=false;
                            board.can_rook_black_small=false;
                        }
                        else if let Some(ref coor) = move_pieces && coor.contains(&index_end) {
                            board.square[index_end] = piece;
                            board.square[index_start2] = None;
                            selected=None;
                            move_pieces=None;
                        }
                        if piece == (Some(Piece { kind: PieceType::Rook, color: Color::White })) && index_start2 == 63{
                            board.can_rook_white_small=false;
                        }
                        else if piece == (Some(Piece { kind: PieceType::Rook, color: Color::White })) && index_start2 == 56{
                            board.can_rook_white_big=false;
                        }
                        else if piece == (Some(Piece { kind: PieceType::Rook, color: Color::Black })) && index_start2 == 7{
                            board.can_rook_black_small=false;
                        }
                        else if piece == (Some(Piece { kind: PieceType::Rook, color: Color::Black })) && index_start2 == 0{
                            board.can_rook_black_big=false;
                        }
                        if index_end == 63 { board.can_rook_white_small = false; }
                        if index_end == 56 { board.can_rook_white_big   = false; }
                        if index_end == 7  { board.can_rook_black_small = false; }
                        if index_end == 0  { board.can_rook_black_big   = false; }
                        board.number_of_half_moves+=1;
                    }
                }
            }   
        }
        if who_play == Color::White && !board.number_of_half_moves.is_multiple_of(2){ 
            who_play = Color::Black;
        }else if who_play == Color::Black && board.number_of_half_moves.is_multiple_of(2){
            board.number_of_moves+=1;
            who_play = Color::White;
        }
        next_frame().await;
    }
}