use acquire::agent;
use acquire::game::GameState;

fn main() {
    let mut rng = rand::thread_rng();
    let mut game = GameState::new(4, &mut rng);

    self_play(&mut game);
}

fn self_play(game: &mut GameState) {
    // Take arbitrary actions until the game is over.
    let ai = agent::create_agent(0);
    loop {
        let action = ai.choose_action(game);
        if game.take_turn(action).unwrap() {
            println!("Game over!\n{}", game);
            break;
        }
    }
}
