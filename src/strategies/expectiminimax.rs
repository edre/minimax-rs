
use rand::seq::SliceRandom;

use crate::{BEST_EVAL, Evaluation, Evaluator, Game, StochasticGame, Strategy, TurnBasedGame, TurnBasedGameEvaluator, WORST_EVAL};
use crate::util::{AppliedMove, MovePool};

pub struct ExpectiMinimax<E>
where
    E: Evaluator,
    E::G: StochasticGame+TurnBasedGame,
{
    max_depth: u8,
    move_pool: MovePool<<E::G as Game>::M>,
    rng: rand::rngs::ThreadRng,
    pub prev_value: Evaluation,
    pub eval: E,
}

impl<E> Strategy<E::G> for ExpectiMinimax<E>
where
    E: TurnBasedGameEvaluator,
    E::G: StochasticGame+TurnBasedGame,
    <E::G as Game>::S: Clone,
    <E::G as Game>::M: Copy
{
    fn choose_move(&mut self, s: &<E::G as Game>::S) -> Option<<E::G as Game>::M> {
        if self.max_depth == 0 {
            return None;
        }
        let mut best = WORST_EVAL;
        let mut moves = self.move_pool.alloc();
        let player = E::G::current_player(s);
        self.eval.set_player_on_trait(player);
        if E::G::generate_moves(s, &mut moves).is_some() {
            return None;
        }
        // Randomly permute order that we look at the moves.
        // We'll pick the first best score from this list.
        moves.shuffle(&mut self.rng);

        let mut best_move = *moves.first()?;
        let mut s_clone = s.clone();
        for &m in moves.iter() {
            // determine value for this move
            let mut new = AppliedMove::<E::G>::new(&mut s_clone, m);
            let value = self.expectiminimax(&mut new, self.max_depth - 1, player, WORST_EVAL, BEST_EVAL);
            // Strictly better than any move found so far.
            if value > best {
                best = value;
                best_move = m;
            }
        }
        self.move_pool.free(moves);
        self.prev_value = best;
        Some(best_move)
    }
    fn set_max_depth(&mut self, depth: u8) {
        self.max_depth = depth;
    }
    fn set_timeout(&mut self, _timeout: std::time::Duration) {
        self.max_depth = u8::MAX;
    }
    fn set_depth_or_timeout(&mut self, _depth: u8, _timeout: std::time::Duration) {
        self.max_depth = _depth;
    }
}
impl<E> ExpectiMinimax<E>
where
    E: TurnBasedGameEvaluator,
    E::G: StochasticGame+TurnBasedGame,
{
    pub fn new(evaluator: E, depth: u8) -> Self {
        Self {
            max_depth: depth,
            move_pool: MovePool::<_>::default(),
            rng: rand::rng(),
            prev_value: 0,
            eval: evaluator,
        }
    }
    pub fn expectiminimax(&mut self,
        s: &mut <E::G as Game>::S,
        depth: u8,
        to_choose_player: i8,
        mut alpha: Evaluation, mut beta: Evaluation
    ) -> Evaluation
    {
        if depth == 0 {
            if let Some(winner) = E::G::get_winner(s) {
                // This could be inacurate. Does the player changed after applying the winning move??
                return if E::G::current_player(s) == to_choose_player {
                    match winner {
                        crate::Winner::PlayerJustMoved => BEST_EVAL,
                        crate::Winner::Draw => 0,
                        crate::Winner::PlayerToMove => WORST_EVAL,
                    }
                } else {
                    match winner {
                        crate::Winner::PlayerJustMoved => WORST_EVAL,
                        crate::Winner::Draw => 0,
                        crate::Winner::PlayerToMove => BEST_EVAL,
                    }
                };
            }
            return self.eval.evaluate(s);
        }
        let mut moves = self.move_pool.alloc();
        if let Some(winner) = E::G::generate_moves(s, &mut moves) {
            return if E::G::current_player(s) == to_choose_player {
                    match winner {
                        crate::Winner::PlayerJustMoved => BEST_EVAL,
                        crate::Winner::Draw => 0,
                        crate::Winner::PlayerToMove => WORST_EVAL,
                    }
                } else {
                    match winner {
                        crate::Winner::PlayerJustMoved => WORST_EVAL,
                        crate::Winner::Draw => 0,
                        crate::Winner::PlayerToMove => BEST_EVAL,
                    }
                };
        }

        if moves.is_empty() {
            return WORST_EVAL;
        }
        let best_score = if E::G::is_random_move(s) {
            let mut value = 0.0;

            for m in moves.iter() {
                let p = E::G::get_probability(s, *m);
                let mut new = AppliedMove::<E::G>::new(s, *m);
                let score = self.expectiminimax(&mut new, depth - 1, to_choose_player, alpha, beta);
                value += p * score as f32;
            }
            
            value.round() as Evaluation
        } else if E::G::current_player(s) == to_choose_player {
            let mut best_score = WORST_EVAL;

            for m in moves.iter() {
                let mut new = AppliedMove::<E::G>::new(s, *m);
                let score = self.expectiminimax(&mut new, depth - 1, to_choose_player, alpha, beta);
                if score > best_score {
                    best_score = score;
                }
                if best_score >= beta {
                    break;
                }
                alpha = alpha.max(best_score)
            }
            best_score
        } else {// if state.current_player() == to_choose_player.opponent() {
            debug_assert!(<E::G as TurnBasedGame>::current_player(s) != to_choose_player && E::G::current_player(s) != 0);
            let mut best_score = BEST_EVAL;

            for m in moves.iter() {
                let mut new = AppliedMove::<E::G>::new(s, *m);
                let score = self.expectiminimax(&mut new, depth - 1, to_choose_player, alpha, beta);
                if score < best_score {
                    best_score = score;
                }
                if best_score <= alpha {
                    break;
                }
                beta = beta.min(best_score);
            }
            best_score
        };
        self.move_pool.free(moves);
        best_score
    }
}

#[cfg(test)]
mod tests {
    use crate::{Evaluator, Game, Strategy, TurnBasedGameEvaluator, strategies::expectiminimax::ExpectiMinimax};


    #[derive(Debug, Default, Clone)]
    struct DumbGame {
        choice: Option<DiceChoice>,
        score:i16,
        to_move:bool
    }
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    enum DiceChoice {
        D4, D6, D8
    }
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    enum Move {
        DiceChoice(DiceChoice),
        Random(i16)
    }
    impl crate::Game for DumbGame {
        type S=DumbGame;
        type M=Move;

        fn generate_moves(state: &Self::S, moves: &mut Vec<Self::M>) -> Option<crate::Winner> {
            match state.choice {
                Some(d) => {
                    moves.push(Move::Random(1));
                    moves.push(Move::Random(2));
                    moves.push(Move::Random(3));
                    moves.push(Move::Random(4));
                    match d {
                            DiceChoice::D4 => {},
                            DiceChoice::D6 => {
                                moves.push(Move::Random(5));
                                moves.push(Move::Random(6));
                            },
                            DiceChoice::D8 => {
                                moves.push(Move::Random(5));
                                moves.push(Move::Random(6));
                                moves.push(Move::Random(7));
                                moves.push(Move::Random(8));
                            },
                        }
                }
                None => {
                    moves.push(Move::DiceChoice(DiceChoice::D4));
                    moves.push(Move::DiceChoice(DiceChoice::D6));
                    moves.push(Move::DiceChoice(DiceChoice::D8));
                }
            }
            None
        }
    
        fn apply(state: &mut Self::S, m: Self::M) -> Option<Self::S> {
            let mut clone = state.clone();
            match m {
                Move::DiceChoice(dice_choice) => clone.choice=Some(dice_choice),
                Move::Random(r) => {
                    clone.score += r;
                    clone.choice = None;
                    clone.to_move = !clone.to_move;
                },
            }
            Some(clone)
        }

        fn get_winner(_state: &Self::S) -> Option<crate::Winner> {
            None
        }
    }
    impl crate::TurnBasedGame for DumbGame {
        fn current_player(state: &Self::S) -> i8 {
            if state.to_move {-1} else {1}
        }
    }
    impl crate::StochasticGame for DumbGame {
        fn is_random_move(state: &Self::S) -> bool {
            state.choice.is_some()
        }
    
        fn get_probability(state: &Self::S, _mv: Self::M) -> f32 {
            match state.choice.unwrap() {
                DiceChoice::D4 => 1.0/4.0,
                DiceChoice::D6 => 1.0/6.0,
                DiceChoice::D8 => 1.0/8.0,
            }
        }
    }
    #[derive(Debug, Default)]
    struct Eval(bool);
    impl Evaluator for Eval {
        type G=DumbGame;
    
        fn evaluate(&self, s: &<Self::G as crate::Game>::S) -> crate::Evaluation {
            if self.0 { -s.score * 10 } else { s.score * 10 }
        }
    }
    impl TurnBasedGameEvaluator for Eval {
        fn set_player_on_trait(&mut self, p: i8) {
            self.0 = p != 1;
        }
    }
    #[test]
    fn test() {
        let mut strat=ExpectiMinimax::new(Eval::default(), 8);
        let mut i = 0;
        let mut s=DumbGame::default();
        while i < 10 {
            let m=strat.choose_move(&s);
            if !s.to_move {
                assert_eq!(m, Some(Move::DiceChoice(DiceChoice::D8)));
            } else {
                assert_eq!(m, Some(Move::DiceChoice(DiceChoice::D4)));
            }
            s=DumbGame::apply(&mut s, m.unwrap()).unwrap();
            let m=strat.choose_move(&s);
            if !s.to_move {
                assert_eq!(m, Some(Move::Random(8)));
            } else {
                assert_eq!(m, Some(Move::Random(1)));
            }
            
            s=DumbGame::apply(&mut s, m.unwrap()).unwrap();
            i+=1;
        }
    }
}