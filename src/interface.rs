//! The common structures and traits.

/// An assessment of a game state from the perspective of the player whose turn it is to play.
/// Higher values mean a more favorable state.
/// A draw is defined as a score of zero.
pub type Evaluation = i32;

// These definitions ensure that they negate to each other, but it leaves
// i32::MIN as a valid value less than WORST_EVAL. Don't use this value, and
// any Strategy will panic when it tries to negate it.

/// An absolutely wonderful outcome, e.g. a win.
pub const BEST_EVAL: Evaluation = i32::MAX;
/// An absolutely disastrous outcome, e.g. a loss.
pub const WORST_EVAL: Evaluation = -BEST_EVAL;

/// Evaluates a game's positions.
pub trait Evaluator {
    /// The type of game that can be evaluated.
    type G: Game;
    /// Evaluate the non-terminal state from the persective of the player to
    /// move next.
    fn evaluate(&self, s: &<Self::G as Game>::S) -> Evaluation;
}

/// Defines how a move affects the game state.
///
/// A move is able to change initial `Game` state, as well as revert the state.
/// This allows the game tree to be searched with a constant amount of space.
pub trait Move {
    /// The type of game that the move affects.
    type G: Game;
    /// Change the state of `S` so that the move is applied.
    fn apply(&self, state: &mut <Self::G as Game>::S);
    /// Revert the state of `S` so that the move is undone.
    fn undo(&self, state: &mut <Self::G as Game>::S);
}

/// The result of playing a game until it finishes.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Winner {
    /// The player who made the last move won.
    PlayerJustMoved,
    /// Nobody won.
    Draw,
    /// The player who made the last move lost.
    ///
    /// This is uncommon, and many games (chess, checkers, tic-tac-toe, etc)
    /// do not have this possibility.
    PlayerToMove,
}

impl Winner {
    /// Canonical evaluations for end states.
    pub fn evaluate(&self) -> Evaluation {
        match *self {
            Winner::PlayerJustMoved => WORST_EVAL,
            Winner::PlayerToMove => BEST_EVAL,
            Winner::Draw => 0,
        }
    }
}

/// An optional trait for game state types to support hashing.
///
/// Strategies that cache things by game state require this.
pub trait Zobrist {
    /// Hash of the game position.
    ///
    /// Expected to be pre-calculated and cheaply updated with each apply or
    /// undo.
    fn zobrist_hash(&self) -> u64;
}

/// Defines the rules for a two-player, perfect-knowledge game.
///
/// A game ties together types for the state and moves, generates the possible
/// moves from a particular state, and determines whether a state is terminal.
pub trait Game: Sized {
    /// The type of the game state.
    type S;
    /// The type of game moves.
    type M: Move<G = Self>;

    /// Generate moves at the given state.
    fn generate_moves(state: &Self::S, moves: &mut Vec<Self::M>);

    /// Optional interface to support strategies using quiescence search.
    ///
    /// A "noisy" move is a threatening move that requires a response.
    ///
    /// The term comes from chess, where capturing a piece is considered a noisy
    /// move. Capturing a piece is often the first move out of an exchange of
    /// captures. Evaluating the board state after only the first capture can
    /// give a misleadingly high score. The solution is to continue the search
    /// among only noisy moves and find the score once the board state settles.
    fn generate_noisy_moves(_state: &Self::S, _moves: &mut Vec<Self::M>) {
        // When unimplemented, there are no noisy moves and search terminates
        // immediately.
    }

    /// Returns `Some(PlayerJustMoved)` or `Some(PlayerToMove)` if there's a winner,
    /// `Some(Draw)` if the state is terminal without a winner, and `None` if
    /// the state is non-terminal.
    fn get_winner(state: &Self::S) -> Option<Winner>;
}

/// Defines a method of choosing a move for the current player.
pub trait Strategy<G: Game> {
    fn choose_move(&mut self, state: &G::S) -> Option<G::M>;
}
