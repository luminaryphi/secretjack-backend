//cosmwasm_std must be kept
use cosmwasm_std::{
    to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, Querier, StdError,
    StdResult, Storage,
};

use crate::msg::{CountResponse, HandleMsg, InitMsg, QueryMsg};
use crate::state::{config, config_read, State};



//These are for random num generation
use rand_chacha::ChaChaRng;
use rand::{RngCore, SeedableRng};


//Init function must be kept
pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let state = State {
        seed: String::from(msg.admin_seed),
        owner: deps.api.canonical_address(&env.message.sender)?,

    };



    config(&mut deps.storage).save(&state)?;

    Ok(InitResponse::default())
}




//Max value of dealer hand before they stay
pub const DEALER_STOP:u8 = 17;


//Max hand Value
pub const BLACKJACK:u8 = 21;

//Payout Multiples
pub const NAT_BLACK:f32 = 2.5;
pub const NORM_PAY:f32 = 2;




//All traits for all hands dealt
struct Hand {
    pub contents: Vec<u8>,
    pub val: u8,
    pub ace: bool,

    pub stay: bool,
    pub blackjack: bool,
    pub bust: bool
}

//Takes self, and new_card, comes from car_draw() except in secret_card case
impl Hand {
    fn hit(&self, new_card: u8) {
        self.contents.push(new_card);
        self.val += card_value(new_card);


        //ACE MANAGEMENT
        if card_value(new_card) == 11 && self.ace == false {
            self.ace = true;
        }
        //If player already has reducable ace and recieves another, reduces first ace
        else if self.ace == true && card_value(new_card) == 11 {
            self.val -= 10;
        }
        //If player is bust but has a usable ace, reduce
        if self.ace == true && self.val > BLACKJACK {
            self.val -= 10;
            self.ace == false;
        }

        //Sets possible end conditions
        if self.val == BLACKJACK {
            self.blackjack = true;
        }
        else if self.val > BLACKJACK {
            self.bust = true;
        }

    }

    fn reset(&self) {
        self.contents.clear();
        self.val = 0;
        self.ace = false;

        self.stay = false;
        self.blackjack = false;
        self.bust = false;
    }
}

//SUB STRUCTS
struct Dealer {
    pub hand: Hand,

    pub secret_card: u8,

}

struct Player {
    pub hand: Hand,

    pub did_split: bool,
    pub split_hand: Hand,

}


//Sets up reset functions for player and dealer
impl Dealer {
    fn reset(&self) {
        self.hand.reset();
        self.secret_card = 0;
    }
}

impl Player {
    fn reset(&self) {
        self.hand.reset();
        self.did_split = false;
        self.split_hand.reset();
    }
}



//Main Table Struct
struct Table {
    pub dealer: Dealer,
    pub player: Player,

    //The money the player has on the line
    pub wager: u64,


    pub opening_done: bool,

    //Insurance round indicator. No other actions can be called while this is true until insurance round is resolved
    pub insurance_round: bool

}


impl Table {
    fn reset(&self) {
        self.dealer.reset();
        self.player.reset();

        self.wager = 0;

        self.opening_done = false;
        self.insurance_round = false;
    }
}




//Player reference and storage key for each table
let sender_raw = deps.api.canonical_address(&env.message.sender)?; //Grabs human address, turns it into cannonical address
let sender_key = sender_raw.as_slice(); //Makes address into a key that storage can understand
let mut table: Table = load(&deps.storage, sender_key)?; //Loads table from storage based on the sender_key from accessing wallet




//Player starts with initial bet
//give player 2 cards, dealer recieves one card. Other is hidden
pub fn start_round() {
    //Ensure table is clear prior to round
    table.reset();

    //Give players 2 cards
    for n in 1..=2 {
        table.player.hand.hit(card_draw());
    }



    //Give dealer one card, then give secret card
    table.dealer.hand.hit(car_draw());
    table.dealer.secret_card = card_draw();

    //If value of dealers first card is 10 or 11, allow insurance option
    if card_value(table.dealer.hand[0]) == 10 || card_value(table.dealer.hand[0]) == 11 {
        table.insurance_round = true;
    }



    table.opening_done = true;


    if table.player.val == BLACKJACK && table.insurance_round == false {
        end_round();
    }

    else {
    //Returns game state after opening
    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::Open {
            player_hand: table.player.hand.contens,
            player_val: table.player.hand.val,

            dealer_hand: table.dealer.hand.contents,
            dealer_val: table.dealer.hand.val,


            insureable: table.insurance_round,


            })?),
        })
    }

}


//Insurance Round. Can only be called if insurance_round == true
//Player must pay in half his wager
pub fn insure() {
    if table.insurance_round == true {


        //Dealer has blackjack
        if table.dealer.hand.val + card_value(table.dealer.secret_card) == 21 {
            //Shows secret card
            table.dealer.hand.hit(table.dealer.secret_card);
            //Player recieves a payment equal to his wager (TODO)
            end_round();

        }

        //Dealer didnt have blackjack
        else {
        table.insurance_round == false;

            //Returns Game state
            Ok(HandleResponse {
                messages: vec![],
                log: vec![],
                data: Some(to_binary(&HandleAnswer::Insure {

                })?),
            })
        }
    }
    else {
        //Throw error. Player cannot place insurance
    }
}

//If insurance round is called, player is given option to pass up insurance
pub fn dont_insure() {
    if table.insurance_round == true {

        //Dealer has blackjack
        if table.dealer.hand.val + card_value(table.dealer.secret_card) == 21 {
            //Shows secret card
            table.dealer.hand.hit(table.dealer.secret_card);
            //Player recieves a payment equal to his wager (TODO)
            end_round();

        }

        //Dealer Didn't have blackjack
        else {
        table.insurance_round == false;

            //Returns Game state
            Ok(HandleResponse {
                messages: vec![],
                log: vec![],
                data: Some(to_binary(&HandleAnswer::Insure {

                })?),
            })
        }
    }
    else {
        //Throw error. Player cannot place insurance
    }
}




//Function for player that hits either his split hand or regular hand
pub fn hit() {
    if table.player.did_split == true &&
    table.player.split_hand.stay == false &&
    table.player.split_hand.blackjack == false &&
    table.player.split_hand.bust == false {
        table.player.split_hand.hit(card_draw());

        //Returns last card in split hand and new split val
        Ok(HandleResponse {
            messages: vec![],
            log: vec![],
            data: Some(to_binary(&HandleAnswer::Hit {
                new_card: table.player.split_hand.contents[(table.player.split_hand.contents.len())-1],
                new_val: table.player.split_hand.val,

                which_hand: true,   //true means this came from split hand

            })?),
        })




    }
    else if table.player.hand.stay == false &&
    table.player.hand.blackjack == false &&
    table.player.hand.bust == false {
        table.player.hand.hit(card_draw());

        //Returns last card in normal hand and new hand value
        Ok(HandleResponse {
            messages: vec![],
            log: vec![],
            data: Some(to_binary(&HandleAnswer::Hit {
                new_card: table.player.split_hand.contents[(table.player.split_hand.contents.len())-1],
                new_val: table.player.split_hand.val,

                which_hand: false   //False means this came from normal hand
            })?),
        })

        //If player busts or reaches 21, call dealer turn
        if table.player.hand.val >= BLACKJACK {
            dealer_turn()
        }

    }

}



//Ends players turn
pub fn stand() {
    //If player is using split hand and hasn't already stayed it
    if table.player.did_split == true && table.player.split_hand.stay == false {
        table.player.split_hand.stay = true;
    }
    //If player is using normal hand, move to dealer turn
    else {
        dealer_turn();
    }


}


//Player doubles bet and adds one card
pub fn double_down() {
    if table.player.hand.contents.len() == 2 {
        table.wager *= 2;
        table.player.hand.hit(card_draw());
        stand();


        //Returns game state through handle answer
        Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::Read {
            table
            })?),
        })
    }

    else {
        //Throw error, player should not be able to double down
    }
}


pub fn split() {
    //If players first 2 cards are the same, player hasn't added cards, and hasn't already split
    if table.player.hand.contents[0] == table.player.hand.contents[1] &&
    table.player.did_split == false &&
    table.player.hand.contents.len() == 2 {

        table.player.did_split = true;

        //Takes card from player hand, moves to split hand, sets hand to value of the one card
        table.player.split_hand.hit(table.player.hand.contents[1]);
        table.player.hand.contents.pop();
        table.player.hand.val = card_value(table.player.hand.contents[0]);


        //Returns the value and contents of normal and split hands
        Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::Read {
            player_hand: table.player.hand.contents,
            player_val: table.player.hand.val,

            split_hand: table.player.split_hand.contents,
            split_val: table.player.split_hand.val
            })?),
        })

    }

}



//Dealer takes turn
fn dealer_turn() {
    //Secret card is moved into dealers hand
    table.dealer.hand.hit(table.dealer.secret_card);

    //Dealer will stop on 18 and also 17 if he doesn't have an ace
    while (table.dealer.hand.val < DEALER_STOP) ||
    (table.dealer.hand.val <= DEALER_STOP && table.dealer.hand.ace == true) {
        table.dealer.hand.hit(card_draw);
    }

    end_round();
}



pub fn end_round() {




    //Player gets blackjack and dealer does not
    if table.player.hand.blackjack == true && table.dealer.hand.blackjack == false {

    }

    //Player and dealer both have blackjack, player gets money back
    else if table.player.hand.blackjack == true && table.dealer.blackjack == true {

    }

    //Non blackjack wins count. Winning hands used as a multiple for winnings if player split
    else {
        let mut wining_hands: u8 = 0;

        //Split hand win > dealer and not bust or dealer bust and player didn't
        if (table.player.did_split == true &&
            table.player.split_hand.val > table.dealer.hand.val &&
            table.player.split_hand.bust == false) ||
            (table.dealer.hand.bust == true &&
                table.player.split_hand.bust == false) {
                    winning_hands += 1;
        }


        //Regular hand win
        if (table.player.hand.val > table.dealer.hand.val &&
            table.player.hand.bust == false) ||
            (table.dealer.hand.bust == true && table.player.hand.bust == false) {
                winning hands += 1;
            }


    }






    //Returns the cards drawn by the dealer and end game result
    Ok(HandleResponse {
    messages: vec![],
    log: vec![],
    data: Some(to_binary(&HandleAnswer::Read {
        player_hand: table.player.hand.contents,
        player_val: table.player.hand.val,

        split_hand: table.player.split_hand.contents,
        split_val: table.player.split_hand.val
        })?),
    })

}




//generates a random number between 1-52, returns a u8
fn card_draw() -> u8 {
    let random_seed: [u8; 32] = Sha256::digest(state.seed).into();
    let mut rng = ChaChaRng::from_seed(random_seed);

    return ((rng.next_u32() % 52) as u8); // a number between 0 and 51

}

pub fn card_value(card: u8) -> u8 {
    //Reduces out card suits
    card = (card + 13) % 13;
    let card_val: u8;

    match card {
        0 => card_val = 11, //Aces
        1..=9 => card_val = card + 1, //Cards 2 - 10
        10 | 11 | 12 => card_val = 10,
    }

    return card_val;

}





// PAYMENT functions

//Player recieves money

//Takes to/from addresses and amount
fn payout (
    contract_address: HumanAddr,
    player_address: HumanAddr,
    ammount: uint128,
) -> HandleResponse {
    HandleResponse {
        messages: vec![CosmosMsg::Bank(BankMsg::Send {
            from_address: contract_address,
            to_address: player_address,
            ammount: vec![Coin {
                denom: "uscrt".to_string(),
                amount,
            }],
        })],
        log: vec![],    //No idea what these lines do
        data: None,
    }
}




//handle function must be kept
pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Hit { } => hit(deps, env),
        HandleMsg::Stand { } => stand(deps, env),
        HandleMsg::Double_Down { } => double_down(deps, env),
        HandleMsg::Split { } => split(deps, env),
        HandleMsg::Start_Round { } => start_round(deps, env),
        HandleMsg::Insure { } => insure(deps, env),
        HandleMsg::Dont_Insure { } => dont_insure(deps, env),

    }
}


//query functions must be kept
pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        //QueryMsg::GetCount {} => to_binary(&query_count(deps)?),
    }
}
/*
fn query_count<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<CountResponse> {
    let state = config_read(&deps.storage).load()?;
    Ok(CountResponse { count: state.count })
}
*/
