use acquire::game::{chain_name, GameState, TurnAction, TurnPhase, MAX_NUM_CHAINS};
use std::error::Error;

fn main() {
    let mut rng = rand::thread_rng();
    let mut game = GameState::new(4, &mut rng);

    // Super-janky CLI for testing.
    let mut input = String::new();
    loop {
        print!("{}", game);
        match handle_turn(&mut game, &mut input) {
            Ok(Some(action)) => {
                game.take_turn(action).unwrap();
            }
            Ok(None) => {
                break;
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
        input.clear();
    }
}

fn handle_turn(
    game: &mut GameState,
    input: &mut String,
) -> Result<Option<TurnAction>, Box<dyn Error>> {
    match &game.turn_state.phase {
        TurnPhase::PlaceTile(tile_inds) => {
            println!("Choose a tile (index) to play, or 'q' to quit:");
            for idx in tile_inds {
                println!(
                    "{}: {:?}",
                    idx, game.players[game.turn_state.player].tiles[*idx]
                );
            }
        }
        TurnPhase::CreateChain(_, chain_inds) => {
            println!("Choose a chain (index) to create, or 'q' to quit:");
            for idx in chain_inds {
                println!("{}: {}", idx, chain_name(*idx));
            }
        }
        TurnPhase::PickWinningChain(choices, _) => {
            if choices.len() == 1 {
                println!("Press enter to merge chains, or 'q' to quit:");
            } else {
                println!("Choose a chain (index) to win the merger, or 'q' to quit:");
                for idx in choices {
                    println!("{}: {}", idx, chain_name(*idx));
                }
            }
        }
        TurnPhase::ResolveMerger(_, chains, player_idx) => {
            println!(
                "Player {}: Choose 'sell,trade' amounts of {} stock, or 'q' to quit:",
                player_idx,
                chain_name(chains[0])
            );
        }
        TurnPhase::BuyStock(buyable_amounts) => {
            println!("Choose up to 3 stocks (comma-sep indices), or 'q' to quit:");
            for (i, &amount) in buyable_amounts.iter().enumerate() {
                if amount > 0 {
                    println!("{}: {} buyable in {}", i, amount, chain_name(i));
                }
            }
        }
        TurnPhase::GameOver(final_values) => {
            println!("Game over! Final values: {:?}", final_values);
            return Ok(None);
        }
    }
    std::io::stdin().read_line(input)?;
    if input.trim() == "q" {
        return Ok(None);
    }
    let action = match &game.turn_state.phase {
        TurnPhase::PlaceTile(_) => {
            let tile_idx = input.trim().parse::<usize>()?;
            TurnAction::PlaceTile(tile_idx)
        }
        TurnPhase::CreateChain(_, _) => {
            let chain_idx = input.trim().parse::<usize>()?;
            TurnAction::CreateChain(chain_idx)
        }
        TurnPhase::PickWinningChain(choices, _) => {
            let chain_idx = if choices.len() > 1 {
                input.trim().parse::<usize>()?
            } else {
                0
            };
            TurnAction::PickWinningChain(chain_idx)
        }
        TurnPhase::ResolveMerger(_, _, _) => {
            let mut sell_trade = [0, 0];
            for (i, s) in input.trim().split(',').enumerate() {
                sell_trade[i] = s.parse::<usize>()?;
            }
            TurnAction::ResolveMerger(sell_trade[0], sell_trade[1])
        }
        TurnPhase::BuyStock(_) => {
            let mut buy_order = [0; MAX_NUM_CHAINS];
            for s in input.trim().split(',') {
                if s.is_empty() {
                    continue;
                }
                let idx = s.parse::<usize>()?;
                buy_order[idx] += 1;
            }
            TurnAction::BuyStock(buy_order)
        }
        TurnPhase::GameOver(_) => {
            panic!("Game is over, this should be unreachable");
        }
    };
    Ok(Some(action))
}
