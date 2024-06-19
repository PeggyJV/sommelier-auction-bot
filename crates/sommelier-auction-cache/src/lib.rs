use std::{collections::HashMap, sync::Arc};

use eyre::Result;
use lazy_static::lazy_static;
use sommelier_auction::client::Client;
use sommelier_auction_proto::auction::Auction as AuctionProto;
use tokio::sync::RwLock;
use tracing::{error, info};

pub type Cache<T> = Arc<RwLock<T>>;

pub const USOMM: &str = "usomm";

lazy_static! {
    pub(crate) static ref ACTIVE_AUCTIONS: Cache<HashMap<u32, AuctionProto>> = Cache::default();
}

pub async fn run(grpc_endpoint: String) -> Result<()> {
    info!("creating query client with endpoint {grpc_endpoint}");
    let mut client = match Client::with_endpoints("".to_string(), grpc_endpoint).await {
        Ok(c) => c,
        Err(err) => {
            error!("failed to create query client: {err:?}");
            return Err(err);
        }
    };

    loop {
        if let Err(err) = refresh_active_auctions(&mut client).await {
            error!("failed to refresh active auctions: {err:?}");
        };

        tokio::time::sleep(tokio::time::Duration::from_secs(6)).await;
    }
}

async fn refresh_active_auctions(client: &mut Client) -> Result<()> {
    info!("refreshing active auctions");
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
