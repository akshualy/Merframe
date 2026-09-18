use wf_market::{
    Client, CreateOrderRequest, OrderType, Platform, UpdateAuctionRequest, UpdateOrderRequest,
};

fn client() -> Client {
    Client::new(reqwest::Client::new(), Platform::Pc)
}

#[allow(
    clippy::unwrap_used,
    reason = "a test helper outside a #[test] function"
)]
fn body_json(request: &reqwest::Request) -> serde_json::Value {
    let body = request.body().unwrap();
    let bytes = body.as_bytes().unwrap();
    serde_json::from_slice(bytes).unwrap()
}

#[test]
fn items_request() {
    let client = client();
    let request = client.items_request().unwrap();
    assert_eq!(request.method(), reqwest::Method::GET);
    assert_eq!(request.url().as_str(), "https://api.yareli.net/v1/items");
    assert!(request.headers().get("Authorization").is_none());
}

#[test]
fn orders_for_item_request() {
    let client = client();
    let request = client
        .orders_for_item_request("secura_dual_cestra")
        .unwrap();
    assert_eq!(
        request.url().as_str(),
        "https://api.warframe.market/v2/orders/item/secura_dual_cestra"
    );
}

#[test]
fn create_order_request() {
    let client = client();
    let body = CreateOrderRequest {
        item_id: String::from("54aae292e7798909064f1575"),
        order_type: OrderType::Sell,
        platinum: 25,
        quantity: 1,
        visible: Some(true),
        per_trade: None,
        rank: None,
        subtype: None,
        amber_stars: None,
        cyan_stars: None,
    };
    let request = client.create_order_request(&body).unwrap();
    assert_eq!(request.method(), reqwest::Method::POST);
    assert_eq!(
        request.url().as_str(),
        "https://api.warframe.market/v2/order"
    );
    let json = body_json(&request);
    assert_eq!(json["itemId"], "54aae292e7798909064f1575");
    assert_eq!(json["type"], "sell");
    assert_eq!(json["platinum"], 25);
    assert!(json.get("subtype").is_none());
    assert!(json.get("amberStars").is_none());
    assert!(json.get("cyanStars").is_none());
}

#[test]
fn create_order_with_subtype() {
    let client = client();
    let body = CreateOrderRequest {
        item_id: "5a2feeb1c2c9e90cbdaa23ba".to_string(),
        order_type: OrderType::Sell,
        platinum: 12,
        quantity: 1,
        visible: Some(true),
        per_trade: None,
        rank: None,
        subtype: Some(String::from("radiant")),
        amber_stars: None,
        cyan_stars: None,
    };
    let request = client.create_order_request(&body).unwrap();
    let json = body_json(&request);
    assert_eq!(json["subtype"], "radiant");
    assert!(json.get("amberStars").is_none());
    assert!(json.get("cyanStars").is_none());
}

#[test]
fn create_order_with_stars() {
    let client = client();
    let body = CreateOrderRequest {
        item_id: "5a2feeb1c2c9e90cbdaa23bb".to_string(),
        order_type: OrderType::Sell,
        platinum: 40,
        quantity: 1,
        visible: Some(true),
        per_trade: None,
        rank: None,
        subtype: None,
        amber_stars: Some(2),
        cyan_stars: Some(1),
    };
    let request = client.create_order_request(&body).unwrap();
    let json = body_json(&request);
    assert_eq!(json["amberStars"], 2);
    assert_eq!(json["cyanStars"], 1);
    assert!(json.get("subtype").is_none());
}

#[test]
fn update_order_request() {
    let client = client();
    let body = UpdateOrderRequest {
        platinum: Some(30),
        quantity: None,
        visible: Some(false),
    };
    let request = client.update_order_request("abc123", &body).unwrap();
    assert_eq!(request.method(), reqwest::Method::PATCH);
    assert_eq!(
        request.url().as_str(),
        "https://api.warframe.market/v2/order/abc123"
    );
    let json = body_json(&request);
    assert_eq!(json["platinum"], 30);
    assert_eq!(json["visible"], false);
    assert!(json.get("quantity").is_none());
}

#[test]
fn delete_order_request() {
    let client = client();
    let request = client.delete_order_request("abc123").unwrap();
    assert_eq!(request.method(), reqwest::Method::DELETE);
    assert_eq!(
        request.url().as_str(),
        "https://api.warframe.market/v2/order/abc123"
    );
}

#[test]
fn close_order_request() {
    let client = client();
    let request = client.close_order_request("abc123", 2).unwrap();
    assert_eq!(request.method(), reqwest::Method::POST);
    assert_eq!(
        request.url().as_str(),
        "https://api.warframe.market/v2/order/abc123/close"
    );
    let json = body_json(&request);
    assert_eq!(json, serde_json::json!({ "quantity": 2 }));
}

#[test]
fn orders_visibility_request() {
    let client = client();
    let request = client.set_all_orders_visibility_request(true).unwrap();
    assert_eq!(request.method(), reqwest::Method::PATCH);
    assert_eq!(
        request.url().as_str(),
        "https://api.warframe.market/v2/orders/group/all"
    );
    let json = body_json(&request);
    assert_eq!(json["visible"], true);
}

#[test]
fn riven_attributes_request() {
    let client = client();
    let request = client.riven_attributes_request().unwrap();
    assert_eq!(
        request.url().as_str(),
        "https://api.warframe.market/v2/riven/attributes"
    );
}

#[test]
fn login_request() {
    let client = client();
    let request = client
        .login_request("tester@example.com", "hunter2")
        .unwrap();
    assert_eq!(request.method(), reqwest::Method::POST);
    assert_eq!(
        request.url().as_str(),
        "https://api.warframe.market/v1/auth/signin"
    );
    let json = body_json(&request);
    assert_eq!(json["email"], "tester@example.com");
    assert_eq!(json["password"], "hunter2");
    assert_eq!(json["auth_type"], "header");
    assert_eq!(request.headers().get("Authorization").unwrap(), "JWT");
    assert_eq!(request.headers().get("auth_type").unwrap(), "header");
}

#[test]
fn login_ignores_stored_token() {
    let client = client().with_token(String::from("stale.token.value"));
    let request = client
        .login_request("tester@example.com", "hunter2")
        .unwrap();
    assert_eq!(request.headers().get("Authorization").unwrap(), "JWT");
}

#[test]
fn v1_jwt_prefix() {
    let client = client().with_token("abc.def.ghi".to_string());
    let request = client.profile_v1_request("some_user").unwrap();
    assert_eq!(
        request.headers().get("Authorization").unwrap(),
        "JWT abc.def.ghi"
    );
}

#[test]
fn v2_bearer_prefix() {
    let client = client().with_token("abc.def.ghi".to_string());
    let request = client.me_request().unwrap();
    assert_eq!(
        request.headers().get("Authorization").unwrap(),
        "Bearer abc.def.ghi"
    );
}

#[test]
fn platform_header() {
    let client = Client::new(reqwest::Client::new(), Platform::Switch);
    let request = client
        .orders_for_item_request("secura_dual_cestra")
        .unwrap();
    assert_eq!(request.headers().get("Platform").unwrap(), "switch");
}

#[test]
fn my_auctions_request() {
    let client = client().with_token("abc.def.ghi".to_string());
    let request = client.auctions_my_request("merframetester").unwrap();
    assert_eq!(request.method(), reqwest::Method::GET);
    assert_eq!(
        request.url().as_str(),
        "https://api.warframe.market/v1/profile/merframetester/auctions"
    );
    assert_eq!(
        request.headers().get("Authorization").unwrap(),
        "JWT abc.def.ghi"
    );
}

#[test]
fn update_auction_request() {
    let client = client();
    let body = UpdateAuctionRequest {
        buyout_price: Some(450),
        ..UpdateAuctionRequest::default()
    };
    let request = client
        .update_auction_request("6a9f08ab51f7f20eeeb6add2", &body)
        .unwrap();
    assert_eq!(request.method(), reqwest::Method::PUT);
    assert_eq!(
        request.url().as_str(),
        "https://api.warframe.market/v1/auctions/entry/6a9f08ab51f7f20eeeb6add2"
    );
    let json = body_json(&request);
    assert_eq!(json["buyout_price"], 450);
    assert!(json.get("starting_price").is_none());
    assert!(json.get("visible").is_none());
}

#[test]
fn close_auction_request() {
    let client = client();
    let request = client
        .close_auction_request("6a9f08ab51f7f20eeeb6add2")
        .unwrap();
    assert_eq!(request.method(), reqwest::Method::PUT);
    assert_eq!(
        request.url().as_str(),
        "https://api.warframe.market/v1/auctions/entry/6a9f08ab51f7f20eeeb6add2/close"
    );
}

#[test]
fn auctions_visibility_request() {
    let client = client();
    let request = client.set_auctions_visibility_request(false).unwrap();
    assert_eq!(request.method(), reqwest::Method::PUT);
    assert_eq!(
        request.url().as_str(),
        "https://api.warframe.market/v1/profile/auctions/visibility"
    );
    assert_eq!(body_json(&request)["visible"], false);
}

#[test]
fn chat_requests() {
    let client = client();
    let list = client.chats_request().unwrap();
    assert_eq!(list.method(), reqwest::Method::GET);
    assert_eq!(
        list.url().as_str(),
        "https://api.warframe.market/v1/im/chats"
    );
}

#[test]
fn chat_request_token() {
    let client = Client::new(reqwest::Client::new(), Platform::Pc).with_token("tenno".to_string());
    let request = client.chats_request().unwrap();
    assert_eq!(request.headers().get("Authorization").unwrap(), "JWT tenno");
}

#[test]
fn encoded_path_segments() {
    let client = client();
    let order = client.delete_order_request("5f1e/../../me").unwrap();
    assert_eq!(
        order.url().as_str(),
        "https://api.warframe.market/v2/order/5f1e%2F..%2F..%2Fme"
    );
    let item = client.item_request("braton prime#set?x=1").unwrap();
    assert_eq!(
        item.url().as_str(),
        "https://api.warframe.market/v2/items/braton%20prime%23set%3Fx%3D1"
    );
    let profile = client.profile_v1_request("Tenno Krys").unwrap();
    assert_eq!(
        profile.url().as_str(),
        "https://api.warframe.market/v1/profile/Tenno%20Krys"
    );
    let plain = client
        .orders_for_item_request("braton_prime_barrel")
        .unwrap();
    assert_eq!(
        plain.url().as_str(),
        "https://api.warframe.market/v2/orders/item/braton_prime_barrel"
    );
}
