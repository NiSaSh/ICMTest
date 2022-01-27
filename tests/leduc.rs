extern crate postflop_solver;
use postflop_solver::*;
use rayon::prelude::*;

struct LeducGame {
    root: MutexLike<LeducNode>,
    initial_reach: Vec<f32>,
}

struct LeducNode {
    player: usize,
    board: usize,
    amount: i32,
    children: Vec<(Action, MutexLike<LeducNode>)>,
    iso_chances: Vec<IsomorphicChance>,
    cum_regret: Vec<f32>,
    strategy: Vec<f32>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    None,
    Fold,
    Check,
    Call,
    Bet(i32),
    Raise(i32),
    Chance(usize),
}

const NUM_PRIVATE_HANDS: usize = 6;

#[allow(dead_code)]
const PLAYER_OOP: usize = 0;

#[allow(dead_code)]
const PLAYER_IP: usize = 1;

const PLAYER_CHANCE: usize = 0xff;
const PLAYER_MASK: usize = 0xff;
const PLAYER_TERMINAL_FLAG: usize = 0x100;
const PLAYER_FOLD_FLAG: usize = 0x300;

const NOT_DEALT: usize = 0xff;

impl Game for LeducGame {
    type Node = LeducNode;

    #[inline]
    fn root(&self) -> MutexGuardLike<Self::Node> {
        self.root.lock()
    }

    #[inline]
    fn num_private_hands(&self, _player: usize) -> usize {
        NUM_PRIVATE_HANDS
    }

    #[inline]
    fn initial_reach(&self, _player: usize) -> &[f32] {
        &self.initial_reach
    }

    fn evaluate(&self, result: &mut [f32], node: &Self::Node, player: usize, cfreach: &[f32]) {
        let num_hands = NUM_PRIVATE_HANDS * (NUM_PRIVATE_HANDS - 1);
        let num_hands_inv = 1.0 / num_hands as f32;

        if node.player & PLAYER_FOLD_FLAG == PLAYER_FOLD_FLAG {
            let folded_player = node.player & PLAYER_MASK;
            let payoff = node.amount * [1, -1][(player == folded_player) as usize];
            let payoff_normalized = payoff as f32 * num_hands_inv;
            for my_card in 0..NUM_PRIVATE_HANDS {
                if my_card != node.board {
                    for opp_card in 0..NUM_PRIVATE_HANDS {
                        if my_card != opp_card && opp_card != node.board {
                            result[my_card] += payoff_normalized * cfreach[opp_card];
                        }
                    }
                }
            }
        } else {
            for my_card in 0..NUM_PRIVATE_HANDS {
                if my_card != node.board {
                    for opp_card in 0..NUM_PRIVATE_HANDS {
                        if my_card != opp_card && opp_card != node.board {
                            let payoff = node.amount
                                * match () {
                                    _ if my_card / 2 == node.board / 2 => 1,
                                    _ if opp_card / 2 == node.board / 2 => -1,
                                    _ if my_card / 2 == opp_card / 2 => 0,
                                    _ if my_card > opp_card => 1,
                                    _ => -1,
                                };
                            let payoff_normalized = payoff as f32 * num_hands_inv;
                            result[my_card] += payoff_normalized * cfreach[opp_card];
                        }
                    }
                }
            }
        }
    }
}

impl LeducGame {
    #[inline]
    pub fn new() -> Self {
        Self {
            root: Self::build_tree(),
            initial_reach: vec![1.0; NUM_PRIVATE_HANDS],
        }
    }

    fn build_tree() -> MutexLike<LeducNode> {
        let mut root = LeducNode {
            player: PLAYER_OOP,
            board: NOT_DEALT,
            amount: 1,
            children: Vec::new(),
            iso_chances: Vec::new(),
            cum_regret: Default::default(),
            strategy: Default::default(),
        };
        Self::build_tree_recursive(&mut root, Action::None, [0, 0]);
        Self::allocate_memory_recursive(&mut root);
        MutexLike::new(root)
    }

    fn build_tree_recursive(node: &mut LeducNode, last_action: Action, last_bet: [i32; 2]) {
        if node.is_terminal() {
            return;
        }

        if node.is_chance() {
            Self::push_chance_actions(node);
            node.actions().into_par_iter().for_each(|action| {
                Self::build_tree_recursive(&mut node.play(action), Action::Chance(action), [0, 0]);
            });
            return;
        }

        let actions = Self::get_actions(node, last_action, node.board != NOT_DEALT);

        let mut last_bets = Vec::new();
        let prev_min_bet = last_bet.iter().min().unwrap();

        for (action, next_player) in &actions {
            let mut last_bet = last_bet;
            if *action == Action::Call {
                last_bet[node.player] = last_bet[node.player ^ 1];
            }
            if let Action::Bet(amount) = action {
                last_bet[node.player] = *amount;
            }
            if let Action::Raise(amount) = action {
                last_bet[node.player] = *amount;
            }
            last_bets.push(last_bet);

            let bet_diff = last_bet.iter().min().unwrap() - prev_min_bet;
            node.children.push((
                *action,
                MutexLike::new(LeducNode {
                    player: *next_player,
                    board: node.board,
                    amount: node.amount + bet_diff,
                    children: Vec::new(),
                    iso_chances: Vec::new(),
                    cum_regret: Default::default(),
                    strategy: Default::default(),
                }),
            ));
        }

        node.actions().into_par_iter().for_each(|action| {
            Self::build_tree_recursive(
                &mut node.play(action),
                actions[action].0,
                last_bets[action],
            );
        });
    }

    fn push_chance_actions(node: &mut LeducNode) {
        for index in 0..3 {
            node.children.push((
                Action::Chance(index * 2),
                MutexLike::new(LeducNode {
                    player: PLAYER_OOP,
                    board: index * 2,
                    amount: node.amount,
                    children: Vec::new(),
                    iso_chances: Vec::new(),
                    cum_regret: Default::default(),
                    strategy: Default::default(),
                }),
            ));
        }

        for index in 0..3 {
            node.iso_chances.push(IsomorphicChance {
                index,
                swap_list: [
                    vec![(index * 2, index * 2 + 1)],
                    vec![(index * 2, index * 2 + 1)],
                ],
            });
        }
    }

    fn get_actions(
        node: &LeducNode,
        last_action: Action,
        is_second_round: bool,
    ) -> Vec<(Action, usize)> {
        let raise_amount = [2, 4][is_second_round as usize];

        let player = node.player;
        let player_opponent = player ^ 1;

        let player_after_call = if is_second_round {
            PLAYER_TERMINAL_FLAG | player
        } else {
            PLAYER_CHANCE
        };

        let player_after_check = if player == PLAYER_OOP {
            player_opponent
        } else {
            player_after_call
        };

        let mut actions = Vec::new();

        match last_action {
            Action::None | Action::Check | Action::Chance(_) => {
                actions.push((Action::Check, player_after_check));
                actions.push((Action::Bet(raise_amount), player_opponent));
            }
            Action::Bet(amount) => {
                actions.push((Action::Fold, PLAYER_FOLD_FLAG | player));
                actions.push((Action::Call, player_after_call));
                actions.push((Action::Raise(amount + raise_amount), player_opponent));
            }
            Action::Raise(_) => {
                actions.push((Action::Fold, PLAYER_FOLD_FLAG | player));
                actions.push((Action::Call, player_after_call));
            }
            Action::Fold | Action::Call => unreachable!(),
        };

        actions
    }

    fn allocate_memory_recursive(node: &mut LeducNode) {
        if node.is_terminal() {
            return;
        }

        if !node.is_chance() {
            let num_actions = node.num_actions();
            node.cum_regret = vec![0.0; num_actions * NUM_PRIVATE_HANDS];
            node.strategy = vec![0.0; num_actions * NUM_PRIVATE_HANDS];
        }

        node.actions().into_par_iter().for_each(|action| {
            Self::allocate_memory_recursive(&mut node.play(action));
        });
    }
}

impl GameNode for LeducNode {
    #[inline]
    fn is_terminal(&self) -> bool {
        self.player & PLAYER_TERMINAL_FLAG != 0
    }

    #[inline]
    fn is_chance(&self) -> bool {
        self.player == PLAYER_CHANCE
    }

    #[inline]
    fn player(&self) -> usize {
        self.player
    }

    #[inline]
    fn num_actions(&self) -> usize {
        self.children.len()
    }

    #[inline]
    fn chance_factor(&self) -> f32 {
        1.0 / 4.0
    }

    #[inline]
    fn isomorphic_chances(&self) -> &Vec<IsomorphicChance> {
        &self.iso_chances
    }

    #[inline]
    fn play(&self, action: usize) -> MutexGuardLike<Self> {
        self.children[action].1.lock()
    }

    #[inline]
    fn cum_regret(&self) -> &[f32] {
        &self.cum_regret
    }

    #[inline]
    fn cum_regret_mut(&mut self) -> &mut [f32] {
        &mut self.cum_regret
    }

    #[inline]
    fn strategy(&self) -> &[f32] {
        &self.strategy
    }

    #[inline]
    fn strategy_mut(&mut self) -> &mut [f32] {
        &mut self.strategy
    }
}

#[test]
fn leduc() {
    rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build_global()
        .unwrap();

    let target = 1e-4;
    let game = LeducGame::new();
    solve(&game, 10000, target, 0.0, false);

    let ev = compute_ev(&game, 0);
    let expected_ev = -0.0856; // verified by OpenSpiel
    assert!((ev - expected_ev).abs() <= 2.0 * target);
}