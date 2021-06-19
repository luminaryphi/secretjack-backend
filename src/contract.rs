use std::env;

//cosmwasm_std must be kept
use cosmwasm_std::{Api, Binary, Env, Extern, HandleResponse, HandleResult, InitResponse, Querier, StdError, StdResult, Storage, Uint128, to_binary};

use crate::msg::{CountResponse, HandleMsg, InitMsg, QueryMsg};
use crate::state::{config, config_read, State};



//These are for random num generation
use rand_chacha::ChaChaRng;
use rand::{RngCore, SeedableRng};


//Init contains admin seed and contract owner inf
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
pub const NORM_PAY:u8 = 2;

//Charlie victory constant
pub const CHARLIE:u8 = 6;



//All traits for all hands dealt
struct Hand {
    pub contents: Vec<u8>,
    pub val: u8,
    pub ace: bool,

    pub stay: bool,
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


    }

    fn reset(&self) {
        self.contents.clear();
        self.val = 0;
        self.ace = false;

        self.stay = false;
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

    //players burner wallet
    wallet: u64,


    pub opening_done: bool,

    //Insurance round indicator. No other actions can be called while this is true until insurance round is resolved
    pub insurance_round: bool,



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





//Sending money to game check, ensures everything is correct for wagers/insurance/doubling down etc
pub fn deposit_check(
    env: &Env, 
    required_amount: u64, 
    max_bet: u64
) -> StdResult<u64> {
    let deposit: Uint128;

    if env.message.sent_funds.len() == 0 {
        return Err(StdError::generic_err("TRANSACTION NOT MADE!"));
    } else {
        if env.message.sent_funds[0].denom != "uscrt" {
            return Err(StdError::generic_err("YOU'VE USED THE WRONG CURRENCY!"));
        }
        deposit = env.message.sent_funds[0].amount;

        if deposit < required_amount {
            return Err(StdError::generic_err("NOT ENOUGH FUNDS SENT!"));
        }
        else if deposit > max_bet {
            return Err(StdError::generic_err("YOU CAN'T WAGER THAT MUCH!"))
        }
    }

}


/// Returns StdResult<T> from retrieving the item with the specified key.  Returns a
/// StdError::NotFound if there is no item with that key
///
/// # Arguments
///
/// * `storage` - a reference to the storage this item is in
/// * `key` - a byte slice representing the key that accesses the stored item
pub fn load<T: DeserializeOwned, S: ReadonlyStorage>(storage: &S, key: &[u8]) -> StdResult<T> {
    Bincode2::deserialize(
        &storage
            .get(key)
            .ok_or_else(|| StdError::not_found(type_name::<T>()))?,
    )
}




//Player starts with initial bet
//give player 2 cards, dealer recieves one card. Other is hidden
pub fn start_round<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> HandleResult {

    //STAND IN FOR MAX_BET AND REQUIRED_AMOUNT
    let required_amount = 1;
    let max_bet = 1;

    //Checks if player wagered within allowed range
    deposit_check(&env, required_amount, max_bet);

    //Load Table from sender waller address
    let sender_raw = deps.api.canonical_address(&env.message.sender)?; //Grabs human address, turns it into cannonical address
    let sender_key = sender_raw.as_slice(); //Makes address into a key that storage can understand
    let mut table: Table = load(&deps.storage, sender_key)?; //Loads table from storage based on the sender_key from accessing wallet

    //Ensure table is clear prior to round
    table.reset();

    //Give players 2 cards
    for n in 1..=2 {
        table.player.hand.hit(card_draw(&env));
    }



    //Give dealer one card, then give secret card
    table.dealer.hand.hit(card_draw(&env));
    table.dealer.secret_card = card_draw(&env);

    //If value of dealers first card is 10 or 11, allow insurance option
    if card_value(table.dealer.hand.contents[0]) == 11 {
        table.insurance_round = true;
    }



    table.opening_done = true;


    if table.player.hand.val == BLACKJACK && table.insurance_round == false {
        end_round(&table, &env)
    }

    else {
    //Returns game state after opening
    return Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::Open {
            player_hand: table.player.hand.contents,
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
pub fn insure<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env
) -> HandleResult {

    //Load Table from sender waller address
    let sender_raw = deps.api.canonical_address(&env.message.sender)?; //Grabs human address, turns it into cannonical address
    let sender_key = sender_raw.as_slice(); //Makes address into a key that storage can understand
    let mut table: Table = load(&deps.storage, sender_key)?; //Loads table from storage based on the sender_key from accessing wallet


    if table.insurance_round == true {


        //Checks if player insurance is the correct amount
        deposit_check(&env, table.wager/2, table.wager/2);


        //Dealer has blackjack
        if table.dealer.hand.val + card_value(table.dealer.secret_card) == 21 {
            //Shows secret card
            table.dealer.hand.hit(table.dealer.secret_card);
            //Player recieves a payment equal to his wager (TODO)
            end_round(&table, &env)

        }

        //Dealer didnt have blackjack
        else {
        table.insurance_round == false;

            //Returns Game state
            return Ok(HandleResponse {
                messages: vec![],
                log: vec![],
                data: Some(to_binary(&HandleAnswer::Insure {

                })?),
            });
        }
    }
    else {
        return Err(StdError::generic_err("YOU CAN'T INSURE NOW"));
    }
}


//If insurance round is called, player is given option to pass up insurance
pub fn dont_insure<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env
) -> HandleResult {

    //Load Table from sender waller address
    let sender_raw = deps.api.canonical_address(&env.message.sender)?; //Grabs human address, turns it into cannonical address
    let sender_key = sender_raw.as_slice(); //Makes address into a key that storage can understand
    let mut table: Table = load(&deps.storage, sender_key)?; //Loads table from storage based on the sender_key from accessing wallet



    if table.insurance_round == true {

        //Dealer has blackjack
        if table.dealer.hand.val + card_value(table.dealer.secret_card) == 21 {
            //Shows secret card
            table.dealer.hand.hit(table.dealer.secret_card);
            //Player recieves a payment equal to his wager ( TODO )
            end_round(&table, &env)

        }

        //Dealer Didn't have blackjack
        else {
            table.insurance_round == false;

            //Returns Game state
            return Ok(HandleResponse {
                messages: vec![],
                log: vec![],
                data: Some(to_binary(&HandleAnswer::Insure {

                })?),
            })
        }
    }

}




//Function for player that hits either his split hand or regular hand
pub fn hit<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> HandleResult {


    //Load Table from sender waller address
    let sender_raw = deps.api.canonical_address(&env.message.sender)?; //Grabs human address, turns it into cannonical address
    let sender_key = sender_raw.as_slice(); //Makes address into a key that storage can understand
    let mut table: Table = load(&deps.storage, sender_key)?; //Loads table from storage based on the sender_key from accessing wallet

    //Check if insurance round needs to be resolved
    if table.insurance_round == true {
        return Err(StdError::generic_err("INSURANCE ROUND MUST BE RESOLVED!"));
    }

    //Split hand hit
    else if table.player.did_split == true &&
    table.player.split_hand.stay == false &&
    table.player.split_hand.val < BLACKJACK  &&
    table.player.split_hand.contents.len() < CHARLIE as usize {
        table.player.split_hand.hit(card_draw(&env));

        //Returns last card in split hand and new split val
        return Ok(HandleResponse {
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
    table.player.hand.val < BLACKJACK &&
    table.player.hand.contents.len() < CHARLIE as usize {

        table.player.hand.hit(card_draw(&env));


        //If player main hand busts or reaches 21 or CHARLIE, call dealer turn
        if table.player.hand.val >= BLACKJACK ||
        table.player.hand.contents.len() >= CHARLIE as usize {
            dealer_turn(&table, &env)
        }

        else {
            //Returns last card in normal hand and new hand value
            return Ok(HandleResponse {
                messages: vec![],
                log: vec![],
                data: Some(to_binary(&HandleAnswer::Hit {
                    new_card: table.player.split_hand.contents[(table.player.split_hand.contents.len())-1],
                    new_val: table.player.split_hand.val,

                    which_hand: false   //False means this came from normal hand
                })?),
            })
        }


    }

}



//Ends players turn
pub fn stand<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> HandleResult {

    //Load Table from sender waller address
    let sender_raw = deps.api.canonical_address(&env.message.sender)?; //Grabs human address, turns it into cannonical address
    let sender_key = sender_raw.as_slice(); //Makes address into a key that storage can understand
    let mut table: Table = load(&deps.storage, sender_key)?; //Loads table from storage based on the sender_key from accessing wallet


    //Check if insurance round needs to be resolved
    if table.insurance_round == true {
        return Err(StdError::generic_err("INSURANCE ROUND MUST BE RESOLVED!"));
    }


    //If player is using split hand and hasn't already stayed it
    if table.player.did_split == true && table.player.split_hand.stay == false {
        table.player.split_hand.stay = true;
    }
    //If player is using normal hand, move to dealer turn
    else {
        table.player.hand.stay = true;
        dealer_turn(&table, &env)
    }


}


//Player doubles bet and adds one card
pub fn double_down<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> HandleResult {

    //Load Table from sender waller address
    let sender_raw = deps.api.canonical_address(&env.message.sender)?; //Grabs human address, turns it into cannonical address
    let sender_key = sender_raw.as_slice(); //Makes address into a key that storage can understand
    let mut table: Table = load(&deps.storage, sender_key)?; //Loads table from storage based on the sender_key from accessing wallet

    //Check if insurance round needs to be resolved
    if table.insurance_round == true {
        return Err(StdError::generic_err("INSURANCE ROUND MUST BE RESOLVED!"))
    }


    //Checks if player sent the appropriate amount
    deposit_check(&env, table.wager, table.wager/2);


    else if table.player.hand.contents.len() == 2 {
        table.wager *= 2;
        table.player.hand.hit(card_draw());
        stand();
        //Handle answer will occur in in end_round
    }

    else {
        return Err(StdError::generic_err("YOU CAN'T DOUBLE DOWN NOW!"))
    }
}


pub fn split<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> HandleResult {

    //Load Table from sender waller address
    let sender_raw = deps.api.canonical_address(&env.message.sender)?; //Grabs human address, turns it into cannonical address
    let sender_key = sender_raw.as_slice(); //Makes address into a key that storage can understand
    let mut table: Table = load(&deps.storage, sender_key)?; //Loads table from storage based on the sender_key from accessing wallet


    //Check if insurance round needs to be resolved
    if table.insurance_round == true {
        return Err(StdError::generic_err("INSURANCE ROUND MUST BE RESOLVED!"))
    }



    //If players first 2 cards are the same, player hasn't added cards, and hasn't already split
    if card_value(table.player.hand.contents[0]) == card_value(table.player.hand.contents[1]) &&
    table.player.did_split == false &&
    table.player.hand.contents.len() == 2 {

        table.player.did_split = true;

        //Takes card from player hand, moves to split hand, sets hand to value of the one card
        table.player.split_hand.hit(table.player.hand.contents[1]);
        table.player.hand.contents.pop();
        table.player.hand.val = card_value(table.player.hand.contents[0]);

        //Adds one card to each hand
        table.player.split_hand.hit(card_draw(&env));
        table.player.hand.hit(card_draw(&env));


        //Returns the value and contents of normal and split hands
        return Ok(HandleResponse {
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
fn dealer_turn
    (table: &Table,
    env: &Env
) -> HandleResult {



    //Secret card is moved into dealers hand
    table.dealer.hand.hit(table.dealer.secret_card);

    //Dealer will stop on 18 and also 17 if he doesn't have an ace
    while (table.dealer.hand.val < DEALER_STOP) ||
    (table.dealer.hand.val <= DEALER_STOP && table.dealer.hand.ace == true) {
        table.dealer.hand.hit(card_draw(&env));
    }

    end_round(&table, &env)
}



fn end_round
    (table: &Table,
    env: &Env
) -> HandleResult {




    //Player gets blackjack and dealer does not
    if (table.player.hand.val == 21 &&
    table.player.hand.contents.len() == 2 &&
    table.player.did_split == false) &&
    (table.dealer.hand.val != 21 ||
    table.dealer.hand.contents.len() != 2) {
        payout(INSERTCONTRACTADDRESS, &env.message.sender, table.wager * NAT_BLACK);
    }

    //Player and dealer both have blackjack, player gets money back
    else if (table.player.hand.val == 21 &&
    table.player.hand.contents.len() == 2 &&
    table.player.did_split == false) &&
    (table.dealer.hand.val == 21 &&
    table.dealer.hand.contents.len() == 2) {
        payout(INSERTCONTRACTADDRESS, &env.message.sender, table.wager);
    }

    //Non blackjack wins count. Winning hands used as a multiple for winnings if player split
    else {
        let mut winning_hands: u8 = 0;

        //Split hand win > dealer and not bust or dealer bust and player didn't
        if (table.player.did_split == true &&                           //Player Hand > Dealer Hand Win
            table.player.split_hand.val > table.dealer.hand.val &&
            table.player.split_hand.val <= BLACKJACK) ||
            (table.dealer.hand.val > BLACKJACK &&                       //Dealer Bust / Player Didn't Win
            table.player.split_hand.val <= BLACKJACK) ||
            (table.player.split_hand.val <= BLACKJACK &&                //CHARLIE Win
            table.player.split_hand.contents.len() >= CHARLIE as usize) {
                    winning_hands += 1;
        }


        //Regular hand win
        if (table.player.hand.val > table.dealer.hand.val &&    //Player Hand > Dealer Hand Win
            table.player.hand.val <= BLACKJACK) ||
            (table.dealer.hand.val > BLACKJACK &&               //Dealer Bust / Player Didn't Win
             table.player.hand.val <= BLACKJACK) ||
             (table.player.hand.val <= BLACKJACK &&             //CHARLIE Win
             table.player.hand.contents.len() >= CHARLIE as usize) {
                winning_hands += 1;
            }

        //Player has 1 or two normal wins
        if winning_hands > 0 {
            payout(INSERTCONTRACTADDRESS, &env.message.sender, table.wager * NORM_PAY * winning_hands);

        }

        //Player Lost
        else {

        }

    }






    //Returns the cards drawn by the dealer and end game result
    Ok(HandleResponse {
    messages: vec![],
    log: vec![],
    data: Some(to_binary(&HandleAnswer::Conclude {
        dealer_hand: table.dealer.hand.contents,
        dealer_val: table.dealer.hand.val,

        })?),
    })

}



//generates a random number between 0-51, returns a u8
fn card_draw
    (env: &Env,
    
) -> u8 {

    //Blends admin seed with blockheight and time (Later add player seed)
    let entropy = state.seed;
    entropy.extend_from_slice(&env.block.height.to_be_bytes());
    entropy.extend_from_slice(&env.block.time.to_be_bytes());
    entropy.extend_from_slice(&env.message.sender.0.as_bytes());

    //Takes entropy blend and generates new seed
    let mut random_seed: [u8; 32] = Sha256::digest(entropy).into();


    let mut rng = ChaChaRng::from_seed(random_seed);

    let mut num = ((rng.next_u32() % 52) as u8); // a number between 0 and 51



    //If num is on the edge of 32 bit numbers, retry (32 bit max not evenly divisible by 52)
    while num >= 4294967248 {
        num = ((rng.next_u32() % 52) as u8);
    }

    return num;
}

fn card_value(card: u8) -> u8 {
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

//Player recieves money THIS IS NO LONGER NECESSARY! FUNCTIONS ARE NOW IN ENDGAME AND WITHDRAW!!!

//Takes to/from addresses and amount
fn payout (
    contract_address: HumanAddr,
    player_address: HumanAddr,
    ammount: Uint128,
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
