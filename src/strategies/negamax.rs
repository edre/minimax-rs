//! An implementation of Negamax.
//!
//! With only the basic alpha-pruning implemented. This picks randomly among
//! the "best" moves, so that it's non-deterministic.

use super::super::interface::*;
use super::super::util::*;
use super::common::*;
use rand::seq::SliceRandom;
use std::cmp::max;

pub struct Negamax<E: Evaluator> {
    max_depth: u8,
    move_pool: MovePool<<E::G as Game>::M>,
    rng: rand::rngs::ThreadRng,
    prev_value: Evaluation,
    eval: E,
}

impl<E: Evaluator> Negamax<E> {
    pub fn new(eval: E, depth: u8) -> Negamax<E> {
        Negamax {
            max_depth: depth,
            move_pool: MovePool::<_>::default(),
            rng: rand::rng(),
            prev_value: 0,
            eval,
        }
    }

    #[doc(hidden)]
    pub fn root_value(&self) -> Evaluation {
        unclamp_value(self.prev_value)
    }

    fn negamax(
        &mut self, s: &mut <E::G as Game>::S, depth: u8, mut alpha: Evaluation, beta: Evaluation,
    ) -> Evaluation
    where
        <<E as Evaluator>::G as Game>::M: Copy,
    {
        if let Some(winner) = E::G::get_winner(s) {
            return winner.evaluate();
        }
        if depth == 0 {
            return self.eval.evaluate(s);
        }
        let mut moves = self.move_pool.alloc();
        E::G::generate_moves(s, &mut moves);
        let mut best = WORST_EVAL;
        for m in moves.iter() {
            let mut new = AppliedMove::<E::G>::new(s, *m);
            let value = -self.negamax(&mut new, depth - 1, -beta, -alpha);
            best = max(best, value);
            alpha = max(alpha, value);
            if alpha >= beta {
                break;
            }
        }
        self.move_pool.free(moves);
        clamp_value(best)
    }
}

impl<E: Evaluator> Strategy<E::G> for Negamax<E>
where
    <E::G as Game>::S: Clone,
    <E::G as Game>::M: Copy,
{
    fn choose_move(&mut self, s: &<E::G as Game>::S) -> Option<<E::G as Game>::M> {
        if self.max_depth == 0 {
            return None;
        }
        if E::G::get_winner(s).is_some() {
            return None;
        }
        let mut best = WORST_EVAL;
        let mut moves = self.move_pool.alloc();
        E::G::generate_moves(s, &mut moves);
        // Randomly permute order that we look at the moves.
        // We'll pick the first best score from this list.
        moves.shuffle(&mut self.rng);

        let mut best_move = *moves.first()?;
        let mut s_clone = s.clone();
        for &m in moves.iter() {
            // determine value for this move
            let mut new = AppliedMove::<E::G>::new(&mut s_clone, m);
            let value = -self.negamax(&mut new, self.max_depth - 1, WORST_EVAL, -best);
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
}
