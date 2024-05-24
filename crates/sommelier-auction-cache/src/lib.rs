use std::{collections::HashMap, sync::Arc};

use eyre::Result;
use lazy_static::lazy_static;
use sommelier_auction::client::Client;
use sommelier_auction_proto::auction::Auction as AuctionProto;
use tokio::sync::RwLock;

pub type Cache<T> = Arc<RwLock<T>>;

pub const USOMM: &str = "usomm";

lazy_static! {
    pub(crate) static ref ACTIVE_AUCTIONS: Cache<HashMap<u32, AuctionProto>> = Cache::default();
}

pub async fn run(grpc_endpoint: &str) -> Result<()> {
    let mut client = Client::with_endpoints("".to_string(), grpc_endpoint.to_string()).await?;

    loop {
        refresh_active_auctions(&mut client).await?;

        tokio::time::sleep(tokio::time::Duration::from_secs(6)).await;
    }
}

async fn refresh_active_auctions(client: &mut Client) -> Result<()> {
    let auctions = client.active_auctions().await?;
    let mut active_auctions = ACTIVE_AUCTIONS.write().await;
    for auction in auctions {
        active_auctions.insert(auction.id, auction);
    }

    Ok(())
}

pub async fn get_active_auctions() -> Result<Vec<AuctionProto>> {
    let active_auctions = ACTIVE_AUCTIONS.read().await;

    Ok(active_auctions.values().cloned().collect())
}
