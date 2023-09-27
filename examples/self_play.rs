use acquire::agent;
use acquire::game::{GameState, TurnPhase};
use clap::Parser;

#[derive(Parser)]
struct Args {
    #[clap(short, long, default_value_t = 1000)]
    games: usize,
    #[clap(short, long, value_delimiter = ',', default_value = "0,0,0,0")]
    agents: Vec<usize>,
    #[clap(short, long, default_value_t = false)]
    verbose: bool,
}

fn main() {
    let chain_names = [
        "K".to_owned(),
        "L".to_owned(),
        "M".to_owned(),
        "N".to_owned(),
        "O".to_owned(),
        "P".to_owned(),
        "Q".to_owned(),
    ];
    let args = Args::parse();
    let mut rng = rand::thread_rng();

    let num_players = args.agents.len();
    for game_idx in 0..args.games {
        let mut game = GameState::new(num_players, &mut rng, chain_names.clone());
        let agents = args
            .agents
            .iter()
            .map(|&i| agent::create_agent(i))
            .collect::<Vec<_>>();
        loop {
            let ai = &agents[player_idx(&game)];
            let action = ai.choose_action(&game);
            if game.take_turn(action).unwrap() {
                break;
            }
        }
        if args.verbose {
            println!("Game {}:\n{}", game_idx, game);
        } else if let TurnPhase::GameOver(scores) = game.turn_state.phase {
            println!(
                "{}",
                scores
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            );
        }
    }
}

fn player_idx(game: &GameState) -> usize {
    if let TurnPhase::ResolveMerger(_, _, idx) = game.turn_state.phase {
        idx
    } else {
        game.turn_state.player
    }
}
