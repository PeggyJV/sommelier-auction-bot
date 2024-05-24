use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
/// The parameters for an auction.
pub struct AuctionParameters {
    /// If there are qualifying fees, auctions occur when block_height % auction_interval == 0
    pub auction_interval: u64,
    /// The minimum number of accruals that must occur for a fee denom to be auctioned
    pub fee_accrual_auction_threshold: u64,
    /// Frequency of auction uSOMM price decrease in blocks
    pub price_decrease_block_interval: u64,
    /// The initial price decrease rate for auction uSOMM price
    pub initial_price_decrease_rate: f64,
    /// Acceleration factor for the price decrease rate
    pub auction_price_decrease_acceleration_rate: f64,
    /// The minimum total uSOMM required for a bid to be valid
    pub minimum_bid_in_usomm: u64,
    /// The minimum total usd value of fee tokens requested in a bid
    pub minimum_sale_tokens_usd_value: u64,
}
