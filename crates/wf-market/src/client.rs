use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, RETRY_AFTER};
use reqwest::{Request, StatusCode};

use crate::book::{OrderBook, order_book};
use crate::error::{MarketError, Result};
use crate::models::{
    Auction, Chat, CloseOrderRequest, CreateAuctionRequest, CreateOrderRequest, Item, Order,
    OrdersGroupUpdate, Platform, RivenAttribute, Session, SetAuctionsVisibilityRequest,
    SetGroupVisibilityRequest, SignInRequest, Transaction, UpdateAuctionRequest,
    UpdateOrderRequest, User, V1Profile,
};
use crate::parse;
use crate::ratelimit::{self, RateLimiter};

pub(crate) const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ApiFamily {
    V1,
    V2,
}

pub struct Client {
    http: reqwest::Client,
    token: Option<String>,
    platform: Platform,
    language: String,
    rate_limiter: RateLimiter,
}

impl Client {
    pub fn new(http: reqwest::Client, platform: Platform) -> Self {
        Self {
            http,
            token: None,
            platform,
            language: "en".to_owned(),
            rate_limiter: ratelimit::shared(),
        }
    }

    #[must_use]
    pub fn with_token(mut self, token: String) -> Self {
        self.token = Some(token);
        self
    }

    pub fn has_token(&self) -> bool {
        self.token.is_some()
    }

    fn request_builder(
        &self,
        method: reqwest::Method,
        family: ApiFamily,
        path: &str,
    ) -> reqwest::RequestBuilder {
        let base = match family {
            ApiFamily::V1 => "https://api.warframe.market/v1",
            ApiFamily::V2 => "https://api.warframe.market/v2",
        };
        let url = format!("{base}{path}");
        let mut builder = self
            .http
            .request(method, url)
            .timeout(REQUEST_TIMEOUT)
            .header("Platform", self.platform.as_wire_str())
            .header("Language", &self.language);
        if let Some(token) = &self.token {
            let prefix = match family {
                ApiFamily::V1 => "JWT",
                ApiFamily::V2 => "Bearer",
            };
            builder = builder.header("Authorization", format!("{prefix} {token}"));
        }
        builder
    }

    async fn send(&self, request: Request) -> Result<reqwest::Response> {
        let _permit = self.rate_limiter.acquire().await?;
        let response = self.http.execute(request).await?;
        if response.status() == StatusCode::TOO_MANY_REQUESTS {
            return Err(rate_limited(&self.rate_limiter, response.headers()).await);
        }
        Ok(response)
    }

    async fn execute<T>(
        &self,
        request: Request,
        parser: impl FnOnce(&str) -> Result<T>,
    ) -> Result<T> {
        let response = self.send(request).await?;
        let status = response.status();
        let body = accepted(status, response.text().await?)?;
        parser(&body)
    }

    pub fn items_request(&self) -> Result<Request> {
        Ok(self
            .http
            .get("https://api.yareli.net/v1/items")
            .timeout(REQUEST_TIMEOUT)
            .build()?)
    }

    pub async fn items(&self) -> Result<Vec<Item>> {
        let request = self.items_request()?;
        self.execute(request, parse::envelope::<Vec<Item>>).await
    }

    pub fn item_request(&self, slug: &str) -> Result<Request> {
        Ok(self
            .request_builder(
                reqwest::Method::GET,
                ApiFamily::V2,
                &format!("/items/{}", segment(slug)),
            )
            .build()?)
    }

    pub async fn item(&self, slug: &str) -> Result<Item> {
        let request = self.item_request(slug)?;
        self.execute(request, parse::envelope::<Item>).await
    }

    pub fn orders_for_item_request(&self, slug: &str) -> Result<Request> {
        Ok(self
            .request_builder(
                reqwest::Method::GET,
                ApiFamily::V2,
                &format!("/orders/item/{}", segment(slug)),
            )
            .build()?)
    }

    pub async fn orders_for_item(&self, slug: &str) -> Result<Vec<Order>> {
        let request = self.orders_for_item_request(slug)?;
        self.execute(request, parse::envelope::<Vec<Order>>).await
    }

    pub async fn order_book(&self, slug: &str) -> Result<OrderBook> {
        Ok(order_book(self.orders_for_item(slug).await?))
    }

    pub fn orders_my_request(&self) -> Result<Request> {
        Ok(self
            .request_builder(reqwest::Method::GET, ApiFamily::V2, "/orders/my")
            .build()?)
    }

    pub async fn orders_my(&self) -> Result<Vec<Order>> {
        let request = self.orders_my_request()?;
        self.execute(request, parse::envelope::<Vec<Order>>).await
    }

    pub fn me_request(&self) -> Result<Request> {
        Ok(self
            .request_builder(reqwest::Method::GET, ApiFamily::V2, "/me")
            .build()?)
    }

    pub async fn me(&self) -> Result<Session> {
        let request = self.me_request()?;
        let response = self.send(request).await?;
        let status = response.status();
        let rotated = response
            .headers()
            .get(reqwest::header::AUTHORIZATION)
            .and_then(jwt_token)
            .filter(|token| self.token.as_ref() != Some(token));
        let body = accepted(status, response.text().await?)?;
        Ok(Session {
            user: parse::envelope(&body)?,
            rotated_token: rotated,
        })
    }

    pub fn user_request(&self, slug: &str) -> Result<Request> {
        Ok(self
            .request_builder(
                reqwest::Method::GET,
                ApiFamily::V2,
                &format!("/user/{}", segment(slug)),
            )
            .build()?)
    }

    pub async fn user(&self, slug: &str) -> Result<User> {
        let request = self.user_request(slug)?;
        self.execute(request, parse::envelope::<User>).await
    }

    pub fn create_order_request(&self, body: &CreateOrderRequest) -> Result<Request> {
        Ok(self
            .request_builder(reqwest::Method::POST, ApiFamily::V2, "/order")
            .json(body)
            .build()?)
    }

    pub async fn create_order(&self, body: &CreateOrderRequest) -> Result<Order> {
        let request = self.create_order_request(body)?;
        self.execute(request, parse::envelope::<Order>).await
    }

    pub fn update_order_request(&self, id: &str, body: &UpdateOrderRequest) -> Result<Request> {
        Ok(self
            .request_builder(
                reqwest::Method::PATCH,
                ApiFamily::V2,
                &format!("/order/{}", segment(id)),
            )
            .json(body)
            .build()?)
    }

    pub async fn update_order(&self, id: &str, body: &UpdateOrderRequest) -> Result<Order> {
        let request = self.update_order_request(id, body)?;
        self.execute(request, parse::envelope::<Order>).await
    }

    pub fn delete_order_request(&self, id: &str) -> Result<Request> {
        Ok(self
            .request_builder(
                reqwest::Method::DELETE,
                ApiFamily::V2,
                &format!("/order/{}", segment(id)),
            )
            .build()?)
    }

    pub async fn delete_order(&self, id: &str) -> Result<Order> {
        let request = self.delete_order_request(id)?;
        self.execute(request, parse::envelope::<Order>).await
    }

    pub fn close_order_request(&self, id: &str, quantity: u32) -> Result<Request> {
        Ok(self
            .request_builder(
                reqwest::Method::POST,
                ApiFamily::V2,
                &format!("/order/{}/close", segment(id)),
            )
            .json(&CloseOrderRequest { quantity })
            .build()?)
    }

    pub async fn close_order(&self, id: &str, quantity: u32) -> Result<Transaction> {
        let request = self.close_order_request(id, quantity)?;
        self.execute(request, parse::envelope::<Transaction>).await
    }

    pub fn set_all_orders_visibility_request(&self, visible: bool) -> Result<Request> {
        Ok(self
            .request_builder(reqwest::Method::PATCH, ApiFamily::V2, "/orders/group/all")
            .json(&SetGroupVisibilityRequest { visible })
            .build()?)
    }

    pub async fn set_all_orders_visibility(&self, visible: bool) -> Result<OrdersGroupUpdate> {
        let request = self.set_all_orders_visibility_request(visible)?;
        self.execute(request, parse::envelope::<OrdersGroupUpdate>)
            .await
    }

    pub fn riven_attributes_request(&self) -> Result<Request> {
        Ok(self
            .request_builder(reqwest::Method::GET, ApiFamily::V2, "/riven/attributes")
            .build()?)
    }

    pub async fn riven_attributes(&self) -> Result<Vec<RivenAttribute>> {
        let request = self.riven_attributes_request()?;
        self.execute(request, parse::envelope::<Vec<RivenAttribute>>)
            .await
    }

    pub fn login_request(&self, email: &str, password: &str) -> Result<Request> {
        let body = SignInRequest::new(email.to_string(), password.to_string());
        let mut request = self
            .request_builder(reqwest::Method::POST, ApiFamily::V1, "/auth/signin")
            .header("auth_type", "header")
            .json(&body)
            .build()?;
        request.headers_mut().insert(
            reqwest::header::AUTHORIZATION,
            HeaderValue::from_static("JWT"),
        );
        Ok(request)
    }

    pub async fn login(&self, email: &str, password: &str) -> Result<String> {
        let request = self.login_request(email, password)?;
        let response = self.send(request).await?;
        let status = response.status();
        let authorization_header = response
            .headers()
            .get(reqwest::header::AUTHORIZATION)
            .cloned();
        accepted(status, response.text().await?)?;
        authorization_header
            .as_ref()
            .and_then(jwt_token)
            .ok_or(MarketError::MissingJwt)
    }

    pub fn profile_v1_request(&self, username: &str) -> Result<Request> {
        Ok(self
            .request_builder(
                reqwest::Method::GET,
                ApiFamily::V1,
                &format!("/profile/{}", segment(username)),
            )
            .build()?)
    }

    pub async fn profile_v1(&self, username: &str) -> Result<V1Profile> {
        let request = self.profile_v1_request(username)?;
        self.execute(request, parse::v1_payload::<V1Profile>).await
    }

    pub fn chats_request(&self) -> Result<Request> {
        Ok(self
            .request_builder(reqwest::Method::GET, ApiFamily::V1, "/im/chats")
            .build()?)
    }

    pub async fn chats(&self) -> Result<Vec<Chat>> {
        let request = self.chats_request()?;
        self.execute(request, parse::parse_chats).await
    }

    pub fn auctions_my_request(&self, slug: &str) -> Result<Request> {
        Ok(self
            .request_builder(
                reqwest::Method::GET,
                ApiFamily::V1,
                &format!("/profile/{}/auctions", segment(slug)),
            )
            .build()?)
    }

    pub async fn auctions_my(&self, slug: &str) -> Result<Vec<Auction>> {
        let request = self.auctions_my_request(slug)?;
        self.execute(request, parse::parse_v1_auctions).await
    }

    pub fn create_auction_request(&self, body: &CreateAuctionRequest) -> Result<Request> {
        Ok(self
            .request_builder(reqwest::Method::POST, ApiFamily::V1, "/auctions/create")
            .json(body)
            .build()?)
    }

    pub async fn create_auction(&self, body: &CreateAuctionRequest) -> Result<Auction> {
        let request = self.create_auction_request(body)?;
        self.execute(request, parse::parse_v1_auction).await
    }

    pub fn update_auction_request(&self, id: &str, body: &UpdateAuctionRequest) -> Result<Request> {
        Ok(self
            .request_builder(
                reqwest::Method::PUT,
                ApiFamily::V1,
                &format!("/auctions/entry/{}", segment(id)),
            )
            .json(body)
            .build()?)
    }

    pub async fn update_auction(&self, id: &str, body: &UpdateAuctionRequest) -> Result<Auction> {
        let request = self.update_auction_request(id, body)?;
        self.execute(request, parse::parse_v1_auction).await
    }

    pub fn close_auction_request(&self, id: &str) -> Result<Request> {
        Ok(self
            .request_builder(
                reqwest::Method::PUT,
                ApiFamily::V1,
                &format!("/auctions/entry/{}/close", segment(id)),
            )
            .build()?)
    }

    pub async fn close_auction(&self, id: &str) -> Result<Auction> {
        let request = self.close_auction_request(id)?;
        self.execute(request, parse::parse_v1_auction).await
    }

    pub fn set_auctions_visibility_request(&self, visible: bool) -> Result<Request> {
        Ok(self
            .request_builder(
                reqwest::Method::PUT,
                ApiFamily::V1,
                "/profile/auctions/visibility",
            )
            .json(&SetAuctionsVisibilityRequest { visible })
            .build()?)
    }

    pub async fn set_auctions_visibility(&self, visible: bool) -> Result<()> {
        let request = self.set_auctions_visibility_request(visible)?;
        let response = self.send(request).await?;
        let status = response.status();
        accepted(status, response.text().await?)?;
        Ok(())
    }
}

pub(crate) fn segment(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(char::from(byte));
            }
            _ => {
                encoded.push('%');
                encoded.push(char::from(HEX[usize::from(byte >> 4)]));
                encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
            }
        }
    }
    encoded
}

fn jwt_token(header: &HeaderValue) -> Option<String> {
    header
        .to_str()
        .ok()?
        .strip_prefix("JWT ")
        .map(str::to_owned)
}

pub(crate) async fn rate_limited(limiter: &RateLimiter, headers: &HeaderMap) -> MarketError {
    let pause = match headers
        .get(RETRY_AFTER)
        .and_then(|value| value.to_str().ok()?.parse().ok())
    {
        Some(secs) => Duration::from_secs(secs).min(Duration::from_secs(300)),
        None => Duration::from_secs(30),
    };
    limiter.back_off(pause).await;
    MarketError::RateLimited(pause)
}

pub(crate) fn accepted(status: StatusCode, body: String) -> Result<String> {
    if status == StatusCode::UNAUTHORIZED {
        return Err(MarketError::Unauthorized);
    }
    if status.is_success() {
        Ok(body)
    } else {
        Err(MarketError::Http(status, body))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(retry_after: &'static str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(RETRY_AFTER, HeaderValue::from_static(retry_after));
        headers
    }

    async fn pause(headers: &HeaderMap) -> Duration {
        let limiter = RateLimiter::new(3, Duration::from_secs(1));
        match rate_limited(&limiter, headers).await {
            MarketError::RateLimited(pause) => pause,
            error => panic!("{error}"),
        }
    }

    #[tokio::test]
    async fn retry_after_seconds() {
        assert_eq!(pause(&headers("12")).await, Duration::from_secs(12));
    }

    #[tokio::test]
    async fn retry_after_capped() {
        assert_eq!(pause(&headers("86400")).await, Duration::from_secs(300));
    }

    #[tokio::test]
    async fn retry_after_dated_or_missing() {
        let dated = headers("Wed, 21 Oct 2026 07:28:00 GMT");
        assert_eq!(pause(&dated).await, Duration::from_secs(30));
        assert_eq!(pause(&HeaderMap::new()).await, Duration::from_secs(30));
    }
}
