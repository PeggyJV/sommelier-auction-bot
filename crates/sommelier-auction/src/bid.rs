use serde::{Deserialize, Serialize};

use crate::denom::Denom;

/// Represents an order for one auction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bid {
    /// The ID of the auction to submit big to
    pub auction_id: u32,
    /// The denomination of the token being auctioned
    pub fee_token: Denom,
    /// The most usomm (1,000,000 usomm = 1 SOMM) the bidder is willing to pay
    pub maximum_usomm_in: u128,
    /// The minimum units of fee token the bidder is willing to receive
    pub minimum_tokens_out: u128,
}
