use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

// Grid cells named from 1-A to 12-I.
const GRID_WIDTH: usize = 12;
const GRID_HEIGHT: usize = 9;
pub const MAX_NUM_CHAINS: usize = 7;
const STOCKS_PER_CHAIN: usize = 25;
const BUY_LIMIT: usize = 3;
const SAFE_CHAIN_SIZE: usize = 11;

// Contains (row, col) indices.
#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Tile(usize, usize);
impl std::fmt::Debug for Tile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let row = (b'A' + self.0 as u8) as char;
        let col = 1 + self.1;
        write!(f, "{}-{}", col, row)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Player {
    pub cash: usize,
    stocks: [usize; MAX_NUM_CHAINS],
    tiles: Vec<Tile>,
}
impl Player {
    fn new(cash: usize, tiles: Vec<Tile>) -> Self {
        Self {
            cash,
            stocks: [0; MAX_NUM_CHAINS],
            tiles,
        }
    }
    fn display(&self, chain_names: &[String]) -> String {
        format!(
            "Cash: ${}, Stocks: [{}], Tiles: [{}]",
            self.cash,
            self.stocks
                .iter()
                .enumerate()
                .filter(|(_, &num_stocks)| num_stocks > 0)
                .map(|(i, &num_stocks)| format!("{}: {}", chain_names[i], num_stocks))
                .collect::<Vec<String>>()
                .join(", "),
            self.tiles
                .iter()
                .map(|t| format!("{:?}", t))
                .collect::<Vec<String>>()
                .join(", ")
        )
    }
    pub fn total_shares(&self) -> usize {
        self.stocks.iter().sum()
    }
    pub fn num_tiles(&self) -> usize {
        self.tiles.len()
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
enum GridCell {
    Empty = 0,
    Hotel = 1,
    Chain0 = 2,
    Chain1 = 3,
    Chain2 = 4,
    Chain3 = 5,
    Chain4 = 6,
    Chain5 = 7,
    Chain6 = 8,
    Dummy = 99,
}
impl GridCell {
    fn from_chain_idx(chain_idx: usize) -> Self {
        match chain_idx {
            0 => GridCell::Chain0,
            1 => GridCell::Chain1,
            2 => GridCell::Chain2,
            3 => GridCell::Chain3,
            4 => GridCell::Chain4,
            5 => GridCell::Chain5,
            6 => GridCell::Chain6,
            999 => GridCell::Dummy,
            _ => panic!("Invalid chain index"),
        }
    }
    fn to_chain_index(self) -> Option<usize> {
        match self {
            GridCell::Chain0 => Some(0),
            GridCell::Chain1 => Some(1),
            GridCell::Chain2 => Some(2),
            GridCell::Chain3 => Some(3),
            GridCell::Chain4 => Some(4),
            GridCell::Chain5 => Some(5),
            GridCell::Chain6 => Some(6),
            GridCell::Dummy => Some(999),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TurnAction {
    // Payload: tile index.
    PlaceTile(usize),
    // Payload: chain index.
    CreateChain(usize),
    // Payload: chain index.
    PickWinningChain(usize),
    // Payload: (sell amount, trade amount).
    ResolveMerger(usize, usize),
    // Payload: stocks bought per chain.
    BuyStock([usize; MAX_NUM_CHAINS]),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TurnState {
    pub player: usize,
    pub phase: TurnPhase,
}

enum TilePlayability {
    Playable,
    TemporarilyUnplayable,
    PermanentlyUnplayable,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BoardState {
    grid: [[GridCell; GRID_WIDTH]; GRID_HEIGHT],
    chain_sizes: [usize; MAX_NUM_CHAINS],
    stock_market: [usize; MAX_NUM_CHAINS],
    chain_names: [String; MAX_NUM_CHAINS],
}
impl std::fmt::Display for BoardState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
                    GridCell::Dummy => write!(f, "X")?,
                    _ => {
                        let name = &self.chain_names[cell.to_chain_index().unwrap()];
                        write!(f, "{}", name.chars().next().unwrap())?
                    }
                }
            }
            writeln!(f)?;
        }
        writeln!(f, "Stock market: {:?}", self.stock_market)?;
        writeln!(f, "Chain sizes: {:?}", self.chain_sizes)
    }
}

pub struct GameState {
    pub board: BoardState,
    pub players: Vec<Player>,
    pub turn_state: TurnState,
    unclaimed_tiles: Vec<Tile>,
}
impl std::fmt::Display for GameState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, p) in self.players.iter().enumerate() {
            writeln!(f, "Player {}: value = ${}", i, self.player_value(i))?;
            writeln!(f, "  {}", p.display(&self.board.chain_names))?;
        }
        write!(f, "{}", self.board)?;
        writeln!(f, "{:?}", self.turn_state)
    }
}
impl GameState {
    pub fn new(
        num_players: usize,
        rng: &mut impl rand::Rng,
        chain_names: [String; MAX_NUM_CHAINS],
    ) -> Self {
        let mut unclaimed_tiles = (0..GRID_HEIGHT)
            .flat_map(|row| (0..GRID_WIDTH).map(move |col| Tile(row, col)))
            .collect::<Vec<Tile>>();
        unclaimed_tiles.shuffle(rng);
        let players = (0..num_players)
            .map(|_| Player::new(6000, unclaimed_tiles.split_off(unclaimed_tiles.len() - 6)))
            .collect();
        let mut grid = [[GridCell::Empty; GRID_WIDTH]; GRID_HEIGHT];
        for t in unclaimed_tiles.drain(unclaimed_tiles.len() - num_players..) {
            grid[t.0][t.1] = GridCell::Hotel;
        }
        let board = BoardState {
            grid,
            chain_sizes: [0; MAX_NUM_CHAINS],
            stock_market: [STOCKS_PER_CHAIN; MAX_NUM_CHAINS],
            chain_names,
        };
        let turn_state = TurnState {
            player: rng.gen_range(0..num_players),
            phase: TurnPhase::PlaceTile((0..6).collect()),
        };
        Self {
            board,
            players,
            turn_state,
            unclaimed_tiles,
        }
    }
    pub fn from_parts(
        board: BoardState,
        players: Vec<Player>,
        turn_state: TurnState,
        unclaimed_tiles: Vec<Tile>,
    ) -> Self {
        Self {
            board,
            players,
            turn_state,
            unclaimed_tiles,
        }
    }
    pub fn num_unclaimed_tiles(&self) -> usize {
        self.unclaimed_tiles.len()
    }
    pub fn take_turn(&mut self, action: TurnAction) -> Result<bool, String> {
        match action {
            TurnAction::PlaceTile(idx) => self.place_tile(idx),
            TurnAction::CreateChain(idx) => self.create_chain(idx),
            TurnAction::PickWinningChain(idx) => self.pick_winning_chain(idx),
            TurnAction::ResolveMerger(sell, trade) => self.resolve_merger(sell, trade),
            TurnAction::BuyStock(stocks) => self.buy_stock(stocks),
        }?;
        Ok(matches!(self.turn_state.phase, TurnPhase::GameOver(_)))
    }
    fn available_stocks(&self) -> [usize; MAX_NUM_CHAINS] {
        let mut available_stocks = [0; MAX_NUM_CHAINS];
        for (i, &num_stocks) in self.board.stock_market.iter().enumerate() {
            if num_stocks > 0 && self.board.chain_sizes[i] > 1 {
                available_stocks[i] = num_stocks;
            }
        }
        available_stocks
    }
    pub fn stock_price(&self, chain_index: usize) -> usize {
        chain_stock_price(chain_index, self.board.chain_sizes[chain_index])
    }
    pub fn player_value(&self, player: usize) -> usize {
        let mut value = self.players[player].cash;
        for (chain_index, &num_stocks) in self.players[player].stocks.iter().enumerate() {
            if num_stocks > 0 {
                value += num_stocks * self.stock_price(chain_index);
            }
        }
        value
    }
    fn tile_playability(&self, tile: Tile) -> TilePlayability {
        // A tile cannot be played if it would merge two or more safe chains,
        // or if it would create an 8th chain.
        let neighbors = grid_neighbors(tile, &self.board.grid);
        if neighbors.is_empty() {
            return TilePlayability::Playable;
        }
        let mut neighbor_chains = grid_neighbors(tile, &self.board.grid)
            .iter()
            .filter_map(|(_, cell)| cell.to_chain_index())
            .collect::<Vec<usize>>();
        // Check for new chain creation.
        if neighbor_chains.is_empty() {
            return if self.board.chain_sizes.iter().any(|&size| size == 0) {
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
            .filter(|&i| self.board.chain_sizes[i] >= SAFE_CHAIN_SIZE)
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
        let neighbors = grid_neighbors(tile, &self.board.grid);
        if neighbors.is_empty() {
            // Just a single hotel.
            self.board.grid[tile.0][tile.1] = GridCell::Hotel;
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
            .filter_map(|(_, cell)| cell.to_chain_index())
            .collect::<Vec<usize>>();
        candidates.sort_unstable();
        candidates.dedup();
        match candidates.len() {
            // New chain.
            0 => {
                let available_chains = self
                    .board
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
                self.board.chain_sizes[chain_index] += neighbors.len();
                let chain = GridCell::from_chain_idx(chain_index);
                self.board.grid[tile.0][tile.1] = chain;
                for (t, _) in neighbors {
                    self.board.grid[t.0][t.1] = chain;
                }
                self.turn_state.phase = TurnPhase::BuyStock(self.available_stocks());
            }
            // Merging 2+ chains.
            _ => {
                // Sort candidates by chain size, descending.
                candidates.sort_unstable_by_key(|&i| 1000 - self.board.chain_sizes[i]);
                // Only the largest chain can be the winner of the merge.
                let max_chain_size = self.board.chain_sizes[candidates[0]];
                let winner_choices: Vec<usize> = candidates
                    .iter()
                    .filter(|&i| self.board.chain_sizes[*i] == max_chain_size)
                    .copied()
                    .collect();
                // Assign newly-chained hotel tiles to a dummy chain.
                // This brings adjacent non-chain hotels into the chain, even
                // though the player may still need to decide which chain is
                // the winner of the merger.
                self.board.grid[tile.0][tile.1] = GridCell::Dummy;
                for (t, c) in neighbors {
                    if let GridCell::Hotel = c {
                        self.board.grid[t.0][t.1] = GridCell::Dummy;
                    }
                }
                self.turn_state.phase = TurnPhase::PickWinningChain(winner_choices, candidates);
            }
        }
        Ok(())
    }
    fn create_chain(&mut self, chain_index: usize) -> Result<(), String> {
        if let TurnPhase::CreateChain(tile, valid_indices) = &self.turn_state.phase {
            if !valid_indices.contains(&chain_index) {
                return Err(format!("Invalid chain index: {}", chain_index));
            }
            if self.board.chain_sizes[chain_index] != 0 {
                return Err(format!("Chain {} already exists", chain_index));
            }
            let neighbors = grid_neighbors(*tile, &self.board.grid);
            self.board.chain_sizes[chain_index] = 1 + neighbors.len();
            // TODO: It's rare but possible that these neighbors also have
            // un-chained neighbors (due to the random initialization).
            // We should handle that case here before updating the grid.
            let chain = GridCell::from_chain_idx(chain_index);
            self.board.grid[tile.0][tile.1] = chain;
            for (t, _) in neighbors {
                self.board.grid[t.0][t.1] = chain;
            }
            // Founder's bonus: one free stock.
            if self.board.stock_market[chain_index] > 0 {
                self.board.stock_market[chain_index] -= 1;
                self.players[self.turn_state.player].stocks[chain_index] += 1;
            }
            self.turn_state.phase = TurnPhase::BuyStock(self.available_stocks());
            Ok(())
        } else {
            Err(format!("Wrong phase: {:?}", self.turn_state.phase))
        }
    }
    fn pick_winning_chain(&mut self, chain_index: usize) -> Result<(), String> {
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
            let winner_chain = GridCell::from_chain_idx(chain_index);
            let mut new_hotels = 0;
            for row in &mut self.board.grid {
                for cell in row {
                    if *cell == GridCell::Dummy {
                        *cell = winner_chain;
                        new_hotels += 1;
                    } else if let Some(idx) = cell.to_chain_index() {
                        if loser_chains.contains(&idx) {
                            *cell = winner_chain;
                            new_hotels += 1;
                        }
                    }
                }
            }
            // Update the winning chain's size.
            self.board.chain_sizes[chain_index] += new_hotels;
            self.turn_state.phase =
                TurnPhase::ResolveMerger(chain_index, loser_chains, self.turn_state.player);
            Ok(())
        } else {
            Err(format!("Wrong phase: {:?}", self.turn_state.phase))
        }
    }
    fn resolve_merger(&mut self, sell_amount: usize, trade_amount: usize) -> Result<(), String> {
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
                    num_not_kept, self.board.chain_names[loser_index], prev_stocks
                ));
            }
            // Validate that there are enough winner chain stocks to trade for.
            if num_traded > self.board.stock_market[*winner_chain] {
                return Err(format!(
                    "Cannot trade {} stocks of {}, market has {} available.",
                    num_traded,
                    self.board.chain_names[*winner_chain],
                    self.board.stock_market[*winner_chain]
                ));
            }

            self.players[*selling_player].stocks[loser_index] -= num_not_kept;
            self.players[*selling_player].cash += loser_price * sell_amount;
            self.board.stock_market[loser_index] += num_not_kept;
            self.board.stock_market[*winner_chain] -= num_traded;
            self.players[*selling_player].stocks[*winner_chain] += num_traded;

            // If this is the merging player's turn, distribute the merger bonuses.
            if *selling_player == self.turn_state.player {
                pay_bonuses(loser_index, loser_price, &mut self.players);
            }

            let next_player = (selling_player + 1) % self.players.len();
            if next_player == self.turn_state.player {
                // This merger is complete.
                self.board.chain_sizes[loser_index] = 0;
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
            Err(format!("Wrong phase: {:?}", self.turn_state.phase))
        }
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
            self.board.stock_market[chain_index] -= num_stocks;
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
        let chain_sizes = &self.board.chain_sizes;
        let max_chain_size = *chain_sizes.iter().max().unwrap();
        let is_game_over = max_chain_size > 40
            || (max_chain_size >= SAFE_CHAIN_SIZE
                && chain_sizes
                    .iter()
                    .all(|&size| size >= SAFE_CHAIN_SIZE || size == 0));
        if is_game_over {
            // Pay bonuses for each active chain.
            for (i, &size) in chain_sizes.iter().enumerate() {
                if size > 0 {
                    pay_bonuses(i, chain_stock_price(i, size), &mut self.players);
                }
            }
            let final_values = (0..self.players.len())
                .map(|i| self.player_value(i))
                .collect();
            self.turn_state.phase = TurnPhase::GameOver(final_values);
            return;
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

fn pay_bonuses(stock_index: usize, stock_price: usize, players: &mut [Player]) {
    let majority_bonus = stock_price * 10;
    let second_bonus = majority_bonus / 2;
    // Sort players by how many of this stock they have.
    let mut holdings: Vec<(usize, usize)> = players
        .iter()
        .enumerate()
        .map(|(i, p)| (p.stocks[stock_index], i))
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

fn chain_stock_price(chain_index: usize, chain_size: usize) -> usize {
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

fn grid_neighbors(
    tile: Tile,
    grid: &[[GridCell; GRID_WIDTH]; GRID_HEIGHT],
) -> Vec<(Tile, GridCell)> {
    let mut neighbors = Vec::new();
    let mut maybe_push = |r: usize, c: usize| {
        let cell = grid[r][c];
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_debug() {
        assert_eq!(format!("{:?}", Tile(0, 0)), "1-A");
        assert_eq!(format!("{:?}", Tile(0, 9)), "10-A");
        assert_eq!(format!("{:?}", Tile(1, 0)), "1-B");
        assert_eq!(format!("{:?}", Tile(9, 9)), "10-J");
    }

    #[test]
    fn player_display() {
        let chain_names = ["A".to_string(), "B".to_string(), "C".to_string()];
        assert_eq!(
            Player::new(0, vec![]).display(&chain_names),
            "Cash: $0, Stocks: [], Tiles: []"
        );
        assert_eq!(
            Player::new(500, vec![Tile(1, 1)]).display(&chain_names),
            "Cash: $500, Stocks: [], Tiles: [2-B]"
        );
        let mut p = Player::new(100, vec![Tile(1, 1), Tile(2, 2)]);
        p.stocks[1] = 13;
        assert_eq!(
            p.display(&chain_names),
            "Cash: $100, Stocks: [B: 13], Tiles: [2-B, 3-C]"
        );
    }

    #[test]
    fn gridcell_roundtrip() {
        for i in 0..MAX_NUM_CHAINS {
            assert_eq!(GridCell::from_chain_idx(i).to_chain_index(), Some(i));
        }
        assert_eq!(
            GridCell::from_chain_idx(GridCell::Dummy.to_chain_index().unwrap()),
            GridCell::Dummy
        );
        assert_eq!(GridCell::Empty.to_chain_index(), None);
    }

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
    fn game_display() {
        assert!(make_game().to_string().contains("Player 0: value = $6000"));
    }

    #[test]
    fn game_unclaimed_tiles() {
        assert_eq!(make_game().num_unclaimed_tiles(), 94);
    }

    #[test]
    fn distributes_bonus() {
        let mut players = [
            Player::new(0, vec![]),
            Player::new(100, vec![]),
            Player::new(200, vec![]),
        ];
        // 1000 split 3 ways is rounded up to 400 each.
        distribute_bonus(1000, &[0, 1, 2], &mut players);
        assert_eq!(players[0].cash, 400);
        assert_eq!(players[1].cash, 500);
        assert_eq!(players[2].cash, 600);
        // 100 to a single player.
        distribute_bonus(100, &[0], &mut players);
        assert_eq!(players[0].cash, 500);
        assert_eq!(players[1].cash, 500);
        assert_eq!(players[2].cash, 600);
    }

    #[test]
    fn pay_bonuses_simple() {
        let mut players = [
            Player::new(0, vec![]),
            Player::new(100, vec![]),
            Player::new(200, vec![]),
        ];
        // Test the simple case of a single majority holder and single second place.
        players[0].stocks[3] = 1;
        players[1].stocks[3] = 4;
        players[2].stocks[3] = 3;
        pay_bonuses(3, 300, &mut players);
        assert_eq!(players[0].cash, 0); // No bonus.
        assert_eq!(players[1].cash, 3100); // Majority bonus is 3000.
        assert_eq!(players[2].cash, 1700); // Second place bonus is 1500.
    }

    #[test]
    fn pay_bonuses_majority_tie() {
        let mut players = [
            Player::new(0, vec![]),
            Player::new(100, vec![]),
            Player::new(200, vec![]),
        ];
        // Tie for majority.
        players[0].stocks[2] = 7;
        players[1].stocks[3] = 3;
        players[2].stocks[3] = 3;
        pay_bonuses(3, 300, &mut players);
        assert_eq!(players[0].cash, 0); // No bonus.
        assert_eq!(players[1].cash, 2400); // Combined bonus: 4500 / 2 => 2300
        assert_eq!(players[2].cash, 2500); // Combined bonus: 4500 / 2 => 2300
    }

    #[test]
    fn pay_bonuses_second_place_tie() {
        let mut players = [
            Player::new(0, vec![]),
            Player::new(100, vec![]),
            Player::new(200, vec![]),
        ];
        // Tie for second place.
        players[0].stocks[3] = 1;
        players[1].stocks[3] = 3;
        players[2].stocks[3] = 1;
        pay_bonuses(3, 300, &mut players);
        assert_eq!(players[0].cash, 800); // Second place: 1500 / 2 => 800
        assert_eq!(players[1].cash, 3100); // Majority bonus is 3000.
        assert_eq!(players[2].cash, 1000); // Second place: 1500 / 2 => 800
    }

    #[test]
    fn pay_bonuses_sole_majority() {
        let mut players = [
            Player::new(0, vec![]),
            Player::new(100, vec![]),
            Player::new(200, vec![]),
        ];
        // Only one player has any of the relevant stock.
        players[0].stocks[1] = 0;
        players[1].stocks[3] = 3;
        players[2].stocks[0] = 0;
        pay_bonuses(3, 300, &mut players);
        assert_eq!(players[0].cash, 0); // No bonus.
        assert_eq!(players[1].cash, 4600); // Combined bonus: 4500
        assert_eq!(players[2].cash, 200); // No bonus.
    }

    #[test]
    fn computes_stock_price() {
        // Any chain of size 0 has a price of 0.
        assert_eq!(chain_stock_price(0, 0), 0);
        assert_eq!(chain_stock_price(3, 0), 0);
        assert_eq!(chain_stock_price(6, 0), 0);
        // Small chains scale linearly, plus some extra based on the chain index.
        assert_eq!(chain_stock_price(0, 2), 200);
        assert_eq!(chain_stock_price(3, 2), 300);
        assert_eq!(chain_stock_price(6, 6), 800);
        // Medium chains have a fixed price.
        assert_eq!(chain_stock_price(0, 7), 600);
        assert_eq!(chain_stock_price(0, 10), 600);
    }

    #[test]
    #[should_panic(expected = "Invalid chain size")]
    fn chain_stock_price_invalid_chain_size() {
        chain_stock_price(0, 1);
    }

    #[test]
    #[should_panic(expected = "Invalid chain index")]
    fn chain_stock_price_invalid_chain_index() {
        chain_stock_price(99, 4);
    }

    #[test]
    fn finds_grid_neighbors() {
        let mut grid = [[GridCell::Empty; GRID_WIDTH]; GRID_HEIGHT];
        assert_eq!(grid_neighbors(Tile(0, 0), &grid), vec![]);
        // One neighbor
        grid[1][1] = GridCell::Hotel;
        assert_eq!(
            grid_neighbors(Tile(2, 1), &grid),
            vec![(Tile(1, 1), GridCell::Hotel)]
        );
        // Two neighbors.
        grid[3][1] = GridCell::Chain1;
        assert_eq!(
            grid_neighbors(Tile(2, 1), &grid),
            vec![
                (Tile(1, 1), GridCell::Hotel),
                (Tile(3, 1), GridCell::Chain1),
            ]
        );
    }
}
