#![no_std]
#![allow(warnings)]
use gstd::*;
use pebbles_game_io::*;

static mut PEBBLES_GAME: Option<GameState> = None;

pub fn get_random_u32() -> u32 {
    let salt = msg::id();
    let (hash, _num) = exec::random(salt.into()).expect("get_random_u32(): random call failed");
    u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]])
}

pub fn pebbles_auto_remove(game_state: &GameState) -> u32 {
    match game_state.difficulty {
        DifficultyLevel::Easy => (get_random_u32() % (game_state.max_pebbles_per_turn)) + 1,

        DifficultyLevel::Hard => {
            let peb_rem = game_state.pebbles_remaining;

            if peb_rem <= game_state.max_pebbles_per_turn {
                peb_rem
            } else {
                let ret_peb = peb_rem % (game_state.max_pebbles_per_turn + 1);
                // check `return pebbles` is valid or is not zero.
                if ret_peb == 0 {
                    1
                } else {
                    ret_peb
                }
            }
        }
    }
}

// Randomly choose who plays is the first player.
pub fn choose_first_player() -> Player {
    match get_random_u32() % 2 {
        0 => Player::User,
        _ => Player::Program,
    }
}

// Verify the number of input pebbles and the maximum number of pebbles for each round.
pub fn check_pebbles_input(
    init_msg_pebbles_count: u32,
    init_msg_max_pebbles_per_turn: u32,
) -> bool {
    if init_msg_pebbles_count < 1
        || init_msg_max_pebbles_per_turn < 1
        || init_msg_max_pebbles_per_turn >= init_msg_pebbles_count
    {
        return false;
    }
    true
}

// Start or Restart a game.
pub fn restart_game(
    init_msg_difficulty: DifficultyLevel,
    init_msg_pebbles_count: u32,
    init_msg_max_pebbles_per_turn: u32,
) {
    // Checks input data for validness
    if !check_pebbles_input(init_msg_pebbles_count, init_msg_max_pebbles_per_turn) {
        panic!("Invalid input message: Pebbles Count or Max Pebbles per Turn is invalid.");
    }
    // Chooses the first player using the `choose_first_player()` function
    let first_player: Player = choose_first_player();

    // Fills the `GameState` structure
    let mut pebbles_game = GameState {
        difficulty: init_msg_difficulty,
        pebbles_count: init_msg_pebbles_count,
        max_pebbles_per_turn: init_msg_max_pebbles_per_turn,
        pebbles_remaining: init_msg_pebbles_count,
        first_player: first_player.clone(),
        winner: None,
    };

    // Processes the first turn if the first player is Program
    if first_player == Player::Program {
        let program_take = pebbles_auto_remove(&pebbles_game);
        pebbles_game.pebbles_remaining =
            pebbles_game.pebbles_remaining.saturating_sub(program_take);
    }

    unsafe { PEBBLES_GAME = Some(pebbles_game) };
}

#[no_mangle]
pub extern "C" fn init() {
    // Receives `PebblesInit` using the `msg::load` function
    let load_init_msg = msg::load::<PebblesInit>().expect("Unable to load message");

    restart_game(
        load_init_msg.difficulty,
        load_init_msg.pebbles_count,
        load_init_msg.max_pebbles_per_turn,
    );
}

#[no_mangle]
pub extern "C" fn handle() {
    // Receives `PebblesAction` using `msg::load` function
    let load_action_msg = msg::load::<PebblesAction>().expect("Unable to load message");

    let get_pebbles_game = unsafe { PEBBLES_GAME.get_or_insert(Default::default()) };

    match load_action_msg {
        PebblesAction::GiveUp => {
            // The Program is a winner.
            get_pebbles_game.winner = Some(Player::Program);

            msg::reply(
                PebblesEvent::Won(
                    get_pebbles_game
                        .winner
                        .as_ref()
                        .expect("The Program Win")
                        .clone(),
                ),
                0,
            )
            .expect("Unable to reply GiveUp");
        }

        PebblesAction::Restart {
            difficulty,
            pebbles_count,
            max_pebbles_per_turn,
        } => {
            // Start or Restart a game.
            restart_game(difficulty.clone(), pebbles_count, max_pebbles_per_turn);

            msg::reply(
                PebblesInit {
                    difficulty,
                    pebbles_count,
                    max_pebbles_per_turn,
                },
                0,
            )
            .expect("Unable to reply Restart");
        }

        PebblesAction::Turn(mut x) => {
            // The User round execute
            if x > get_pebbles_game.max_pebbles_per_turn {
                x = get_pebbles_game.max_pebbles_per_turn;
            }

            get_pebbles_game.pebbles_remaining =
                get_pebbles_game.pebbles_remaining.saturating_sub(x);

            // Checks input data for validness
            if !check_pebbles_input(
                get_pebbles_game.pebbles_count,
                get_pebbles_game.max_pebbles_per_turn,
            ) {
                panic!("Invalid PebblesAction User turn message: Pebbles Count or Max Pebbles per Turn is invalid.");
            }

            let peb_rem = get_pebbles_game.pebbles_remaining;

            if peb_rem == 0 {
                let won_exist = get_pebbles_game.winner.clone();

                if won_exist.is_some() {
                    // The Program is a winner.
                    msg::reply(
                        PebblesEvent::Won(won_exist.as_ref().expect("Game Over.").clone()),
                        0,
                    )
                    .expect("Unable to reply Turn for Winner");

                    exec::leave();
                } else {
                    // The User is a winner.
                    get_pebbles_game.winner = Some(Player::User);

                    msg::reply(
                        PebblesEvent::Won(
                            get_pebbles_game
                                .winner
                                .as_ref()
                                .expect("Game Over.")
                                .clone(),
                        ),
                        0,
                    )
                    .expect("Unable to reply Turn for Winner");

                    exec::leave();
                    // msg::send(ActorId::new(id), get_pebbles_game.clone(), 0).expect("Unable to send");
                }
            } else {
                msg::reply(PebblesEvent::CounterTurn(peb_rem), 0).expect("Unable to reply");
                // The Program round execute
                let program_take = pebbles_auto_remove(get_pebbles_game);

                // Checks input data for validness
                if !check_pebbles_input(
                    get_pebbles_game.pebbles_count,
                    get_pebbles_game.max_pebbles_per_turn,
                ) {
                    panic!("Invalid PebblesAction Program turn message: Pebbles Count or Max Pebbles per Turn is invalid.");
                }

                get_pebbles_game.pebbles_remaining = get_pebbles_game
                    .pebbles_remaining
                    .saturating_sub(program_take);

                let peb_rem = get_pebbles_game.pebbles_remaining;

                if peb_rem == 0 {
                    // The Program is a winner.
                    get_pebbles_game.winner = Some(Player::Program);
                }
            }
        }
    };
}

#[no_mangle]
pub extern "C" fn state() {
    let pebbles_game = unsafe { PEBBLES_GAME.take().expect("Error in taking current state") };

    // Checks input data for validness
    if !check_pebbles_input(
        pebbles_game.pebbles_count,
        pebbles_game.max_pebbles_per_turn,
    ) {
        panic!("Invalid PebblesAction User turn message: Pebbles Count or Max Pebbles per Turn is invalid.");
    }

    // returns the `GameState` structure using the `msg::reply` function
    msg::reply(pebbles_game, 0).expect("Failed to reply state");
}

#[cfg(test)]
mod tests {
    use crate::check_pebbles_input;
    use gstd::*;

    #[test]
    fn test_check_pebbles_input() {
        let res: bool = check_pebbles_input(0, 0);
        assert!(!res);
        let res: bool = check_pebbles_input(10, 3);
        assert!(res);
        let res: bool = check_pebbles_input(1, 2);
        assert!(!res);
    }
}
