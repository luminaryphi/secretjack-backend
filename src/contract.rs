//cosmwasm_std must be kept
use cosmwasm_std::{
    to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, Querier, StdError,
    StdResult, Storage,
};

use crate::msg::{CountResponse, HandleMsg, InitMsg, QueryMsg};
use crate::state::{config, config_read, State};

//allows us to use hashmaps
use std::collections::HashMap;

//Allows us to abort functions



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




struct Table {
    //Dealer hand and hand value
    dealer_hand: Vec<u8>,
    dealer_secret_card: u8, //Dealers opening card that stays hidden at first
    dealer_val: u8,

    dealer_ace: bool, //True if dealer has reducable ace

    dealer_bust: bool,
    dealer_stay: bool,

    player_address: i64,

    //Player hand and hand value
    player_hand: Vec<u8>,
    player_val: u8,

    //Tells if player has an ace that can be reduced if val is above 21
    player_ace: bool,

    //player split bool
    //player split hand
    //player split hand value u8


    player_bust: bool,
    player_stay: bool,
    player_blackjack: bool,

    player_bet: u64,

    //True if the opening hands have been drawn.
    opening_done: bool,

    //Insurance round indicator. No other actions can be called while this is true until insurance round is resolved
    insurance_round: bool,


}


//Player reference and storage key
let sender_raw = deps.api.canonical_address(&env.message.sender)?; //Grabs human address, turns it into cannonical address
let sender_key = sender_raw.as_slice(); //Makes address into a key that storage can understand
let mut table: Table = load(&deps.storage, sender_key)?; //Loads table from storage based on the sender_key from accessing wallet




//Declares all the functions that change state
pub enum HandleMsg {
    hit {},
    stand {},
    double_down {},
    split {},
    insurance_round {},
    start_round {},

}

//Player starts with initial bet
//give player 2 cards, dealer recieves one card. Other is hidden
pub fn start_round() {

    //Give players 2 cards
    for n in 1..=2 {
        hit();

    }

    //Dealer recieves 2 cards, one hidden
    let new_card: u8 = card_draw();
    table.dealer_hand.push(new_card);
    table.dealer_val += cards.get(new_card);

    if cards.get(new_card) == 11 {
        dealer_ace = true;
    }

    //Assigns hidden card
    table.dealer_secret_card = card_draw();

    //If dealers first card is 10 value or ace, sets insurance round
    if cards.get(new_card) == 10 || cards.get(new_card) == 11 {
        table.insurance_round = true;
    }




    table.opening_done = true;


    if table.blackjack == true && table.insurance_round == false {
        dealer_turn();
    }

}


//Insurance Round. Can only be called if insurance_round == true
//Player must pay in half his wager
pub fn insure() {
    if table.insurance_round == true {


        if table.dealer_val + cards.get(dealer_secret_card) == 21 {
            //Player recieves a payment equal to his wager (TODO)
            end_round();

        }
        table.insurance_round == false;
    }
    else {
        //Throw error. Player cannot place insurance
    }
}


//Checks if player has ended turn by staying or busting.
fn end_check() -> bool {
    if table.player_bust || table.player_stay {
        return true;
    }
    else {
        return false;
    }
}



//FUNCTIONS FOR ACTIONS

pub fn hit() {
    //Adding 1 card to hand
    let new_card: u8 = card_draw();
    table.player_hand.push(new_card);
    table.player_val += cards.get(new_card);


    //ACE MANAGEMENT

    //If player gets ace and doesn't have one, sets ace bool to true
    if table.player_ace == false && cards.get(new_card) == 11 {
        table.player_ace == true;
    }
    //If player already has reducable ace and recieves another, reduces first ace
    else if table.player_ace == true && cards.get(new_card) == 11 {
        table.player_val -= 10;
    }



    //END CHECKS

    //If player is over 21 has reducable ace, reduce, otherwise, bust
    if player_val > BLACKJACK {
        if table.player_ace == true {
            player_val -= 10;
            table.player_ace = false;
        }
        else {
            table.player_bust = true;
            end_round();
        }
    }


    //Test blackjack
    else if table.player_val == BLACKJACK {
        //Tests for natural blackjack
        if table.player_hand.len() == 2 {
            table.player_blackjack = true;
        }

        if opening_done == true {
            dealer_turn();
        }
    }

}


//Ends players turn
pub fn stand() {
    table.player_stay = true;
    dealer_turn();

}


//Player doubles bet and adds one card
pub fn double_down() {
    if !end_check() {
        table.player_bet *= 2;
        hit();
        stand();
    }
}


pub fn split() {
    //Only optional if player recieves 2 of the same card from dealer
    //Player splits hand into 2, gives second hand an equal bet.
    //
}



//Dealer takes turn
fn dealer_turn() {
    //Dealers secret card is added to his hand
    table.dealer_hand.push(table.dealer_secret_card);
    table.dealer_val += cards.get(table.dealer_secret_card);

    //Checks if first 2 cards are double aces and reduces before hit cycle
    if table.dealer_ace == true && cards.get(table.dealer_secret_card) == 11 {
        table.dealer_val -= 10;
    }

    //Hits until dealer reaches his max value
    if table.dealer_val < table.dealer_max {
        while table.dealer_val < table.dealer_max {
            let new_card = card_draw();
            table.dealer_hand.push(new_card);
            table.dealer_val += cards.get(new_card);

            //Ace management
            //If dealer recieves second ace, reduce the first
            if table.dealer_ace == true && cards.get(new_card) == 11 {
                table.dealer_val -= 10;
            }
            //If dealer passes 21 but has reducable ace, reduce
            else if table.dealer_ace == true && table.dealer_val > BLACKJACK {
                table.dealer_val -= 10;
                table.dealer_ace = false;
            }


        }
        if table.dealer_val > BLACKJACK {
            table.dealer_bust = true;
        }
    }

    end_round();
}



pub fn end_round() {
    //If player bust or stand is true && dealer bust or stand is true
        //if player is not bust && player hand value > dealer hand value || dealer is bust but player is not
            //player wins!
        //else if dealer is not bust and dealer hand value > player hand value || player bust but dealer is not
            //Dealer wins
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










//Init function must be kept
pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let state = State {
        owner: deps.api.canonical_address(&env.message.sender)?,
    };

    config(&mut deps.storage).save(&state)?;

    Ok(InitResponse::default())
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



}
