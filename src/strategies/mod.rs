//! Strategy implementations.

pub mod iterative;
#[cfg(not(target_arch = "wasm32"))]
pub mod mcts;
pub mod negamax;
pub mod random;
#[cfg(not(target_arch = "wasm32"))]
pub mod ybw;

mod common;
#[cfg(not(target_arch = "wasm32"))]
mod sync_util;
mod table;
