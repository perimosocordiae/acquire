use rand::seq::SliceRandom;

// Grid cells named from 1-A to 12-I.
const GRID_WIDTH: usize = 12;
const GRID_HEIGHT: usize = 9;
pub const MAX_NUM_CHAINS: usize = 7;
const STOCKS_PER_CHAIN: usize = 25;
const BUY_LIMIT: usize = 3;
const SAFE_CHAIN_SIZE: usize = 11;
const DUMMY_CHAIN_INDEX: usize = 999;

pub fn chain_name(chain_index: usize) -> &'static str {
    match chain_index {
        0 => "Tower",
        1 => "Luxor",
        2 => "American",
        3 => "Worldwide",
        4 => "Festival",
        5 => "Imperial",
        6 => "Continental",
        DUMMY_CHAIN_INDEX => "Dummy",
        _ => panic!("Invalid chain index"),
    }
}

// Contains (row, col) indices.
#[derive(Clone, Copy)]
pub struct Tile(usize, usize);
impl std::fmt::Debug for Tile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let row = (b'A' + self.0 as u8) as char;
        let col = 1 + self.1;
        write!(f, "{}-{}", col, row)
    }
}

pub struct Player {
    pub cash: usize,
    stocks: [usize; MAX_NUM_CHAINS],
    pub tiles: Vec<Tile>,
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
pub enum TurnPhase {
    // Player has not yet placed a tile. Payload: playable tile indices.
    PlaceTile(Vec<usize>),
    // Player's tile is creating a new chain. Payload: (tile, available chains).
    CreateChain(Tile, Vec<usize>),
    // 2+ existing chains are merging.
    // Payload: (choices for the winning chain, all merging chains).
    PickWinningChain(Vec<usize>, Vec<usize>),
    // Post-merger stock disposal.
    // Payload: (winning chain, remaining chains to merge, player idx)
    ResolveMerger(usize, Vec<usize>, usize),
    // Buy phase. Payload indicates the number of buyable stocks per chain.
    BuyStock([usize; MAX_NUM_CHAINS]),
    // Game over. Payload indicates each player's final value.
    GameOver(Vec<usize>),
}

#[derive(Debug)]
pub struct TurnState {
    pub player: usize,
    pub phase: TurnPhase,
}

enum TilePlayability {
    Playable,
    TemporarilyUnplayable,
    PermanentlyUnplayable,
}

pub struct GameState {
    pub players: Vec<Player>,
    pub turn_state: TurnState,
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
    pub fn new(num_players: usize, rng: &mut impl rand::Rng) -> Self {
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
    pub fn stock_price(&self, chain_index: usize) -> usize {
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
    pub fn place_tile(&mut self, idx: usize) -> Result<(), String> {
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
                // Assign newly-chained hotel tiles to a dummy chain index.
                // This brings adjacent non-chain hotels into the chain, even
                // though the player may still need to decide which chain is
                // the winner of the merger.
                let dummy = GridCell::Chain(DUMMY_CHAIN_INDEX);
                self.grid[tile.0][tile.1] = dummy;
                for (t, c) in neighbors {
                    if let GridCell::Hotel = c {
                        self.grid[t.0][t.1] = dummy;
                    }
                }
                self.turn_state.phase = TurnPhase::PickWinningChain(winner_choices, candidates);
            }
        }
        Ok(())
    }
    pub fn create_chain(&mut self, chain_index: usize) -> Result<(), String> {
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
    pub fn pick_winning_chain(&mut self, chain_index: usize) -> Result<(), String> {
        if let TurnPhase::PickWinningChain(valid_indices, merging_chains) = &self.turn_state.phase {
            if !valid_indices.contains(&chain_index) {
                return Err(format!("Invalid chain index: {}", chain_index));
            }
            let loser_chains = merging_chains
                .iter()
                .filter(|&&i| i != chain_index)
                .copied()
                .collect::<Vec<usize>>();
            // Update the grid and count additions to the winning chain.
            let mut new_hotels = 0;
            for row in &mut self.grid {
                for cell in row {
                    if let GridCell::Chain(idx) = cell {
                        if *idx == DUMMY_CHAIN_INDEX || loser_chains.contains(idx) {
                            *cell = GridCell::Chain(chain_index);
                            new_hotels += 1;
                        }
                    }
                }
            }
            // Update the winning chain's size.
            self.chain_sizes[chain_index] += new_hotels;
            self.turn_state.phase =
                TurnPhase::ResolveMerger(chain_index, loser_chains, self.turn_state.player);
            Ok(())
        } else {
            return Err(format!("Wrong phase: {:?}", self.turn_state.phase));
        }
    }
    pub fn resolve_merger(
        &mut self,
        sell_amount: usize,
        trade_amount: usize,
    ) -> Result<(), String> {
        if let TurnPhase::ResolveMerger(winner_chain, loser_chains, selling_player) =
            &self.turn_state.phase
        {
            let loser_index = loser_chains[0];
            let loser_price = self.stock_price(loser_index);

            let num_not_kept = sell_amount + trade_amount;
            let num_traded = trade_amount / 2;
            // Validate that the selling player has enough stocks to sell / trade.
            let prev_stocks = self.players[*selling_player].stocks[loser_index];
            if prev_stocks < num_not_kept {
                return Err(format!(
                    "Cannot sell/trade {} stocks of {}, only have {} total.",
                    num_not_kept,
                    chain_name(loser_index),
                    prev_stocks
                ));
            }
            // Validate that there are enough winner chain stocks to trade for.
            if num_traded > self.stock_market[*winner_chain] {
                return Err(format!(
                    "Cannot trade {} stocks of {}, market has {} available.",
                    num_traded,
                    chain_name(*winner_chain),
                    self.stock_market[*winner_chain]
                ));
            }

            self.players[*selling_player].stocks[loser_index] -= num_not_kept;
            self.players[*selling_player].cash += loser_price * sell_amount;
            self.stock_market[loser_index] += num_not_kept;
            self.stock_market[*winner_chain] -= num_traded;
            self.players[*selling_player].stocks[*winner_chain] += num_traded;

            // If this is the merging player's turn, distribute the merger bonuses.
            if *selling_player == self.turn_state.player {
                pay_bonuses(loser_price, &mut self.players);
            }

            let next_player = (selling_player + 1) % self.players.len();
            if next_player == self.turn_state.player {
                // This merger is complete.
                self.chain_sizes[loser_index] = 0;
                if loser_chains.len() > 1 {
                    // There are more mergers to resolve.
                    self.turn_state.phase = TurnPhase::ResolveMerger(
                        *winner_chain,
                        loser_chains[1..].to_vec(),
                        next_player,
                    );
                } else {
                    // All mergers are resolved, move on to the buy phase.
                    self.turn_state.phase = TurnPhase::BuyStock(self.available_stocks());
                }
            } else {
                // Let the next player resolve the merger.
                self.turn_state.phase =
                    TurnPhase::ResolveMerger(*winner_chain, loser_chains.clone(), next_player);
            }
            Ok(())
        } else {
            return Err(format!("Wrong phase: {:?}", self.turn_state.phase));
        }
    }
    pub fn buy_stock(&mut self, buy_order: [usize; MAX_NUM_CHAINS]) -> Result<(), String> {
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
        let is_game_over = max_chain_size > 40
            || (max_chain_size >= SAFE_CHAIN_SIZE
                && self
                    .chain_sizes
                    .iter()
                    .all(|&size| size >= SAFE_CHAIN_SIZE || size == 0));
        if is_game_over {
            // Pay bonuses for each active chain.
            for (i, &size) in self.chain_sizes.iter().enumerate() {
                if size > 0 {
                    pay_bonuses(self.stock_price(i), &mut self.players);
                }
            }
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
        if playable_tiles.is_empty() {
            self.turn_state.phase = TurnPhase::BuyStock(self.available_stocks());
        } else {
            self.turn_state.phase = TurnPhase::PlaceTile(playable_tiles);
        }
    }
}

fn distribute_bonus(bonus: usize, receiving_players: &[usize], players: &mut [Player]) {
    let mut amount = bonus / receiving_players.len();
    // Round up to the nearest 100.
    if amount % 100 != 0 {
        amount += 100 - (amount % 100);
    }
    for p in receiving_players {
        players[*p].cash += amount;
    }
}

fn pay_bonuses(stock_price: usize, players: &mut [Player]) {
    let majority_bonus = stock_price * 10;
    let second_bonus = majority_bonus / 2;
    let mut holdings: Vec<(usize, usize)> = players
        .iter()
        .enumerate()
        .map(|(i, p)| (p.stocks[i], i))
        .collect();
    holdings.sort_unstable();
    holdings.reverse();
    let max_held = holdings[0].0;
    let majority_players: Vec<usize> = holdings
        .iter()
        .filter(|(held, _)| *held == max_held)
        .map(|(_, i)| *i)
        .collect();
    if majority_players.len() > 1 {
        // Bonuses are summed in the case of a tie for first place.
        distribute_bonus(majority_bonus + second_bonus, &majority_players, players);
    } else {
        let majority_player = majority_players[0];
        players[majority_player].cash += majority_bonus;
        let second_held = holdings[1].0;
        // If the majority holder is the only holder, give them
        // the second bonus as well.
        if second_held == 0 {
            players[majority_player].cash += second_bonus;
        } else {
            let second_players: Vec<usize> = holdings
                .iter()
                .filter(|(held, _)| *held == second_held)
                .map(|(_, i)| *i)
                .collect();
            distribute_bonus(second_bonus, &second_players, players);
        }
    }
}
