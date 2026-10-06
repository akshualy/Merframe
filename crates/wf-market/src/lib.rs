mod error;
mod listings;
mod models;
mod parse;

#[cfg(feature = "fetch")]
mod bulk;
#[cfg(feature = "fetch")]
mod client;
#[cfg(feature = "fetch")]
mod ratelimit;
#[cfg(feature = "fetch")]
mod ws;

pub use error::{MarketError, Result};
pub use listings::{ItemListings, Reach, TraderStatus, item_listings};
pub use models::{
    Auction, AuctionItem, Chat, CloseOrderRequest, CreateAuctionItem, CreateAuctionRequest,
    CreateOrderRequest, GoodRoll, GoodRollAlternative, Item, ItemLocalization, Order, OrderType,
    OrdersGroupUpdate, Platform, Polarity, PriceEntry, PriceTable, Rarity, RivenAttribute,
    RivenAttributeInstance, RivenAttributeLocalization, RivenAuction, RivenAuctionAttribute,
    RivenData, RivenWeapon, Session, SetAuctionsVisibilityRequest, SetGroupVisibilityRequest,
    SignInRequest, StatRef, UpdateAuctionRequest, UpdateOrderRequest, User, UserPrivate,
    UserStatus, WeaponAuctions,
};
pub use parse::{
    envelope, parse_chats, parse_price_table, parse_riven_auctions, parse_riven_data,
    parse_v1_auction, parse_v1_auctions, v1_payload,
};

#[cfg(feature = "fetch")]
pub use bulk::{Etagged, bulk_prices, bulk_riven_data, riven_auctions};
#[cfg(feature = "fetch")]
pub use client::Client;
#[cfg(feature = "fetch")]
pub use error::order_rejection;
#[cfg(feature = "fetch")]
pub use ws::{
    AuthSignInPayload, Envelope, IncomingEvent, MarketSocket, StatusSetEventPayload,
    StatusSetPayload, parse_event,
};
