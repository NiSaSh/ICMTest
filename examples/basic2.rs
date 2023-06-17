use postflop_solver::*;

fn main() {
    // ranges of OOP and IP in string format
    let oop_range = "66+,A8s+,A5s-A4s,AJo+,K9s+,KQo,QTs+,JTs,96s+,85s+,75s+,65s,54s";
    let ip_range = "QQ-22,AQs-A2s,ATo+,K5s+,KJo+,Q8s+,J8s+,T7s+,96s+,86s+,75s+,64s+,53s+";

    let card_config = CardConfig {
        range: [oop_range.parse().unwrap(), ip_range.parse().unwrap()],
        flop: flop_from_str("Td9d6h").unwrap(),
        turn: card_from_str("Qc").unwrap(),
        river: NOT_DEALT,
    };

    // bet sizes -> 60% of the pot, geometric size, and all-in
    // raise sizes -> 2.5x of the previous bet
    // see the documentation of `BetSizeCandidates` for more details
    let bet_sizes = BetSizeCandidates::try_from(("25%,50%", "2.5x")).unwrap();

    let tree_config = TreeConfig {
        initial_state: BoardState::Turn,
        starting_pot: 200,
        effective_stack: 900,
        rake_rate: 0.0,
        rake_cap: 0.0,
        flop_bet_sizes: [bet_sizes.clone(), bet_sizes.clone()], // [OOP, IP]
        turn_bet_sizes: [bet_sizes.clone(), bet_sizes.clone()],
        river_bet_sizes: [bet_sizes.clone(), bet_sizes.clone()],
        turn_donk_sizes: None, // use default bet sizes
        river_donk_sizes: None,
        add_allin_threshold: 5.0, // add all-in if (maximum bet size) <= 1.5x pot
        force_allin_threshold: 0.0, // force all-in if (SPR after the opponent's call) <= 0.15
        merging_threshold: 0.1,
    };

    // build the game tree
    let action_tree = ActionTree::new(tree_config).unwrap();
    let mut game = PostFlopGame::with_config(card_config, action_tree).unwrap();

    // obtain the private hands
    let oop_cards = game.private_cards(0);
    let oop_cards_str = holes_to_strings(oop_cards).unwrap();
   
    // check memory usage
    let (mem_usage, mem_usage_compressed) = game.memory_usage();
    println!(
        "Memory usage without compression: {:.2}GB",
        mem_usage as f64 / (1024.0 * 1024.0 * 1024.0)
    );
    println!(
        "Memory usage with compression: {:.2}GB",
        mem_usage_compressed as f64 / (1024.0 * 1024.0 * 1024.0)
    );
    println!("Test");
    // allocate memory without compression
    game.allocate_memory(false);

    // allocate memory with compression
    // game.allocate_memory(true);

    // solve the game
    let max_num_iterations = 100000;
    let target_exploitability = game.tree_config().starting_pot as f32 * 0.0001; // 0.5% of the pot
    let exploitability = solve(&mut game, max_num_iterations, target_exploitability, true);
    println!("Exploitability: {:.2}", exploitability);

    // solve the game manually
    // for i in 0..max_num_iterations {
    //     solve_step(&game, i);
    //     if (i + 1) % 10 == 0 {
    //         let exploitability = compute_exploitability(&game);
    //         if exploitability <= target_exploitability {
    //             println!("Exploitability: {:.2}", exploitability);
    //             break;
    //         }
    //     }
    // }
    // finalize(&mut game);

    // get equity and EV of a specific hand
    game.cache_normalized_weights();
    let equity = game.equity(0); // `0` means OOP player
    let ev = game.expected_values(0);
    println!("Equity of oop_hands[0]: {:.2}%", 100.0 * equity[0]);
    println!("EV of oop_hands[0]: {:.2}", ev[0]);

    // get equity and EV of whole hand
    let weights = game.normalized_weights(0);
    let average_equity = compute_average(&equity, weights);
    let average_ev = compute_average(&ev, weights);
    println!("Average equity: {:.2}%", 100.0 * average_equity);
    println!("Average EV: {:.2}", average_ev);

    // get available actions (OOP)
    let actions = game.available_actions();
    println!("{:?}",actions);
	let cards = game.private_cards(1);
	let length = cards.len();
	println!("{:?}",card_to_string(0));
	println!("{:?}",card_to_string(1));
	println!("The length of the vector is: {}", length);
	println!("{:?}",cards);
	
    // play `Bet(120)`
    game.play(1);

    // get available actions (IP)
    let actions = game.available_actions();
	println!("{:?}",actions);
    // confirm that IP does not fold the nut straight
    let ip_cards = game.private_cards(1);
    let strategy = game.strategy();
	let length = strategy.len();
	println!("The length of the vector is: {}", length);
	println!("{:?}",strategy[0]);
	println!("{:?}",strategy[250]);
	println!("{:?}",strategy[500]);
	println!("{:?}",strategy[750]);
    let ksjs = holes_to_strings(ip_cards)
        .unwrap()
        .iter()
        .position(|s| s == "KsJs")
        .unwrap();

    // strategy[index] => Fold
    // strategy[index + ip_cards.len()] => Call
    // strategy[index + 2 * ip_cards.len()] => Raise(300)

    // play `Call`
    game.play(1);

    // confirm that the current node is a chance node (i.e., river node)
    assert!(game.is_chance_node());

    // confirm that "7s" can be dealt
    let card_7s = card_from_str("7s").unwrap();
    assert!(game.possible_cards() & (1 << card_7s) != 0);

    // deal "7s"
    game.play(card_7s as usize);

    // back to the root node
    game.back_to_root();
}