use wf_inventory::Inventory;

use crate::catalog::Catalog;
use crate::favourites::Favourites;
use crate::listings::MarketListings;
use crate::prices::PriceSource;

#[derive(Clone, Copy)]
pub(crate) struct View<'a> {
    pub(crate) inventory: &'a Inventory,
    pub(crate) catalog: &'a Catalog,
    pub(crate) prices: &'a dyn PriceSource,
    pub(crate) favourites: &'a Favourites,
    pub(crate) listings: &'a MarketListings,
}
