pub mod state;
use crate::state::*;

pub mod update;
pub use crate::update::*;

use std::collections::HashSet;

use base64::engine::general_purpose::STANDARD_NO_PAD as b64;
use base64::Engine;

use ic_kit::prelude::*;

#[derive(Debug, Deserialize, Serialize, CandidType)]
pub struct InitArgs {
    /// collection name
    pub name: String,
    /// collection symbol
    pub symbol: String,
    /// collection description
    pub description: Option<String>,
    /// royalties in basis points
    pub royalties: u16,
    /// royalties recipient
    pub royalty_recipient: Account,
    /// base64 encoded collection image
    pub image: Option<String>,
    /// supply cap on tokens in this collection
    pub supply_cap: Option<usize>,
    /// authority that is able to mint new tokens in this collection
    pub authority: Principal,
}

#[init]
fn init(c: &mut Collection, args: InitArgs) {
    if args.royalties > 10000 {
        panic!("royalties must be between 0 and 10000");
    }

    if args.supply_cap.is_some() && args.supply_cap.unwrap() == 0 {
        panic!("supply cap must be greater than 0");
    }

    *c = Collection {
        name: args.name,
        symbol: args.symbol.to_uppercase(),
        royalties: args.royalties,
        royalty_recipient: args.royalty_recipient,
        description: args.description,
        image: args
            .image
            .map(|s| b64.decode(s).expect("decode base64 image")),
        supply_cap: args.supply_cap,
        authority: Some(args.authority),

        ..Default::default()
    };
}

#[query]
fn icrc7_name(collection: &Collection) -> String {
    collection.name.to_owned()
}

#[query]
fn icrc7_symbol(collection: &Collection) -> String {
    collection.symbol.to_owned()
}

#[query]
fn icrc7_description(collection: &Collection) -> Option<String> {
    collection.description.to_owned()
}

#[query]
fn icrc7_image(collection: &Collection) -> Option<Vec<u8>> {
    collection.image.to_owned()
}

#[query]
fn icrc7_royalties(collection: &Collection) -> u16 {
    collection.royalties
}

#[query]
fn icrc7_royalty_recipient(collection: &Collection) -> Account {
    collection.royalty_recipient.clone()
}

#[query]
fn icrc7_supply_cap(collection: &Collection) -> Option<Nat> {
    collection.supply_cap.map(Into::into)
}

#[query]
fn icrc7_total_supply(collection: &Collection) -> Nat {
    collection.tokens.len().into()
}

#[query]
fn icrc7_metadata(collection: &Collection, id: TokenID) -> Option<TokenMetadata> {
    collection.tokens.get(&id).map(|t| TokenMetadata {
        icrc7_id: t.id.clone(),
        icrc7_name: t.name.clone(),
        icrc7_image: t.image.clone(),
    })
}

#[query]
fn icrc7_owner_of(collection: &Collection, id: TokenID) -> Option<Account> {
    collection.tokens.get(&id).map(|t| t.owner.clone())
}

#[query]
fn icrc7_balance_of(collection: &Collection, owner: Account) -> Nat {
    collection
        .tokens
        .values()
        .filter(|t| t.owner == owner.to_canonical())
        .count()
        .into()
}

#[query]
fn icrc7_tokens_of(collection: &Collection, owner: Account) -> Vec<TokenID> {
    collection
        .tokens
        .values()
        .filter(|t| t.owner == owner.to_canonical())
        .map(|t| t.id.clone())
        .collect()
}

#[derive(Debug, Deserialize, Serialize, CandidType, PartialEq)]
pub struct CollectionMetadata {
    pub icrc7_name: String,
    pub icrc7_symbol: String,
    pub icrc7_royalties: u16,
    pub icrc7_royalty_recipient: Account,
    pub icrc7_description: Option<String>,
    pub icrc7_image: Option<Vec<u8>>,
    pub icrc7_total_supply: Nat,
    pub icrc7_supply_cap: Option<Nat>,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, CandidType)]
pub struct TokenMetadata {
    pub icrc7_id: TokenID,
    pub icrc7_name: String,
    pub icrc7_image: Vec<u8>,
}

#[query]
fn icrc7_collection_metadata(c: &Collection, incl: HashSet<String>) -> CollectionMetadata {
    CollectionMetadata {
        icrc7_name: maybe_field("icrc7_name", &incl, || c.name.clone()),
        icrc7_symbol: maybe_field("icrc7_symbol", &incl, || c.symbol.clone()),
        icrc7_royalties: maybe_field("icrc7_royalties", &incl, || c.royalties),
        icrc7_royalty_recipient: maybe_field("icrc7_royalty_recipient", &incl, || {
            c.royalty_recipient.clone()
        }),
        icrc7_description: maybe_field("icrc7_description", &incl, || c.description.clone()),
        icrc7_image: maybe_field("icrc7_image", &incl, || c.image.clone()),
        icrc7_total_supply: maybe_field("icrc7_total_supply", &incl, || c.tokens.len().into()),
        icrc7_supply_cap: maybe_field("icrc7_supply_cap", &incl, || c.supply_cap.map(Into::into)),
    }
}

// lazily compute field if it is present in set
fn maybe_field<T, F>(field: &str, fields: &HashSet<String>, f: F) -> T
where
    T: Default,
    F: FnOnce() -> T,
{
    if fields.is_empty() || fields.contains(field) {
        f()
    } else {
        T::default()
    }
}

#[derive(Deserialize, Serialize, CandidType)]
pub struct Standard {
    pub name: String,
    pub url: String,
}

#[query]
fn icrc7_supported_standards() -> Vec<Standard> {
    vec![Standard {
        name: "ICRC-7".to_owned(),
        url: "https://github.com/dfinity/ICRC/ICRCs/ICRC-7".to_owned(),
    }]
}

#[derive(KitCanister)]
#[candid_path("icrc7.did")]
pub struct Icrc7Canister;
