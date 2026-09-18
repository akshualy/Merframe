use crate::error::{MarketError, Result};
use crate::models::{ApiEnvelope, Auction, Chat, PriceTable, RivenData, V1Payload, WeaponAuctions};

pub fn envelope<T>(json: &str) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let envelope: ApiEnvelope<T> = serde_json::from_str(json)?;
    envelope.data.ok_or(MarketError::MissingData)
}

pub fn v1_payload<T>(json: &str) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let payload: V1Payload<T> = serde_json::from_str(json)?;
    Ok(payload.payload)
}

#[derive(Debug, serde::Deserialize)]
struct V1AuctionPayload {
    auction: Auction,
}

#[derive(Debug, serde::Deserialize)]
struct V1AuctionsPayload {
    auctions: Vec<Auction>,
}

pub fn parse_v1_auction(json: &str) -> Result<Auction> {
    Ok(v1_payload::<V1AuctionPayload>(json)?.auction)
}

pub fn parse_v1_auctions(json: &str) -> Result<Vec<Auction>> {
    Ok(v1_payload::<V1AuctionsPayload>(json)?.auctions)
}

#[derive(Debug, serde::Deserialize)]
struct V1ChatsPayload {
    chats: Vec<Chat>,
}

pub fn parse_chats(json: &str) -> Result<Vec<Chat>> {
    Ok(v1_payload::<V1ChatsPayload>(json)?.chats)
}

pub fn parse_price_table(json: &str) -> Result<PriceTable> {
    Ok(serde_json::from_str(json)?)
}

pub fn parse_riven_data(json: &str) -> Result<RivenData> {
    Ok(serde_json::from_str(json)?)
}

pub fn parse_riven_auctions(json: &str) -> Result<WeaponAuctions> {
    Ok(serde_json::from_str(json)?)
}
