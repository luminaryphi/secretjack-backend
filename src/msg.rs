use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub admin_seed: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Hit {},
    Stand {},
    Double_Down {},
    Split {},
    Start_Round {},
    Insure {},
    Dont_Insure {},

}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleAnswer {
    Open {
        player_hand: Vec,
        player_val: u8,

        dealer_hand: Vec,
        dealer_val: u8,

        insureable: bool,

    },
    Hit {
        last_card: u8,
        new_val: u8,
        which_hand: bool, //True will be split, false will be normal

    },
    Split {
        player_hand: Vec,
        player_val: u8,

        split_hand: Vec,
        split_val: u8
    },

    Insure {

    },

    Conclude {
        dealer_hand: Vec,
        dealer_val: u8,
    },


}





#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {

}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CountResponse {
}
