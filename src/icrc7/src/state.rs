use std::collections::{BTreeMap, HashMap, HashSet};

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
    pub fn new(owner: Principal, subaccount: Option<Subaccount>) -> Self {
        Account { owner, subaccount }
    }

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

/// Internal state of the canister
#[derive(Default, Debug, Clone, Deserialize, Serialize, CandidType)]

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

    // transfers are stored in a BTreeMap to allow for efficient purging of old transfers
    // key is (transfer_timestamp, transfer_id), so we can have multiple transfers at the same nanosecond
    // this is inspried by Redis streams ids
    pub transfers: BTreeMap<(u64, TransferID), Transfer>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, CandidType)]
pub struct Transfer {
    pub from: Account,
    pub to: Account,
    pub token_ids: HashSet<TokenID>,
    pub memo: Option<Vec<u8>>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, CandidType)]
pub struct Approval {
    pub from: Principal,
    pub from_subaccount: Option<Subaccount>,
    pub to: Principal,
    pub token_ids: Option<HashSet<TokenID>>,
    pub expires_at: Option<u64>,
    pub memo: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, CandidType)]
pub struct Token {
    pub id: TokenID,
    pub name: String,
    pub image: Vec<u8>,
    pub owner: Account,
}

// 24h in nanoseconds
const TX_DEDUPLICATION_WINDOW: u64 = 24 * 60 * 60 * 1_000_000_000;

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
        let approvals = self.approvals_by_principal.get(&from_acc.owner)?;

        for approval_id in approvals {
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

        self.transfers.insert((created_at, id.clone()), transfer);

        id
    }

    pub fn find_duplicate_transfer(&self, t: &Transfer) -> Option<TransferID> {
        // search all transactions that happened in this nanosecond
        let range = (t.created_at, Nat::from(0))..(t.created_at + 1, Nat::from(0));

        for ((created_at, id), transfer) in self.transfers.range(range) {
            if *created_at != t.created_at {
                break;
            }

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

    // purge old transactions and approvals
    pub fn gc(&mut self, now: u64) {
        // purge transactions older than TX_DEDUPLICATION_WINDOW
        let split_key = &(now - TX_DEDUPLICATION_WINDOW, Nat::from(0));
        // we want to keep everything after split_key
        let after = self.transfers.split_off(&split_key);
        self.transfers = after;

        // purge expired approvals
        self.approvals.retain(|_k, a| {
            if a.expires_at.is_some() && a.expires_at.unwrap() < now {
                return false;
            }
            true
        });
        self.approvals.shrink_to_fit();

        self.approvals_by_principal.retain(|_k, v| {
            v.retain(|id| self.approvals.get(id).is_some());
            !v.is_empty()
        });
        self.approvals_by_principal.shrink_to_fit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gc() {
        let mut c = Collection::default();

        let now = 10000000000000000000;

        let t1 = Transfer {
            created_at: now - TX_DEDUPLICATION_WINDOW - 1,
            ..Default::default()
        };

        let t2 = Transfer {
            created_at: now - TX_DEDUPLICATION_WINDOW + 1,
            ..Default::default()
        };

        let t3 = Transfer {
            created_at: now + TX_DEDUPLICATION_WINDOW + 1,
            ..Default::default()
        };

        c.add_transfer(t1.clone());
        c.add_transfer(t2.clone());
        c.add_transfer(t3.clone());

        assert_eq!(c.transfers.len(), 3);

        c.gc(now);

        assert_eq!(c.transfers.len(), 2);
        assert!(c.transfers.contains_key(&(t2.created_at, 1.into())));
        assert!(c.transfers.contains_key(&(t3.created_at, 2.into())));
    }

    #[test]
    fn test_gc_approvals() {
        let mut c = Collection::default();

        let now = 10000000000000000000;

        let a1 = Approval {
            expires_at: Some(now - 1),
            from: Principal::anonymous(),
            from_subaccount: None,
            to: Principal::anonymous(),
            token_ids: None,
            memo: None,
        };

        let a2 = Approval {
            expires_at: Some(now + 1),
            from: Principal::anonymous(),
            from_subaccount: None,
            to: Principal::anonymous(),
            token_ids: None,
            memo: None,
        };

        let a3 = Approval {
            expires_at: Some(now + 2),
            from: Principal::anonymous(),
            from_subaccount: None,
            to: Principal::anonymous(),
            token_ids: None,
            memo: None,
        };

        c.add_approval(a1.clone());
        c.add_approval(a2.clone());
        c.add_approval(a3.clone());

        assert_eq!(c.approvals.len(), 3);
        assert_eq!(c.approvals_by_principal[&Principal::anonymous()].len(), 3);

        c.gc(now);

        assert_eq!(c.approvals.len(), 2);
        assert_eq!(c.approvals_by_principal[&Principal::anonymous()].len(), 2);
        assert!(c.approvals.contains_key(&1.into()));
        assert!(c.approvals.contains_key(&2.into()));
    }
}
