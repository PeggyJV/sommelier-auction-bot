use std::collections::HashMap;

use eyre::Result;
use sommelier_auction::{auction::Auction, client::Client, bid::Bid, denom::Denom};
use tokio::sync::mpsc::Sender;

use crate::order::Order;

// This is a temporary type to house the auction monitoring function so we can 
// spawn a thread to run it. In the future we should think about a generalized 
// "Strategy" trait that has a Sender<Bid> and decides when to send a bid over
// the channel. The OrderEngine could then take in an arbitrary strategy, run it,
// and relay bids sent over the channel to a bidder service.
pub struct Watcher {
    active_auctions: Vec<Auction>,
    client: Option<Client>,
    grpc_endpoint: String,
    orders: HashMap<Denom, Vec<Order>>,
    prices: HashMap<Denom, f64>
}

impl Watcher {
    pub fn new(orders: HashMap<Denom, Vec<Order>>, grpc_endpoint: String) -> Self {
        Self {
            active_auctions: Vec::new(),
            client: None,
            grpc_endpoint,
            orders,
            prices: HashMap::new()
        }
    }

    pub fn update_prices(&mut self, prices: HashMap<Denom, f64>) {
        self.prices = prices;
    }

    async fn refresh_active_auctions(&mut self) -> Result<()> {
        let active_auctions = self.client.as_mut().unwrap().active_auctions().await?;
        self.active_auctions = active_auctions;

        Ok(())
    }

    pub async fn monitor_auctions(&mut self, tx: Sender<Bid>) -> Result<()> {
        self.client = Some(Client::with_endpoints("".to_string(), self.grpc_endpoint.clone()).await?);

        loop {
            self.refresh_active_auctions().await?;

            // for each active auction, check if any orders qualify for a bid
            for auction in &self.active_auctions {
                let auction_denom = match Denom::try_from(auction.starting_tokens_for_sale.clone().unwrap().denom.clone()) {
                    Ok(d) => d,
                    Err(_) => {
                        // log error
                        continue
                    }
                };
                if let Some(orders) = self.orders.get(&auction_denom) {
                    for order in orders {
                        // if we don't have a usd price for the token, move on
                        if let Some(usd_unit_value) = self.prices.get(&auction_denom) {
                            if let Some(bid) = self.evaluate_bid(&order, *usd_unit_value, &auction) {
                                // submit bid
                            }
                        } else {
                            // log
                        }
                    }
                }
            }

            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    }

    // Collin: Currently not checking USOMM price in USD and thus not guaranteeing a profitable
    // arbitrage. We're simply checking how much USD value we can get out with the max possible
    // USOMM offer. 
    fn evaluate_bid(&self, order: &Order, usd_unit_value: f64, auction: &Auction) -> Option<Bid> { 
        let auction_unit_price_in_usomm = auction.current_unit_price_in_usomm.parse::<f64>().unwrap();
        let remaining_tokens_for_sale = auction.remaining_tokens_for_sale.clone().unwrap().amount.parse::<u64>().unwrap();

        // the auction will give us the best possible price which makes this simpler
        let max_allowed_usomm_offer = order.maximum_usomm_in; 
        let min_possible_token_out = std::cmp::min((max_allowed_usomm_offer as f64 / auction_unit_price_in_usomm) as u64, remaining_tokens_for_sale);
        let usd_value_out = min_possible_token_out as f64 * usd_unit_value;
    
        if order.minimum_usd_value_out as f64 <= usd_value_out {
            return Some(Bid {
                auction_id: auction.id.clone(),
                fee_token: order.fee_token.clone(),
                maximum_usomm_in: max_allowed_usomm_offer,
                minimum_tokens_out: min_possible_token_out,
            })
        }

        None
    }
}
