use std::collections::{HashMap, HashSet, VecDeque};

use ic_kit::prelude::*;

pub type TokenID = Nat;
pub type ApprovalID = Nat;
pub type TransferID = Nat;

pub type Subaccount = [u8; 32];

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize, CandidType)]
pub struct Account {
    pub owner: Principal,
    pub subaccount: Option<Subaccount>,
}

impl Default for Account {
    fn default() -> Self {
        Account {
            owner: Principal::from_slice(&[]),
            subaccount: Some([0u8; 32]),
        }
    }
}

impl Account {
    pub fn from_owner(owner: Principal) -> Self {
        Account {
            owner,
            subaccount: None,
        }
        .to_canonical()
    }

    /// Returns a canonicalized version of the account.
    /// If subaccount is not set, fills it with zeros (default account for principal)
    pub fn to_canonical(&self) -> Self {
        Account {
            owner: self.owner,
            subaccount: Some(self.subaccount.unwrap_or([0u8; 32])),
        }
    }
}

#[derive(Default)]

pub struct Collection {
    pub name: String,
    pub symbol: String,
    pub royalties: u16,
    pub royalty_recipient: Account,
    pub description: Option<String>,
    pub image: Option<Vec<u8>>,
    pub supply_cap: Option<usize>,
    pub authority: Option<Principal>,

    pub tokens: HashMap<TokenID, Token>,

    pub approval_id_seq: ApprovalID,
    pub approvals: HashMap<ApprovalID, Approval>,
    pub approvals_by_principal: HashMap<Principal, Vec<ApprovalID>>,

    pub transfer_id_seq: TransferID,
    pub transfers: HashMap<TransferID, Transfer>,
    // Using tuple so all transfers are trurly unique
    pub transfers_by_created_at: VecDeque<(u64, TransferID)>,
}

#[derive(Debug, Clone)]
pub struct Transfer {
    pub from: Account,
    pub to: Account,
    pub token_ids: HashSet<TokenID>,
    pub memo: Option<Vec<u8>>,
    pub created_at: u64,
}

pub struct Approval {
    pub from: Principal,
    pub from_subaccount: Option<Subaccount>,
    pub to: Principal,
    pub token_ids: Option<HashSet<TokenID>>,
    pub expires_at: Option<i64>,
    pub memo: Option<Vec<u8>>,
}

impl Collection {
    pub fn add_token(&mut self, token: Token) {
        self.tokens.insert(token.id.clone(), token);
    }

    pub fn add_approval(&mut self, approval: Approval) -> ApprovalID {
        let id = self.approval_id_seq.clone();
        self.approval_id_seq += 1;

        let from = approval.from;

        self.approvals.insert(id.clone(), approval);
        self.approvals_by_principal
            .entry(from)
            .or_default()
            .push(id.clone());

        id
    }

    pub fn add_transfer(&mut self, transfer: Transfer) -> TransferID {
        let created_at = transfer.created_at;
        let id = self.transfer_id_seq.clone();
        self.transfer_id_seq += 1;

        self.transfers.insert(id.clone(), transfer);
        self.transfers_by_created_at
            .push_back((created_at, id.clone()));

        id
    }
}

pub struct Token {
    pub id: TokenID,
    pub name: String,
    pub image: Vec<u8>,
    pub owner: Account,
}
