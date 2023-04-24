mod game;
use crate::game::{chain_name, GameState, TurnPhase, MAX_NUM_CHAINS};
use std::error::Error;

fn main() {
    let mut rng = rand::thread_rng();
    let mut game = GameState::new(4, &mut rng);

    if std::env::args().nth(1) == Some("self_play".to_string()) {
        self_play(&mut game);
    } else {
        game_loop(&mut game);
    }
}

fn self_play(game: &mut GameState) {
    // Take arbitrary actions until the game is over.
    loop {
        match &game.turn_state.phase {
            TurnPhase::PlaceTile(tile_inds) => {
                game.place_tile(tile_inds[0]).unwrap();
            }
            TurnPhase::CreateChain(_, chain_inds) => {
                game.create_chain(chain_inds[0]).unwrap();
            }
            TurnPhase::PickWinningChain(choices, _) => {
                game.pick_winning_chain(choices[0]).unwrap();
            }
            TurnPhase::ResolveMerger(_, _, _) => {
                game.resolve_merger(0, 0).unwrap();
            }
            TurnPhase::BuyStock(buyable_amounts) => {
                let mut buy_order = [0; MAX_NUM_CHAINS];
                for (i, &amount) in buyable_amounts.iter().enumerate() {
                    if amount > 0 && game.stock_price(i) < game.players[game.turn_state.player].cash
                    {
                        buy_order[i] = 1;
                        break;
                    }
                }
                game.buy_stock(buy_order).unwrap();
            }
            TurnPhase::GameOver(_) => {
                println!("Game over!\n{}", game);
                break;
            }
        }
    }
}

fn game_loop(game: &mut GameState) {
    // Super-janky CLI for testing.
    let mut input = String::new();
    loop {
        print!("{}", game);
        match handle_turn(game, &mut input) {
            Ok(true) => {}
            Ok(false) => {
                break;
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
        input.clear();
    }
}

fn handle_turn(game: &mut GameState, input: &mut String) -> Result<bool, Box<dyn Error>> {
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
            return Ok(false);
        }
    }
    std::io::stdin().read_line(input)?;
    if input.trim() == "q" {
        return Ok(false);
    }
    match &game.turn_state.phase {
        TurnPhase::PlaceTile(_) => {
            let tile_idx = input.trim().parse::<usize>()?;
            game.place_tile(tile_idx)?;
        }
        TurnPhase::CreateChain(_, _) => {
            let chain_idx = input.trim().parse::<usize>()?;
            game.create_chain(chain_idx)?;
        }
        TurnPhase::PickWinningChain(choices, _) => {
            if choices.len() > 1 {
                let chain_idx = input.trim().parse::<usize>()?;
                game.pick_winning_chain(chain_idx)?;
            } else {
                game.pick_winning_chain(choices[0])?;
            }
        }
        TurnPhase::ResolveMerger(_, _, _) => {
            let mut sell_trade = [0, 0];
            for (i, s) in input.trim().split(',').enumerate() {
                sell_trade[i] = s.parse::<usize>()?;
            }
            game.resolve_merger(sell_trade[0], sell_trade[1])?;
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
            game.buy_stock(buy_order)?;
        }
        TurnPhase::GameOver(_) => {
            panic!("Game is over, this should be unreachable");
        }
    }
    Ok(true)
}
