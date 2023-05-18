use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

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
    pub transfers_by_created_at: BTreeSet<(u64, TransferID)>,
}

#[derive(Debug, Clone)]
pub struct Transfer {
    pub from: Account,
    pub to: Account,
    pub token_ids: HashSet<TokenID>,
    pub memo: Option<Vec<u8>>,
    pub created_at: u64,
}

#[derive(Debug, Clone)]
pub struct Approval {
    pub from: Principal,
    pub from_subaccount: Option<Subaccount>,
    pub to: Principal,
    pub token_ids: Option<HashSet<TokenID>>,
    pub expires_at: Option<u64>,
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

    /// search valid approvals that will allow `delegate` to transfer `token_id` from `from_acc`
    /// returns none if no approval match
    pub fn find_approval_for_delegate(
        &self,
        from_acc: &Account,
        delegate: &Principal,
        token_id: &TokenID,
    ) -> Option<ApprovalID> {
        let approvals = self.approvals_by_principal.get(&from_acc.owner);

        for approval_id in approvals.unwrap_or(&vec![]) {
            let approval = self.approvals.get(approval_id)?;

            if approval.to != *delegate {
                // not the right delegate
                return None;
            }

            if approval.token_ids.is_some()
                && !approval.token_ids.as_ref().unwrap().contains(token_id)
            {
                // approval is for another token(s)
                return None;
            }

            if approval.expires_at.is_some() && approval.expires_at.unwrap() < ic::time() {
                // approval has expired
                return None;
            }

            if approval.from_subaccount.is_some()
                && from_acc.subaccount.unwrap_or_default() != approval.from_subaccount.unwrap()
            {
                // approval is for another subaccount
                return None;
            }

            return Some(approval_id.clone());
        }

        None
    }

    pub fn add_transfer(&mut self, transfer: Transfer) -> TransferID {
        let created_at = transfer.created_at;
        let id = self.transfer_id_seq.clone();
        self.transfer_id_seq += 1;

        self.transfers.insert(id.clone(), transfer);
        self.transfers_by_created_at
            .insert((created_at, id.clone()));

        id
    }

    pub fn find_duplicate_transfer(&self, t: &Transfer) -> Option<TransferID> {
        // search all transactions that happened in this nanosecond
        let range = (t.created_at, Nat::from(0))..(t.created_at + 1, Nat::from(0));

        for (created_at, id) in self.transfers_by_created_at.range(range) {
            if *created_at != t.created_at {
                break;
            }

            let transfer = self.transfers.get(id).unwrap();

            // transfers are only equal if all fields are equal
            if transfer.from == t.from
                && transfer.to == t.to
                && transfer.token_ids == t.token_ids
                && transfer.memo == t.memo
            {
                return Some(id.clone());
            }
        }

        None
    }
}

pub struct Token {
    pub id: TokenID,
    pub name: String,
    pub image: Vec<u8>,
    pub owner: Account,
}
