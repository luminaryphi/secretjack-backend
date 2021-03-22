//cosmwasm_std must be kept
use cosmwasm_std::{
    to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, Querier, StdError,
    StdResult, Storage,
};

use crate::msg::{CountResponse, HandleMsg, InitMsg, QueryMsg};
use crate::state::{config, config_read, State};

//allows us to use hashmaps
use std::collections::HashMap;




//Init function must be kept
pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
    admin_seed: String    //Initial encryption seed passed by owner
) -> StdResult<InitResponse> {
    let state = State {
        owner: deps.api.canonical_address(&env.message.sender)?,
    };




    config(&mut deps.storage).save(&state)?;

    Ok(InitResponse::default())
}







//CardID + Values (should maybe be in init function????)
let cards = HashMap::new();





//Diamonds
cards.insert(1, 11);    //Ace
cards.insert(2, 2);
cards.insert(3, 3);
cards.insert(4, 4);
cards.insert(5, 5);
cards.insert(6, 6);
cards.insert(7, 7);
cards.insert(8, 8);
cards.insert(9, 9);
cards.insert(10, 10);   //Ten
cards.insert(11, 10);   //Jack
cards.insert(12, 10);   //Queen
cards.insert(13, 10);   //King

//Hearts
cards.insert(14, 11);   //Ace
cards.insert(15, 2);
cards.insert(16, 3);
cards.insert(17, 4);
cards.insert(18, 5);
cards.insert(19, 6);
cards.insert(20, 7);
cards.insert(21, 8);
cards.insert(22, 9);
cards.insert(23, 10);   //Ten
cards.insert(24, 10);   //Jack
cards.insert(25, 10);   //Queen
cards.insert(26, 10);   //King

//Clubs
cards.insert(27, 11);   //Ace
cards.insert(28, 2);
cards.insert(29, 3);
cards.insert(30, 4);
cards.insert(31, 5);
cards.insert(32, 6);
cards.insert(33, 7);
cards.insert(34, 8);
cards.insert(35, 9);
cards.insert(36, 10);   //Ten
cards.insert(37, 10);   //Jack
cards.insert(38, 10);   //Queen
cards.insert(39, 10);   //King

//Spades
cards.insert(40, 11); //Ace
cards.insert(41, 2);
cards.insert(42, 3);
cards.insert(43, 4);
cards.insert(44, 5);
cards.insert(45, 6);
cards.insert(46, 7);
cards.insert(47, 8);
cards.insert(48, 9);
cards.insert(49, 10);   //Ten
cards.insert(50, 10);   //Jack
cards.insert(51, 10);   //Queen
cards.insert(52, 10);   //King


//Max value of dealer hand before they stay
let dealer_max:u8 = 17;

//Max hand Value
const BLACKJACK:u8 = 21;

//Payout Multiples
const NAT_BLACK:f32 = 2.5;
const NORM_PAY:f32 = 2;




//All traits for all hands dealt
struct Hand {
    contents: Vec<u8>,
    val: u8,
    ace: bool,

    stay: bool,
    blackjack: bool,
    bust: bool
}

//Takes self, and new_card, comes from car_draw() except in secret_card case
impl Hand {
    fn hit(&self, new_card: u8) {
        self.contents.push(new_card);
        self.val += cards.get(new_card);


        //ACE MANAGEMENT
        if cards.get(new_card) == 11 && self.ace == false {
            self.ace = true;
        }
        //If player already has reducable ace and recieves another, reduces first ace
        else if self.ace == true && cards.get(new_card) == 11 {
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
    hand: Hand,

    secret_card: u8,

}

struct Player {
    hand: Hand,

    did_split: bool,
    split_hand: Hand,

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
    dealer: Dealer,
    player: Player,

    //The money the player has on the line
    wager: u64,


    opening_done: bool,

    //Insurance round indicator. No other actions can be called while this is true until insurance round is resolved
    insurance_round: bool

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
    if cards.get(table.dealer.hand[0]) == 10 || cards.get(table.dealer.hand[0]) == 11 {
        table.insurance_round = true;
    }



    table.opening_done = true;


    if table.player.val == BLACKJACK && table.insurance_round == false {
        dealer_turn();
    }

}


//Insurance Round. Can only be called if insurance_round == true
//Player must pay in half his wager
pub fn insure() {
    if table.insurance_round == true {


        if table.dealer.hand.val + cards.get(table.dealer.secret_card) == 21 {
            //Player recieves a payment equal to his wager (TODO)
            end_round();

        }
        table.insurance_round == false;
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
    }
    else if table.player.hand.stay == false &&
    table.player.hand.blackjack == false &&
    table.player.hand.bust == false {
        table.player.hand.hit(card_draw());
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
        table.player.hand.val = cards.get(table.player.hand.contents[0]);

    }

}



//Dealer takes turn
fn dealer_turn() {
    //Secret card is moved into dealers hand
    table.dealer.hand.hit(table.dealer.secret_card);

    while table.dealer.hand.val < dealer_max {
        table.dealer.hand.hit(card_draw);
    }

    end_round();
}



pub fn end_round() {
}


//generates a random number between 1-52, returns a u8
fn card_draw() -> u8 {

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



//Declares all the functions that change state
pub enum HandleMsg {
    hit {},
    stand {},
    double_down {},
    split {},
    insurance_round {},
    start_round {},

}



//handle function must be kept
pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Increment {} => try_increment(deps, env),
        HandleMsg::Reset { count } => try_reset(deps, env, count),
    }
}

//query functions must be kept
pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetCount {} => to_binary(&query_count(deps)?),
    }
}

fn query_count<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<CountResponse> {
    let state = config_read(&deps.storage).load()?;
    Ok(CountResponse { count: state.count })
}
