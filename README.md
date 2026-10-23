# ChessBoard

Chess in Rust with my own engine and a GUI made with [macroquad](https://macroquad.rs). No chess crate, the board is an array of 64 `Option<Piece>` and the rules are in `src/moves.rs`.

```sh
cargo run --release
```

Run it from the repo root, the piece images are loaded from `pieces/`. And keep `--release`: in debug the engine is more than 10 times slower, so it searches less deep in the same time and plays a lot worse.

You play White against the engine by default. Click one of your pieces to see its legal moves as blue dots, then click a dot to play. To castle, click the square where the king lands. On a promotion a column with queen, rook, bishop and knight shows up and you click the one you want. R starts a new game and M changes the mode (engine plays White, then human vs human, then back). The board doesn't flip when you play Black and there is no undo.

The engine thinks about 1 s per move (`ENGINE_TIME` in `src/main.rs`) in its own thread so the window doesn't freeze. The bar at the bottom shows its last move, the depth it reached and its eval from White's side.

## Engine

It's a negamax with alpha-beta pruning and iterative deepening, in `src/engine.rs`. The eval is material plus the piece-square tables from the Simplified Evaluation Function, with another king table for the endgame. Captures are tried first (MVV-LVA), then promotions, and the best move of the previous depth goes first at the next one. At depth 0 a quiescence search keeps going with captures only, so it doesn't evaluate in the middle of a trade. If the side to move is in check there it searches all its moves instead, so it can see a mate right at the horizon. A position that already happened counts as a draw.

From the start position, in release on my machine, it gets to depth 6 in 1 s at about 1 million nodes per second.

There's no transposition table or opening book, and the move generation allocates a `Vec` for every position, so it doesn't get much deeper than that. The quiescence search also doesn't look for mates and skips underpromotions.

## Tests

```sh
cargo test --release
cargo test --release -- --ignored --nocapture
```

The second one runs the slow tests: perft depth 5 from the start position (about 2 s) and the speed benchmark that prints the depth and nodes per second. Perft counts every leaf of the legal move tree and checks it against the known values. It's tested from the start position up to depth 5 (4,865,609 nodes), Kiwipete up to depth 3 and two other standard positions. The other tests cover FEN, castling, en passant, promotions and the draw rules, and the engine is checked on positions like mate in 2 or a hanging queen.
