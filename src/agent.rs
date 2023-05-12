use crate::game::{GameState, TurnAction, TurnPhase, MAX_NUM_CHAINS};

pub trait Agent {
    fn choose_action(&self, game: &GameState) -> TurnAction;
}

pub fn create_agent(_difficulty: usize) -> Box<dyn Agent + Send> {
    Box::<RandomAgent>::default()
}

#[derive(Default)]
struct RandomAgent;
impl Agent for RandomAgent {
    fn choose_action(&self, game: &GameState) -> TurnAction {
        match &game.turn_state.phase {
            TurnPhase::PlaceTile(tile_inds) => TurnAction::PlaceTile(tile_inds[0]),
            TurnPhase::CreateChain(_, chain_inds) => TurnAction::CreateChain(chain_inds[0]),
            TurnPhase::PickWinningChain(choices, _) => TurnAction::PickWinningChain(choices[0]),
            TurnPhase::ResolveMerger(_, _, _) => TurnAction::ResolveMerger(0, 0),
            TurnPhase::BuyStock(buyable_amounts) => {
                let mut buy_order = [0; MAX_NUM_CHAINS];
                for (i, &amount) in buyable_amounts.iter().enumerate() {
                    if amount > 0 && game.stock_price(i) < game.players[game.turn_state.player].cash
                    {
                        buy_order[i] = 1;
                        break;
                    }
                }
                TurnAction::BuyStock(buy_order)
            }
            TurnPhase::GameOver(_) => TurnAction::PlaceTile(0),
        }
    }
}
