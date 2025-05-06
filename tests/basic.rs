use gtest::{Log, Program, System};
use pebbles_game_io::*;

const ADMIN: u64 = 100;
const MAX_NUMBER_OF_TURNS: u32 = 10;
const MAX_PEBBLES_PER_TURN: u32 = 5;
const PEBBLES_COUNT: u32 = 35;
const GAS: u128 = 10000000000000000000000000;

#[test]
fn success_restart_game() {
    let system = System::new();

    system.init_logger();
    let game = Program::current(&system);
    // start game
    system.mint_to(ADMIN, GAS);

    game.send(
        ADMIN,
        PebblesInit {
            difficulty: DifficultyLevel::Easy,
            pebbles_count: PEBBLES_COUNT,
            max_pebbles_per_turn: MAX_PEBBLES_PER_TURN,
        },
    );
    system.run_next_block();

    for i in 1..MAX_NUMBER_OF_TURNS {
        game.send(ADMIN, PebblesAction::Turn(5));
        system.run_next_block();
        // in the third round, perform a restart
        if i == 5 {
            game.send(
                ADMIN,
                PebblesAction::Restart {
                    difficulty: DifficultyLevel::Easy,
                    pebbles_count: PEBBLES_COUNT,
                    max_pebbles_per_turn: MAX_PEBBLES_PER_TURN,
                },
            );
            system.run_next_block();
            break;
        }
    }

    game.send(ADMIN, PebblesAction::Turn(5));
    system.run_next_block();
    let state: GameState = game.read_state(b"").unwrap();
    let pebbles_remaining: u32 = state.pebbles_remaining;
    let winner: Option<Player> = state.winner.clone();

    assert_ne!(pebbles_remaining, 0);
    assert_eq!(winner, None);
}

#[test]
fn success_giveup() {
    let system = System::new();

    system.init_logger();
    let game = Program::current(&system);
    system.mint_to(ADMIN, GAS);
    game.send(
        ADMIN,
        PebblesInit {
            difficulty: DifficultyLevel::Easy,
            pebbles_count: PEBBLES_COUNT,
            max_pebbles_per_turn: MAX_PEBBLES_PER_TURN,
        },
    );

    for i in 1..MAX_NUMBER_OF_TURNS {
        if i == 5 {
            game.send(ADMIN, PebblesAction::GiveUp);
            system.run_next_block();
            Log::builder()
                .dest(ADMIN)
                .payload(PebblesEvent::Won(Player::Program));
            break;
        }

        let user_choice = 5;
        game.send(ADMIN, PebblesAction::Turn(user_choice));
    }
    let state: GameState = game.read_state(b"").unwrap();
    let winner: Player = state.winner.as_ref().expect("The Program win").clone();

    assert_eq!(winner, Player::Program);
}

#[test]
fn success_run_game_with_difficulty_easy() {
    let system = System::new();

    system.init_logger();
    let game = Program::current(&system);
    system.mint_to(ADMIN, GAS);
    game.send(
        ADMIN,
        PebblesInit {
            difficulty: DifficultyLevel::Easy,
            pebbles_count: PEBBLES_COUNT,
            max_pebbles_per_turn: MAX_PEBBLES_PER_TURN,
        },
    );
    system.run_next_block();
    for _ in 1..MAX_NUMBER_OF_TURNS {
        game.send(ADMIN, PebblesAction::Turn(5));
        system.run_next_block();

        let state: GameState = game.read_state(b"").unwrap();
        let pebbles_remaining: u32 = state.pebbles_remaining;

        if pebbles_remaining == 0 {
            break;
        }
    }

    let state: GameState = game.read_state(b"").unwrap();
    let pebbles_remaining: u32 = state.pebbles_remaining;
    let winner: Player = state.winner.as_ref().expect("REASON").clone();

    assert_eq!(pebbles_remaining, 0);
    assert!(winner == Player::Program || winner == Player::User);
}

#[test]
fn success_run_game_with_difficulty_hard() {
    let system = System::new();

    system.init_logger();
    let game = Program::current(&system);

    system.mint_to(ADMIN, GAS);

    game.send(
        ADMIN,
        PebblesInit {
            difficulty: DifficultyLevel::Hard,
            pebbles_count: PEBBLES_COUNT,
            max_pebbles_per_turn: MAX_PEBBLES_PER_TURN,
        },
    );
    system.run_next_block();

    for _ in 1..MAX_NUMBER_OF_TURNS {
        game.send(ADMIN, PebblesAction::Turn(5));
        system.run_next_block();
        let state: GameState = game.read_state(b"").unwrap();
        let pebbles_remaining: u32 = state.pebbles_remaining;

        if pebbles_remaining == 0 {
            break;
        }
    }
    let state: GameState = game.read_state(b"").unwrap();
    let pebbles_remaining: u32 = state.pebbles_remaining;

    assert_eq!(pebbles_remaining, 0);
    assert!(matches!(
        state.winner,
        Some(Player::Program) | Some(Player::User)
    ));
}
