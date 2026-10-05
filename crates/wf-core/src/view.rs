use crate::account::Account;
use crate::catalog::Catalog;
use crate::favourites::Favourites;
use crate::identity::ItemTable;
#[cfg(test)]
use crate::inventory_view::ItemIndex;
use crate::listings::MarketListings;
#[cfg(test)]
use crate::prices::FixedPrices;
use crate::prices::PriceSource;

#[derive(Clone, Copy)]
pub(crate) struct View<'a> {
    pub(crate) account: &'a Account,
    pub(crate) catalog: &'a Catalog,
    pub(crate) items: &'a ItemTable,
    pub(crate) prices: &'a dyn PriceSource,
    pub(crate) favourites: &'a Favourites,
    pub(crate) listings: &'a MarketListings,
}

#[cfg(test)]
pub(crate) struct Fixture {
    pub(crate) account: Account,
    pub(crate) catalog: Catalog,
    pub(crate) items: ItemTable,
    pub(crate) prices: FixedPrices,
    pub(crate) favourites: Favourites,
    pub(crate) listings: MarketListings,
}

#[cfg(test)]
impl Fixture {
    pub(crate) fn new(catalog: Catalog, inventory: wf_inventory::Inventory) -> Self {
        Self {
            account: Account::new(inventory),
            items: ItemTable::build(&catalog),
            catalog,
            prices: FixedPrices::default(),
            favourites: Favourites::default(),
            listings: MarketListings::default(),
        }
    }

    pub(crate) fn view(&self) -> View<'_> {
        View {
            account: &self.account,
            catalog: &self.catalog,
            items: &self.items,
            prices: &self.prices,
            favourites: &self.favourites,
            listings: &self.listings,
        }
    }

    pub(crate) fn with_prices(self, prices: FixedPrices) -> Self {
        Self { prices, ..self }
    }

    pub(crate) fn with_market(mut self, listed: &[wf_market::Item]) -> Self {
        self.items.index_market(listed);
        self
    }

    pub(crate) fn rows<R>(
        &self,
        build: fn(&View, &mut ItemIndex) -> Vec<R>,
    ) -> (Vec<R>, ItemIndex) {
        let mut index = ItemIndex::default();
        let rows = build(&self.view(), &mut index);
        (rows, index)
    }
}
