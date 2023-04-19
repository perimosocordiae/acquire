use rand::seq::SliceRandom;
use std::error::Error;

fn main() {
    let mut rng = rand::thread_rng();
    let mut game = GameState::new(4, &mut rng);

    // Super-janky CLI for testing.
    let mut input = String::new();
    loop {
        print!("{}", game);
        match game_loop(&mut game, &mut input) {
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

fn game_loop(game: &mut GameState, input: &mut String) -> Result<bool, Box<dyn Error>> {
    match &game.turn_state.phase {
        TurnPhase::PlaceTile(_) => {
            println!("Choose a tile (index) to play, or 'q' to quit:");
        }
        TurnPhase::CreateChain(_, _) => {
            println!("Choose a chain (index) to create, or 'q' to quit:");
        }
        TurnPhase::MergeChains(choices, _) => {
            if choices.len() == 1 {
                println!("Press enter to merge chains, or 'q' to quit:");
            } else {
                println!("Choose a chain (index) to win the merger, or 'q' to quit:");
            }
        }
        TurnPhase::SellStock => {
            println!("Choose how much stock to sell, or 'q' to quit:");
        }
        TurnPhase::BuyStock(_) => {
            println!("Choose up to 3 stocks (comma-sep indices), or 'q' to quit:");
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
        TurnPhase::MergeChains(choices, _) => {
            if choices.len() > 1 {
                let chain_idx = input.trim().parse::<usize>()?;
                game.merge_chains(chain_idx)?;
            } else {
                game.merge_chains(choices[0])?;
            }
        }
        TurnPhase::SellStock => {
            let amount = input.trim().parse::<usize>()?;
            game.sell_stock(amount)?;
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

// Grid cells named from 1-A to 12-I.
const GRID_WIDTH: usize = 12;
const GRID_HEIGHT: usize = 9;
const MAX_NUM_CHAINS: usize = 7;
const STOCKS_PER_CHAIN: usize = 25;
const BUY_LIMIT: usize = 3;
const SAFE_CHAIN_SIZE: usize = 11;

fn chain_name(chain_index: usize) -> &'static str {
    match chain_index {
        0 => "Tower",
        1 => "Luxor",
        2 => "American",
        3 => "Worldwide",
        4 => "Festival",
        5 => "Imperial",
        6 => "Continental",
        _ => panic!("Invalid chain index"),
    }
}

// Contains (row, col) indices.
#[derive(Clone, Copy)]
struct Tile(usize, usize);
impl std::fmt::Debug for Tile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let row = (b'A' + self.0 as u8) as char;
        let col = 1 + self.1;
        write!(f, "{}-{}", col, row)
    }
}

struct Player {
    cash: usize,
    stocks: [usize; MAX_NUM_CHAINS],
    tiles: Vec<Tile>,
}
impl std::fmt::Display for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cash: ${}, Stocks: [", self.cash)?;
        self.stocks
            .iter()
            .enumerate()
            .filter(|(_, &num_stocks)| num_stocks > 0)
            .map(|(i, &num_stocks)| format!("{}: {}", chain_name(i), num_stocks))
            .collect::<Vec<String>>()
            .join(", ")
            .fmt(f)?;
        write!(f, "], Tiles: [")?;
        self.tiles
            .iter()
            .map(|t| format!("{:?}", t))
            .collect::<Vec<String>>()
            .join(", ")
            .fmt(f)?;
        write!(f, "]")
    }
}

#[derive(Clone, Copy, PartialEq)]
enum GridCell {
    Empty,
    Hotel,
    Chain(usize),
}

#[derive(Debug)]
enum TurnPhase {
    // Player has not yet placed a tile. Payload: playable tile indices.
    PlaceTile(Vec<usize>),
    // Player's tile is creating a new chain. Payload: (tile, available chains).
    CreateChain(Tile, Vec<usize>),
    // 2+ existing chains are merging.
    // Payload: (choices for the winning chain, all merging chains).
    MergeChains(Vec<usize>, Vec<usize>),
    // Post-merger stock disposal.
    SellStock,
    // Buy phase. Payload indicates the number of buyable stocks per chain.
    BuyStock([usize; MAX_NUM_CHAINS]),
    // Game over. Payload indicates each player's final value.
    GameOver(Vec<usize>),
}

#[derive(Debug)]
struct TurnState {
    player: usize,
    phase: TurnPhase,
}

enum TilePlayability {
    Playable,
    TemporarilyUnplayable,
    PermanentlyUnplayable,
}

struct GameState {
    players: Vec<Player>,
    turn_state: TurnState,
    grid: [[GridCell; GRID_WIDTH]; GRID_HEIGHT],
    unclaimed_tiles: Vec<Tile>,
    chain_sizes: [usize; MAX_NUM_CHAINS],
    stock_market: [usize; MAX_NUM_CHAINS],
}
impl std::fmt::Display for GameState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, p) in self.players.iter().enumerate() {
            writeln!(f, "Player {}: value = ${}", i, self.player_value(i))?;
            writeln!(f, "  {}", p)?;
        }
        writeln!(f, "{} unclaimed tiles", self.unclaimed_tiles.len())?;
        for col in 0..=GRID_WIDTH {
            write!(f, "{}", col % 10)?;
        }
        writeln!(f)?;
        for (i, row) in self.grid.iter().enumerate() {
            write!(f, "{}", (b'A' + i as u8) as char)?;
            for cell in row.iter() {
                match cell {
                    GridCell::Empty => write!(f, "_")?,
                    GridCell::Hotel => write!(f, "*")?,
                    GridCell::Chain(i) => write!(f, "{}", chain_name(*i).chars().next().unwrap())?,
                }
            }
            writeln!(f)?;
        }
        writeln!(f, "Stock market: {:?}", self.stock_market)?;
        writeln!(f, "Chain sizes: {:?}", self.chain_sizes)?;
        writeln!(f, "{:?}", self.turn_state)
    }
}
impl GameState {
    fn new(num_players: usize, rng: &mut impl rand::Rng) -> Self {
        let mut grid = [[GridCell::Empty; GRID_WIDTH]; GRID_HEIGHT];
        let mut unclaimed_tiles = (0..GRID_HEIGHT)
            .flat_map(|row| (0..GRID_WIDTH).map(move |col| Tile(row, col)))
            .collect::<Vec<Tile>>();
        unclaimed_tiles.shuffle(rng);
        let players = (0..num_players)
            .map(|_| Player {
                cash: 6000,
                stocks: [0; MAX_NUM_CHAINS],
                tiles: unclaimed_tiles.split_off(unclaimed_tiles.len() - 6),
            })
            .collect();
        let turn_state = TurnState {
            player: rng.gen_range(0..num_players),
            phase: TurnPhase::PlaceTile((0..6).collect()),
        };
        for t in unclaimed_tiles.drain(unclaimed_tiles.len() - num_players..) {
            grid[t.0][t.1] = GridCell::Hotel;
        }
        Self {
            players,
            turn_state,
            grid,
            unclaimed_tiles,
            chain_sizes: [0; MAX_NUM_CHAINS],
            stock_market: [STOCKS_PER_CHAIN; MAX_NUM_CHAINS],
        }
    }
    fn available_stocks(&self) -> [usize; MAX_NUM_CHAINS] {
        let mut available_stocks = [0; MAX_NUM_CHAINS];
        for (i, &num_stocks) in self.stock_market.iter().enumerate() {
            if num_stocks > 0 && self.chain_sizes[i] > 1 {
                available_stocks[i] = num_stocks;
            }
        }
        available_stocks
    }
    fn stock_price(&self, chain_index: usize) -> usize {
        let chain_size = self.chain_sizes[chain_index];
        let price = match chain_size {
            0..=0 => {
                return 0;
            }
            2..=6 => chain_size * 100,
            7..=10 => 600,
            11..=20 => 700,
            21..=30 => 800,
            31..=40 => 900,
            41..=999 => 1000,
            _ => panic!("Invalid chain size"),
        };
        match chain_index {
            0..=1 => price,
            2..=4 => price + 100,
            5..=6 => price + 200,
            _ => panic!("Invalid chain index"),
        }
    }
    fn player_value(&self, player: usize) -> usize {
        let mut value = self.players[player].cash;
        for (chain_index, &num_stocks) in self.players[player].stocks.iter().enumerate() {
            if num_stocks > 0 {
                value += num_stocks * self.stock_price(chain_index);
            }
        }
        value
    }
    fn grid_neighbors(&self, tile: Tile) -> Vec<(Tile, GridCell)> {
        let mut neighbors = Vec::new();
        let mut maybe_push = |r: usize, c: usize| {
            let cell = self.grid[r][c];
            if cell != GridCell::Empty {
                neighbors.push((Tile(r, c), cell));
            }
        };
        if tile.0 > 0 {
            maybe_push(tile.0 - 1, tile.1);
        }
        if tile.0 < GRID_HEIGHT - 1 {
            maybe_push(tile.0 + 1, tile.1);
        }
        if tile.1 > 0 {
            maybe_push(tile.0, tile.1 - 1);
        }
        if tile.1 < GRID_WIDTH - 1 {
            maybe_push(tile.0, tile.1 + 1);
        }
        neighbors
    }
    fn tile_playability(&self, tile: Tile) -> TilePlayability {
        // A tile cannot be played if it would merge two or more safe chains,
        // or if it would create an 8th chain.
        let neighbors = self.grid_neighbors(tile);
        if neighbors.is_empty() {
            return TilePlayability::Playable;
        }
        let mut neighbor_chains = self
            .grid_neighbors(tile)
            .iter()
            .filter_map(|(_, cell)| match cell {
                GridCell::Chain(i) => Some(*i),
                _ => None,
            })
            .collect::<Vec<usize>>();
        // Check for new chain creation.
        if neighbor_chains.is_empty() {
            return if self.chain_sizes.iter().any(|&size| size == 0) {
                TilePlayability::Playable
            } else {
                TilePlayability::TemporarilyUnplayable
            };
        }
        // Check for safe neighbor chains.
        neighbor_chains.sort_unstable();
        neighbor_chains.dedup();
        let num_safe_neighbors = neighbor_chains
            .into_iter()
            .filter(|&i| self.chain_sizes[i] >= SAFE_CHAIN_SIZE)
            .count();
        if num_safe_neighbors <= 1 {
            TilePlayability::Playable
        } else {
            TilePlayability::PermanentlyUnplayable
        }
    }
    fn place_tile(&mut self, idx: usize) -> Result<(), String> {
        if let TurnPhase::PlaceTile(valid_indices) = &self.turn_state.phase {
            if !valid_indices.contains(&idx) {
                return Err(format!("Invalid tile index: {}", idx));
            }
        } else {
            return Err(format!("Wrong phase: {:?}", self.turn_state.phase));
        }
        let tile = self.players[self.turn_state.player].tiles.remove(idx);
        // Check for neighboring chains or hotels.
        let neighbors = self.grid_neighbors(tile);
        if neighbors.is_empty() {
            // Just a single hotel.
            self.grid[tile.0][tile.1] = GridCell::Hotel;
            let available_stocks = self.available_stocks();
            if available_stocks.iter().any(|&x| x > 0) {
                self.turn_state.phase = TurnPhase::BuyStock(available_stocks);
            } else {
                self.next_player();
            }
            return Ok(());
        }
        // Find neighboring tiles that are part of a chain.
        let mut candidates = neighbors
            .iter()
            .filter_map(|(_, cell)| match cell {
                GridCell::Chain(idx) => Some(*idx),
                _ => None,
            })
            .collect::<Vec<usize>>();
        candidates.sort_unstable();
        candidates.dedup();
        match candidates.len() {
            // New chain.
            0 => {
                let available_chains = self
                    .chain_sizes
                    .iter()
                    .enumerate()
                    .filter(|(_, &size)| size == 0)
                    .map(|(i, _)| i)
                    .collect::<Vec<usize>>();
                self.turn_state.phase = TurnPhase::CreateChain(tile, available_chains);
            }
            // Joining an existing chain.
            1 => {
                let chain_index = candidates[0];
                self.chain_sizes[chain_index] += neighbors.len();
                self.grid[tile.0][tile.1] = GridCell::Chain(chain_index);
                for (t, _) in neighbors {
                    self.grid[t.0][t.1] = GridCell::Chain(chain_index);
                }
                self.turn_state.phase = TurnPhase::BuyStock(self.available_stocks());
            }
            // Merging 2+ chains.
            _ => {
                // Sort candidates by chain size, descending.
                candidates.sort_unstable_by_key(|&i| 1000 - self.chain_sizes[i]);
                // Only the largest chain can be the winner of the merge.
                let max_chain_size = self.chain_sizes[candidates[0]];
                let winner_choices: Vec<usize> = candidates
                    .iter()
                    .filter(|&i| self.chain_sizes[*i] == max_chain_size)
                    .copied()
                    .collect();
                // Note: we're choosing an arbitrary winner here, but the player
                // will be able to choose the actual winner later. This step is
                // needed to bring adjacent non-chain hotels into the chain.
                let winner_cell = GridCell::Chain(winner_choices[0]);
                self.grid[tile.0][tile.1] = winner_cell;
                let mut added_to_winner = 1;
                for (t, c) in neighbors {
                    if let GridCell::Hotel = c {
                        self.grid[t.0][t.1] = winner_cell;
                        added_to_winner += 1;
                    }
                }
                self.chain_sizes[winner_choices[0]] += added_to_winner;
                self.turn_state.phase = TurnPhase::MergeChains(winner_choices, candidates);
            }
        }
        Ok(())
    }
    fn create_chain(&mut self, chain_index: usize) -> Result<(), String> {
        if let TurnPhase::CreateChain(tile, valid_indices) = &self.turn_state.phase {
            if !valid_indices.contains(&chain_index) {
                return Err(format!("Invalid chain index: {}", chain_index));
            }
            if self.chain_sizes[chain_index] != 0 {
                return Err(format!("Chain {} already exists", chain_index));
            }
            let neighbors = self.grid_neighbors(*tile);
            self.chain_sizes[chain_index] = 1 + neighbors.len();
            // TODO: It's rare but possible that these neighbors also have
            // un-chained neighbors (due to the random initialization).
            // We should handle that case here before updating the grid.
            self.grid[tile.0][tile.1] = GridCell::Chain(chain_index);
            for (t, _) in neighbors {
                self.grid[t.0][t.1] = GridCell::Chain(chain_index);
            }
            // Founder's bonus: one free stock.
            if self.stock_market[chain_index] > 0 {
                self.stock_market[chain_index] -= 1;
                self.players[self.turn_state.player].stocks[chain_index] += 1;
            }
            self.turn_state.phase = TurnPhase::BuyStock(self.available_stocks());
            Ok(())
        } else {
            Err(format!("Wrong phase: {:?}", self.turn_state.phase))
        }
    }
    fn merge_chains(&mut self, chain_index: usize) -> Result<(), String> {
        if let TurnPhase::MergeChains(valid_indices, merging_chains) = &self.turn_state.phase {
            if !valid_indices.contains(&chain_index) {
                return Err(format!("Invalid chain index: {}", chain_index));
            }
            for row in &mut self.grid {
                for cell in row {
                    if let GridCell::Chain(idx) = cell {
                        if idx != &chain_index && merging_chains.contains(idx) {
                            *cell = GridCell::Chain(chain_index);
                        }
                    }
                }
            }
            for &i in merging_chains {
                if i != chain_index {
                    let majority_bonus = self.stock_price(chain_index) * 10;
                    let second_bonus = majority_bonus / 2;
                    let mut holdings: Vec<(usize, usize)> = self
                        .players
                        .iter()
                        .enumerate()
                        .map(|(i, p)| (p.stocks[i], i))
                        .collect();
                    holdings.sort_unstable();
                    holdings.reverse();
                    let max_held = holdings[0].0;
                    let majority_players = holdings
                        .iter()
                        .filter(|(held, _)| *held == max_held)
                        .map(|(_, i)| *i)
                        .collect::<Vec<usize>>();
                    if majority_players.len() > 1 {
                        // Bonuses are summed and then split evenly.
                        let mut amount = (majority_bonus + second_bonus) / majority_players.len();
                        // Round up to the nearest 100.
                        if amount % 100 != 0 {
                            amount += 100 - (amount % 100);
                        }
                        for player in majority_players {
                            self.players[player].cash += amount;
                        }
                    } else {
                        let majority_player = majority_players[0];
                        self.players[majority_player].cash += majority_bonus;
                        let second_held = holdings[1].0;
                        // If the majority holder is the only holder, give them
                        // the second bonus as well.
                        if second_held == 0 {
                            self.players[majority_player].cash += second_bonus;
                        } else {
                            let second_players = holdings
                                .iter()
                                .filter(|(held, _)| *held == second_held)
                                .map(|(_, i)| *i)
                                .collect::<Vec<usize>>();
                            let mut amount = second_bonus / second_players.len();
                            // Round up to the nearest 100.
                            if amount % 100 != 0 {
                                amount += 100 - (amount % 100);
                            }
                            for player in second_players {
                                self.players[player].cash += amount;
                            }
                        }
                    }

                    self.chain_sizes[chain_index] += self.chain_sizes[i];
                    self.chain_sizes[i] = 0;
                }
            }
            // TODO: Prepare a payload for the SellStock phase.
            self.turn_state.phase = TurnPhase::SellStock;
            Ok(())
        } else {
            return Err(format!("Wrong phase: {:?}", self.turn_state.phase));
        }
    }
    fn sell_stock(&mut self, _amount: usize) -> Result<(), String> {
        if let TurnPhase::SellStock = &self.turn_state.phase {
        } else {
            return Err(format!("Wrong phase: {:?}", self.turn_state.phase));
        }
        self.turn_state.phase = TurnPhase::BuyStock(self.available_stocks());
        todo!("Selling stocks is not implemented yet.");
    }
    fn buy_stock(&mut self, buy_order: [usize; MAX_NUM_CHAINS]) -> Result<(), String> {
        if let TurnPhase::BuyStock(available) = &self.turn_state.phase {
            if buy_order.iter().sum::<usize>() > BUY_LIMIT {
                return Err(format!(
                    "Too many stocks bought: {}",
                    buy_order.iter().sum::<usize>()
                ));
            }
            for (chain_index, &num_stocks) in buy_order.iter().enumerate() {
                if available[chain_index] < num_stocks {
                    return Err(format!(
                        "Not enough stocks available for chain {}: {} < {}",
                        chain_index, available[chain_index], num_stocks
                    ));
                }
            }
        } else {
            return Err(format!("Wrong phase: {:?}", self.turn_state.phase));
        }
        let mut cash_spent = 0;
        for (chain_index, &num_stocks) in buy_order.iter().enumerate() {
            if num_stocks > 0 {
                cash_spent += self.stock_price(chain_index) * num_stocks;
            }
        }
        let player = &mut self.players[self.turn_state.player];
        if cash_spent > player.cash {
            return Err(format!(
                "Not enough cash to buy stocks. Price: ${} > Cash: ${}",
                cash_spent, player.cash
            ));
        }
        player.cash -= cash_spent;
        for (chain_index, num_stocks) in buy_order.iter().enumerate() {
            player.stocks[chain_index] += num_stocks;
            self.stock_market[chain_index] -= num_stocks;
        }
        self.next_player();
        Ok(())
    }
    fn next_player(&mut self) {
        // Draw a new tile. If it's permanently unplayable, keep drawing.
        while let Some(tile) = self.unclaimed_tiles.pop() {
            match self.tile_playability(tile) {
                TilePlayability::PermanentlyUnplayable => {}
                _ => {
                    self.players[self.turn_state.player].tiles.push(tile);
                    break;
                }
            }
        }

        // Check for game over conditions.
        let max_chain_size = *self.chain_sizes.iter().max().unwrap();
        if max_chain_size > 40
            || (max_chain_size >= SAFE_CHAIN_SIZE
                && self
                    .chain_sizes
                    .iter()
                    .all(|&size| size >= SAFE_CHAIN_SIZE || size == 0))
        {
            // TODO: Sell off all remaining stock for each player.
            let final_values = (0..self.players.len())
                .into_iter()
                .map(|i| self.player_value(i))
                .collect();
            self.turn_state.phase = TurnPhase::GameOver(final_values);
        }

        // Advance to the next player's turn.
        self.turn_state.player = (self.turn_state.player + 1) % self.players.len();
        let playable_tiles = self.players[self.turn_state.player]
            .tiles
            .iter()
            .enumerate()
            .filter_map(|(i, &tile)| match self.tile_playability(tile) {
                TilePlayability::Playable => Some(i),
                _ => None,
            })
            .collect::<Vec<usize>>();
        self.turn_state.phase = TurnPhase::PlaceTile(playable_tiles);
    }
}
