use rand::seq::SliceRandom;
use rand::Rng;

use crate::game::{GameState, TurnAction, TurnPhase, MAX_NUM_CHAINS};

pub trait Agent {
    fn choose_action(&self, game: &GameState) -> TurnAction;
}

pub fn create_agent(difficulty: usize) -> Box<dyn Agent + Send> {
    match difficulty {
        // Random (valid) actions.
        0 => Box::<RandomAgent>::default(),
        // Simple heuristics on top of random actions.
        _ => Box::<BasicAgent>::default(),
    }
}

#[derive(Default)]
struct RandomAgent;
impl Agent for RandomAgent {
    fn choose_action(&self, game: &GameState) -> TurnAction {
        let mut rng = rand::thread_rng();
        match &game.turn_state.phase {
            TurnPhase::PlaceTile(tile_inds) => {
                let tile_idx = tile_inds.choose(&mut rng).unwrap();
                TurnAction::PlaceTile(*tile_idx)
            }
            TurnPhase::CreateChain(_, chain_inds) => {
                let chain_idx = chain_inds.choose(&mut rng).unwrap();
                TurnAction::CreateChain(*chain_idx)
            }
            TurnPhase::PickWinningChain(choices, _) => {
                let chain_idx = choices.choose(&mut rng).unwrap();
                TurnAction::PickWinningChain(*chain_idx)
            }
            TurnPhase::ResolveMerger(_winner_idx, loser_inds, player_idx) => {
                let loser_idx = loser_inds[0];
                let loser_shares = game.players[*player_idx].stocks[loser_idx];
                // TODO: Enable trading shares as well as selling them.
                let num_sold = rng.gen_range(0..=loser_shares);
                TurnAction::ResolveMerger(num_sold, 0)
            }
            TurnPhase::BuyStock(buyable_amounts) => {
                let my_cash = game.players[game.turn_state.player].cash;
                // Add one index for each buyable share.
                let mut buyable_shares = Vec::new();
                for (i, &amount) in buyable_amounts.iter().enumerate() {
                    let price = game.stock_price(i);
                    if amount > 0 && price < my_cash {
                        let max_shares = (my_cash / price).min(3).min(amount);
                        for _ in 0..max_shares {
                            buyable_shares.push(i);
                        }
                    }
                }
                // Pick up to 3 random buyable shares and buy them, unless we
                // run out of cash first.
                let mut buy_order = [0; MAX_NUM_CHAINS];
                let mut buy_price = 0;
                for &chain_idx in buyable_shares.choose_multiple(&mut rng, 3) {
                    buy_price += game.stock_price(chain_idx);
                    if buy_price > my_cash {
                        break;
                    }
                    buy_order[chain_idx] += 1;
                }
                TurnAction::BuyStock(buy_order)
            }
            TurnPhase::GameOver(_) => TurnAction::PlaceTile(0),
        }
    }
}

fn chain_with_most_shares(game: &GameState, chain_inds: &[usize]) -> usize {
    let my_stocks = &game.players[game.turn_state.player].stocks;
    *chain_inds.iter().max_by_key(|&&i| my_stocks[i]).unwrap()
}

#[derive(Default)]
struct BasicAgent;
impl Agent for BasicAgent {
    fn choose_action(&self, game: &GameState) -> TurnAction {
        match &game.turn_state.phase {
            TurnPhase::PlaceTile(tile_inds) => {
                let my_tiles = &game.players[game.turn_state.player].tiles;
                // Place the tile that has the most neighbors.
                let best_idx = tile_inds
                    .iter()
                    .max_by_key(|&&i| game.board.num_neighbors(my_tiles[i]))
                    .unwrap();
                TurnAction::PlaceTile(*best_idx)
            }
            TurnPhase::CreateChain(_, chain_inds) => {
                TurnAction::CreateChain(chain_with_most_shares(game, chain_inds))
            }
            TurnPhase::PickWinningChain(choices, _) => {
                TurnAction::PickWinningChain(chain_with_most_shares(game, choices))
            }
            _ => RandomAgent.choose_action(game),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::TurnAction;

    fn make_game() -> GameState {
        let chain_names = [
            "K".to_owned(),
            "L".to_owned(),
            "M".to_owned(),
            "N".to_owned(),
            "O".to_owned(),
            "P".to_owned(),
            "Q".to_owned(),
        ];
        let mut rng = rand::thread_rng();
        GameState::new(2, &mut rng, chain_names)
    }

    #[test]
    fn test_random_agent() {
        let mut game = make_game();
        let ai = create_agent(0);
        let action = ai.choose_action(&game);
        assert!(matches!(action, TurnAction::PlaceTile(_)), "{:?}", action);
        assert_eq!(game.take_turn(action), Ok(false));
    }

    #[test]
    fn smoke_full_game() {
        let mut game = make_game();
        let ai = create_agent(0);
        loop {
            let action = ai.choose_action(&game);
            if game.take_turn(action).unwrap() {
                break;
            }
        }
        // Check that asking for an action at the end of game is valid.
        let action = ai.choose_action(&game);
        assert!(matches!(action, TurnAction::PlaceTile(0)), "{:?}", action);
    }
}
