use std::collections::HashSet;

use ic_kit::prelude::*;

use crate::state::*;

use base64::engine::general_purpose::STANDARD_NO_PAD as b64;
use base64::Engine;

/// arguments for the "mint" method
#[derive(Debug, Deserialize, Serialize, CandidType)]
pub struct MintTokenArgs {
    /// token ID
    pub id: TokenID,
    /// token name
    pub name: String,
    /// base64 encoded token image
    pub image: String,
    /// new token owner
    pub owner: Account,
}

#[update]
pub fn mint_token(c: &mut Collection, args: MintTokenArgs) -> Result<TokenID, String> {
    if c.authority.is_none() {
        return Err("can't mint because authority is not set".to_owned());
    }

    if c.authority.as_ref().unwrap() != &caller() {
        return Err(format!(
            "caller is not authority: {} != {}",
            caller(),
            c.authority.as_ref().unwrap(),
        ));
    }

    if c.tokens.len() == c.supply_cap.unwrap_or(usize::MAX) {
        return Err("supply cap reached".to_owned());
    }

    if c.tokens.contains_key(&args.id) {
        return Err("token with this ID already exists".to_owned());
    }

    let image = match b64.decode(args.image) {
        Ok(image) => image,
        Err(e) => {
            return Err(format!("failed to decode base64 image: {}", e));
        }
    };

    let token = Token {
        id: args.id.clone(),
        name: args.name,
        image,
        owner: args.owner.to_canonical(),
    };

    c.add_token(token);

    Ok(args.id)
}

#[derive(Debug, Deserialize, Serialize, CandidType)]
pub struct ApproveArgs {
    pub from_subaccount: Option<Subaccount>,
    pub to: Principal,
    pub token_ids: Option<HashSet<TokenID>>,
    pub expires_at: Option<u64>,
    pub memo: Option<Vec<u8>>,
    pub created_at: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize, CandidType)]
pub enum AppprovalError {
    Unauthorized(Vec<Nat>),
    TooOld,
    TemporarilyUnavailable,
    GenericError { error_code: Nat, message: String },
}

pub const PERMITTED_TIME_DRIFT: u64 = 2 * 60 * 1_000_000_000; // 2 minutes in nanoseconds

#[update]
pub fn icrc7_approve(c: &mut Collection, args: ApproveArgs) -> Result<ApprovalID, AppprovalError> {
    let from = caller();
    if from == Principal::anonymous() {
        return Err(AppprovalError::GenericError {
            error_code: 3.into(),
            message: "anonymous calls are not supported".to_string(),
        });
    }

    // check if caller owns all the tokens they want to approve
    if let Some(ref ids) = args.token_ids {
        let unauthorized_ids = ids
            .iter()
            .filter(|id| c.tokens[id].owner.owner != from)
            .cloned()
            .collect::<Vec<_>>();

        if !unauthorized_ids.is_empty() {
            return Err(AppprovalError::Unauthorized(unauthorized_ids));
        }
    }

    if let Some(created_at) = args.created_at {
        let now = ic::time();
        if now > created_at + PERMITTED_TIME_DRIFT {
            return Err(AppprovalError::TooOld);
        }
    }

    let approval = Approval {
        from,
        from_subaccount: args.from_subaccount,
        to: args.to,
        token_ids: args.token_ids,
        expires_at: args.expires_at,
        memo: args.memo,
    };

    let id = c.add_approval(approval);

    Ok(id)
}

#[derive(Debug, Clone, Deserialize, Serialize, CandidType)]
pub struct TransferArgs {
    pub from: Option<Account>,
    pub to: Account,
    pub token_ids: HashSet<TokenID>,
    pub memo: Option<Vec<u8>>,
    pub created_at_time: Option<u64>,
    pub is_atomic: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, CandidType)]
pub enum TransferError {
    Unauthorized { token_ids: Vec<TokenID> },
    TooOld,
    CreatedInFuture { ledger_time: u64 },
    Duplicate { duplicate_of: Nat },
    TemporarilyUnavailable,
    GenericError { error_code: Nat, message: String },
}

#[update]
pub fn icrc7_transfer(c: &mut Collection, args: TransferArgs) -> Result<TransferID, TransferError> {
    if args.token_ids.is_empty() {
        return Err(TransferError::GenericError {
            error_code: 4.into(),
            message: "token_ids must not be empty".to_string(),
        });
    }

    if let Some(created_at) = args.created_at_time {
        let now = ic::time();
        if now > created_at + PERMITTED_TIME_DRIFT {
            return Err(TransferError::TooOld);
        }
    }

    if let Some(created_at) = args.created_at_time {
        let now = ic::time();
        if now + PERMITTED_TIME_DRIFT < created_at {
            return Err(TransferError::CreatedInFuture { ledger_time: now });
        }
    }

    let from = args
        .from
        .clone()
        .unwrap_or(Account::from_owner(caller()))
        .to_canonical();

    let transfer = Transfer {
        from: from.clone(),
        to: args.to.clone(),
        token_ids: args.token_ids.clone(),
        memo: args.memo.clone(),
        created_at: args.created_at_time.unwrap_or(ic::time()),
    };

    if let Some(id) = c.find_duplicate_transfer(&transfer) {
        return Err(TransferError::Duplicate { duplicate_of: id });
    }

    // since updates in IC are not atomic (i.e. replying with error does not revert state changes)
    // we need to make sure we don't mutate state before checking all preconditions
    let mut apply = |dry: bool| {
        let mut errs = Vec::new();

        for id in &args.token_ids {
            // dry run changes, before actually applying them
            if let Err(e) = transfer_single(c, id.clone(), &from, &args, dry) {
                errs.push(e);
            }
        }

        errs
    };

    let dry_run = args.is_atomic.unwrap_or(true);
    let errs = apply(dry_run);

    if args.is_atomic.unwrap_or(true) && !errs.is_empty() {
        let err = errs.first().cloned().unwrap();
        return Err(err);
    }

    if dry_run {
        // actually apply state changes by running update again
        let errs = apply(false);
        assert!(errs.is_empty(), "dry run should have caught all errors");
    }

    // mutate
    let id = c.add_transfer(transfer);

    Ok(id)
}

fn transfer_single(
    c: &mut Collection,
    id: TokenID,
    from: &Account,
    args: &TransferArgs,
    dry_run: bool,
) -> Result<(), TransferError> {
    if c.tokens.get(&id).is_none() {
        return Err(TransferError::GenericError {
            error_code: 1.into(),
            message: format!("token with id {} does not exist", id),
        });
    }

    if from.owner != caller() {
        // this is either approval or someone wants to transfer someone else's token
        let approval = c.find_approval_for_delegate(from, &caller(), &id);
        if approval.is_none() {
            return Err(TransferError::Unauthorized {
                token_ids: vec![id],
            });
        }
    }

    if *from == args.to {
        return Err(TransferError::GenericError {
            error_code: 2.into(),
            message: "can't transfer to self".to_string(),
        });
    }

    if !dry_run {
        let token = c.tokens.get_mut(&id).unwrap();
        token.owner = args.to.clone().to_canonical();
    }

    Ok(())
}
