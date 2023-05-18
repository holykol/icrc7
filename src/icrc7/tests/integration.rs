extern crate icrc7;

use core::panic;
use std::collections::HashSet;
use std::time::Duration;

use ic_kit::prelude::*;
use ic_kit_runtime::handle::CanisterHandle;
use icrc7::state::*;
use icrc7::*;
use rt::types::{EntryMode, Env};

#[kit_test]
async fn test_basic(replica: Replica) {
    let c = prepare_initialized_canister(&replica).await;

    let reply: String = c
        .new_call("icrc7_name")
        .perform()
        .await
        .decode_one()
        .unwrap();

    assert_eq!(reply, "test collection");
}

#[kit_test]
async fn test_metadata(replica: Replica) {
    let c = prepare_initialized_canister(&replica).await;

    let m: CollectionMetadata = c
        .new_call("icrc7_collection_metadata")
        .with_arg(Vec::<String>::new())
        .perform()
        .await
        .decode_one()
        .unwrap();

    assert_eq!(
        m,
        CollectionMetadata {
            icrc7_name: "test collection".to_owned(),
            icrc7_symbol: "TEST".to_owned(),
            icrc7_royalties: 1000,
            icrc7_royalty_recipient: Account::default(),
            icrc7_description: Some("blah".to_owned()),
            icrc7_image: None,
            icrc7_total_supply: 0.into(),
            icrc7_supply_cap: None,
        }
    );
}

#[kit_test]
async fn test_metadata_only_fields(replica: Replica) {
    let c = prepare_initialized_canister(&replica).await;

    let m: CollectionMetadata = c
        .new_call("icrc7_collection_metadata")
        .with_arg(vec!["icrc7_name".to_owned(), "icrc7_symbol".to_owned()])
        .perform()
        .await
        .decode_one()
        .unwrap();

    assert_eq!(
        m,
        CollectionMetadata {
            icrc7_name: "test collection".to_owned(),
            icrc7_symbol: "TEST".to_owned(),
            icrc7_royalties: 0,
            icrc7_royalty_recipient: Account::default(),
            icrc7_description: None,
            icrc7_image: None,
            icrc7_total_supply: 0.into(),
            icrc7_supply_cap: None,
        }
    );
}

#[kit_test]
async fn test_metadata_single_methods(replica: Replica) {
    let c = prepare_initialized_canister(&replica).await;

    let name: String = c
        .new_call("icrc7_name")
        .perform()
        .await
        .decode_one()
        .unwrap();

    let symbol: String = c
        .new_call("icrc7_symbol")
        .perform()
        .await
        .decode_one()
        .unwrap();

    let royalties: u16 = c
        .new_call("icrc7_royalties")
        .perform()
        .await
        .decode_one()
        .unwrap();

    let royalty_recipient: Account = c
        .new_call("icrc7_royalty_recipient")
        .perform()
        .await
        .decode_one()
        .unwrap();

    let description: Option<String> = c
        .new_call("icrc7_description")
        .perform()
        .await
        .decode_one()
        .unwrap();

    let image: Option<Vec<u8>> = c
        .new_call("icrc7_image")
        .perform()
        .await
        .decode_one()
        .unwrap();

    let total_supply: Nat = c
        .new_call("icrc7_total_supply")
        .perform()
        .await
        .decode_one()
        .unwrap();

    let supply_cap: Option<Nat> = c
        .new_call("icrc7_supply_cap")
        .perform()
        .await
        .decode_one()
        .unwrap();

    assert_eq!(name, "test collection".to_owned());
    assert_eq!(symbol, "TEST".to_owned());
    assert_eq!(royalties, 1000);
    assert_eq!(royalty_recipient, Account::default());
    assert_eq!(description, Some("blah".to_owned()));
    assert_eq!(image, None);
    assert_eq!(total_supply, 0);
    assert_eq!(supply_cap, None);
}

#[kit_test]
async fn test_add_tokens(replica: Replica) {
    let c = prepare_initialized_canister(&replica).await;

    add_token(&c, 1.into(), "NFT-1", &Account::default()).await;

    // icrc7_metadata
    let reply = c
        .new_call("icrc7_metadata")
        .with_arg(Nat::from(1))
        .perform()
        .await;

    println!("{:?}", reply);

    let m: Option<TokenMetadata> = reply.decode_one().unwrap();

    assert_eq!(
        m.expect("token should be present"),
        TokenMetadata {
            icrc7_id: 1.into(),
            icrc7_name: "NFT-1".to_owned(),
            icrc7_image: vec![65, 65, 65, 65],
        }
    );

    // check supply has changed
    let total_supply: Nat = c
        .new_call("icrc7_total_supply")
        .perform()
        .await
        .decode_one()
        .unwrap();

    assert_eq!(total_supply, 1);

    // check you can not add another token with the same id
    let resp: Result<TokenID, String> = c
        .new_call("mint_token")
        .with_arg(MintTokenArgs {
            id: 1.into(),
            name: "NFT-2".to_owned(),
            image: "QUFBQQ".to_owned(),
            owner: Account::default(),
        })
        .perform()
        .await
        .decode_one()
        .unwrap();

    assert!(resp.is_err());
}

#[kit_test]
async fn test_tokens_queries(replica: Replica) {
    let c = prepare_initialized_canister(&replica).await;

    let owner = Account {
        owner: Principal::from_slice(&[1; 4]),
        subaccount: None,
    };

    add_token(&c, 1.into(), "NFT-1", &owner).await;
    add_token(&c, 2.into(), "NFT-1", &owner).await;

    // icrc7_owner_of
    let reply: Option<Account> = c
        .new_call("icrc7_owner_of")
        .with_arg(Nat::from(1))
        .perform()
        .await
        .decode_one()
        .unwrap();

    assert_eq!(reply, Some(owner.to_canonical()));

    // icrc7_balance_of
    let reply: Nat = c
        .new_call("icrc7_balance_of")
        .with_arg(owner.clone())
        .perform()
        .await
        .decode_one()
        .unwrap();

    assert_eq!(reply, Nat::from(2));

    // icrc7_tokens_of
    let reply: HashSet<TokenID> = c
        .new_call("icrc7_tokens_of")
        .with_arg(owner.clone())
        .perform()
        .await
        .decode_one()
        .unwrap();

    // order is not specified
    assert_eq!(reply.len(), 2);
    assert!(reply.contains(&1.into()));
    assert!(reply.contains(&2.into()));
}

#[kit_test]
async fn test_simple_transfer(replica: Replica) {
    let c = prepare_initialized_canister(&replica).await;

    let owner_acc = Account::default();
    let to_acc = Account {
        owner: Principal::from_slice(&[0x1]),
        ..Default::default()
    };

    add_token(&c, 1.into(), "NFT-1", &owner_acc).await;

    let args = TransferArgs {
        from: None,
        to: to_acc,
        token_ids: HashSet::from([1.into()]),
        memo: None,
        created_at_time: None,
        is_atomic: None,
    };

    // check you can transfer tokens
    let resp = perform_transfer(&c, args, owner_acc.owner).await;

    assert_eq!(resp, Ok(0.into())); // first transfer gets 0
}

const NOW: u64 = 3600000000000; // 1 hour in nanoseconds
const MINUTE: u64 = Duration::from_secs(60).as_nanos() as u64;

#[kit_test]
async fn test_old_transfers(replica: Replica) {
    let c = prepare_initialized_canister(&replica).await;

    let owner_acc = Account::default();
    let to_acc = Account {
        owner: Principal::from_slice(&[0x1]),
        ..Default::default()
    };

    add_token(&c, 1.into(), "NFT-1", &owner_acc).await;

    // tx in past
    let call_time = NOW - MINUTE * 3;

    let mut args = TransferArgs {
        from: None,
        to: to_acc,
        token_ids: HashSet::from([1.into()]),
        memo: None,
        created_at_time: Some(call_time),
        is_atomic: None,
    };

    let reply = perform_transfer(&c, args.clone(), owner_acc.owner).await;
    assert_eq!(reply.unwrap_err(), TransferError::TooOld);

    // now we travel into the future
    args.created_at_time = Some(NOW + MINUTE * 3);
    let reply = perform_transfer(&c, args.clone(), owner_acc.owner).await;
    assert_eq!(
        reply.unwrap_err(),
        TransferError::CreatedInFuture { ledger_time: NOW }
    );
}

#[kit_test]
async fn test_atomic_transfers(replica: Replica) {
    let c = prepare_initialized_canister(&replica).await;

    let owner_acc = Account::default().to_canonical();
    let to_acc = Account::from_owner(Principal::from_slice(&[0x1]));

    add_token(&c, 1.into(), "NFT-1", &owner_acc).await;

    let mut args = TransferArgs {
        from: None,
        to: to_acc.clone(),
        // 2 does not exist, so transfer should fail in atomic mode
        token_ids: HashSet::from([1.into(), 2.into()]),
        memo: None,
        created_at_time: None,
        is_atomic: None,
    };

    // check owner did NOT accidentally change (as returned error in update still persists state changes)
    let owner_of_one = c
        .new_call("icrc7_owner_of")
        .with_arg(Nat::from(1))
        .perform()
        .await
        .decode_one::<Option<Account>>()
        .unwrap()
        .unwrap();
    assert_eq!(owner_of_one, owner_acc);

    let reply = perform_transfer(&c, args.clone(), owner_acc.owner).await;
    assert!(matches!(
        reply.unwrap_err(),
        TransferError::GenericError { .. }
    ));

    args.is_atomic = Some(false);
    perform_transfer(&c, args.clone(), owner_acc.owner)
        .await
        .expect("transfer should succeed");

    // check owner changed
    let owner_of_one = c
        .new_call("icrc7_owner_of")
        .with_arg(Nat::from(1))
        .perform()
        .await
        .decode_one::<Option<Account>>()
        .unwrap()
        .unwrap();

    assert_eq!(owner_of_one, to_acc);
}

#[kit_test]
async fn test_transfer_invalid_owner(replica: Replica) {
    let c = prepare_initialized_canister(&replica).await;

    let owner_acc = Account::default();
    let to_acc = Account::from_owner(Principal::from_slice(&[0x22]));
    let other_acc = Account::from_owner(Principal::from_slice(&[0x99]));

    add_token(&c, 1.into(), "NFT-1", &owner_acc).await;

    let args = TransferArgs {
        from: Some(owner_acc),
        to: to_acc.clone(),
        token_ids: HashSet::from([1.into()]),
        memo: None,
        created_at_time: None,
        is_atomic: None,
    };

    // unathorized
    let reply = perform_transfer(&c, args.clone(), other_acc.owner).await;

    match reply {
        Err(TransferError::Unauthorized { token_ids: ids }) => {
            let expected: Vec<Nat> = vec![1.into()];
            assert_eq!(ids, expected);
        }
        _ => panic!("unexpected response: {:?}", reply),
    }
}

#[kit_test]
async fn test_transfer_memo_deduplication(replica: Replica) {
    let c = prepare_initialized_canister(&replica).await;

    let owner_acc = Account::default();
    let to_acc = Account::from_owner(Principal::from_slice(&[0x22]));

    add_token(&c, 1.into(), "NFT-1", &owner_acc).await;

    let memo1 = "memo1".as_bytes().to_owned();

    let args = TransferArgs {
        from: None,
        to: to_acc.clone(),
        token_ids: HashSet::from([1.into()]),
        memo: Some(memo1),
        created_at_time: Some(NOW),
        is_atomic: None,
    };

    // unathorized
    let reply = perform_transfer(&c, args.clone(), owner_acc.owner).await;
    let transfer_id = reply.expect("first transfer should succeed");

    // same memo transfer, should be deduplicated
    let reply = perform_transfer(&c, args.clone(), owner_acc.owner).await;
    let err = reply.expect_err("second transfer should fail");
    assert!(
        matches!(err, TransferError::Duplicate { duplicate_of } if duplicate_of == transfer_id)
    );
}

#[kit_test]
async fn test_approvals(replica: Replica) {
    let c = prepare_initialized_canister(&replica).await;

    let owner_acc = Account::from_owner(Principal::from_slice(&[0x1]));
    let delegate_acc = Account::from_owner(Principal::from_slice(&[0x2, 0x2]));
    let to_acc = Account::from_owner(Principal::from_slice(&[0x3, 0x3, 0x3]));

    add_token(&c, 1.into(), "NFT-1", &owner_acc).await;

    let args = TransferArgs {
        from: Some(owner_acc.clone()),
        to: to_acc.clone(),
        token_ids: HashSet::from([1.into()]),
        memo: None,
        created_at_time: None,
        is_atomic: None,
    };

    perform_transfer(&c, args.clone(), delegate_acc.owner)
        .await
        .expect_err("transfer should fail");

    let approve_args = ApproveArgs {
        from_subaccount: None,
        to: delegate_acc.owner.clone(),
        token_ids: None,
        memo: None,
        created_at: None,
        expires_at: None,
    };

    let _approval_id = c
        .new_call("icrc7_approve")
        .with_arg(approve_args)
        .with_caller(owner_acc.owner)
        .perform()
        .await
        .decode_one::<Result<ApprovalID, AppprovalError>>()
        .unwrap()
        .unwrap();

    perform_transfer(&c, args.clone(), delegate_acc.owner)
        .await
        .expect("transfer should succeed");
}

#[kit_test]
async fn test_approvals_for_certain_token(replica: Replica) {
    let c = prepare_initialized_canister(&replica).await;

    let owner_acc = Account::from_owner(Principal::from_slice(&[0x1]));
    let delegate_acc = Account::from_owner(Principal::from_slice(&[0x2, 0x2]));
    let to_acc = Account::from_owner(Principal::from_slice(&[0x3, 0x3, 0x3]));

    add_token(&c, 1.into(), "NFT-1", &owner_acc).await;
    add_token(&c, 2.into(), "NFT-2", &owner_acc).await;

    // using not atomic transfer to quickly check that only one token is transferred
    let args = TransferArgs {
        from: Some(owner_acc.clone()),
        to: to_acc.clone(),
        token_ids: HashSet::from([1.into(), 2.into()]),
        memo: None,
        created_at_time: None,
        is_atomic: Some(false),
    };

    let approve_args = ApproveArgs {
        from_subaccount: None,
        to: delegate_acc.owner.clone(),
        token_ids: Some(HashSet::from([1.into()])),
        memo: None,
        created_at: None,
        expires_at: None,
    };

    perform_approve(&c, approve_args, owner_acc.owner)
        .await
        .expect("approve should succeed");

    perform_transfer(&c, args.clone(), delegate_acc.owner)
        .await
        .expect("transfer should partially succeed");

    let owner_tokens: HashSet<TokenID> = c
        .new_call("icrc7_tokens_of")
        .with_arg(owner_acc.clone())
        .perform()
        .await
        .decode_one()
        .unwrap();
    assert_eq!(owner_tokens, HashSet::from([2.into()]));

    let to_tokens: HashSet<TokenID> = c
        .new_call("icrc7_tokens_of")
        .with_arg(to_acc.clone())
        .perform()
        .await
        .decode_one()
        .unwrap();
    assert_eq!(to_tokens, HashSet::from([1.into()]));
}

#[kit_test]
async fn test_approvals_for_different_subaccounts(replica: Replica) {
    let c = prepare_initialized_canister(&replica).await;

    let owner_acc_1 = Account::from_owner(Principal::from_slice(&[0x1]));
    let owner_acc_2 = Account {
        owner: Principal::from_slice(&[0x1]),
        subaccount: Some([1; 32]),
    };

    let delegate_acc = Account::from_owner(Principal::from_slice(&[0x2, 0x2]));
    let to_acc = Account::from_owner(Principal::from_slice(&[0x3, 0x3, 0x3]));

    add_token(&c, 1.into(), "NFT-1", &owner_acc_1).await;
    add_token(&c, 2.into(), "NFT-2", &owner_acc_2).await;

    let approve_args = ApproveArgs {
        from_subaccount: Some([1; 32]), // only allow to transfer from second subaccount
        to: delegate_acc.owner.clone(),
        token_ids: None,
        memo: None,
        created_at: None,
        expires_at: None,
    };

    perform_approve(&c, approve_args, owner_acc_1.owner)
        .await
        .expect("approve should succeed");

    // using not atomic transfer to quickly check that only one token is transferred
    let mut args = TransferArgs {
        from: Some(owner_acc_1.clone()),
        to: to_acc.clone(),
        token_ids: HashSet::from([1.into()]),
        memo: None,
        created_at_time: None,
        is_atomic: None,
    };

    perform_transfer(&c, args.clone(), delegate_acc.owner)
        .await
        .expect_err("transfer should fail");

    args.from = Some(owner_acc_2.clone());
    args.token_ids = HashSet::from([2.into()]);

    perform_transfer(&c, args.clone(), delegate_acc.owner)
        .await
        .expect("transfer should succeed");
}

#[kit_test]
async fn test_expired_approvals(replica: Replica) {
    let c = prepare_initialized_canister(&replica).await;

    let owner_acc = Account::from_owner(Principal::from_slice(&[0x1]));
    let delegate_acc = Account::from_owner(Principal::from_slice(&[0x2, 0x2]));
    let to_acc = Account::from_owner(Principal::from_slice(&[0x3, 0x3, 0x3]));

    add_token(&c, 1.into(), "NFT-1", &owner_acc).await;

    let approve_args = ApproveArgs {
        from_subaccount: None,
        to: delegate_acc.owner.clone(),
        token_ids: None,
        memo: None,
        created_at: None,
        expires_at: Some(NOW - 10), // should we allow creating approvals that already expired?
    };

    perform_approve(&c, approve_args, owner_acc.owner)
        .await
        .expect("approve should succeed");

    // using not atomic transfer to quickly check that only one token is transferred
    let args = TransferArgs {
        from: Some(owner_acc.clone()),
        to: to_acc.clone(),
        token_ids: HashSet::from([1.into()]),
        memo: None,
        created_at_time: None,
        is_atomic: None,
    };

    perform_transfer(&c, args.clone(), delegate_acc.owner)
        .await
        .unwrap_err();
}

#[kit_test]
async fn test_transfer_from_non_existing_account(replica: Replica) {
    let c = prepare_initialized_canister(&replica).await;

    let owner_acc = Account::default();
    let to_acc = Account::from_owner(Principal::from_slice(&[0x22]));

    let args = TransferArgs {
        from: None,
        to: to_acc.clone(),
        token_ids: HashSet::from([1.into()]),
        memo: None,
        created_at_time: None,
        is_atomic: None,
    };

    // cannot transfer from non-existing account
    let reply = perform_transfer(&c, args.clone(), owner_acc.owner).await;
    assert!(matches!(
        reply.unwrap_err(),
        TransferError::GenericError { .. }
    ));
}

#[kit_test]
async fn test_transfer_to_self(replica: Replica) {
    let c = prepare_initialized_canister(&replica).await;

    let owner_acc = Account::default();

    add_token(&c, 1.into(), "NFT-1", &owner_acc).await;

    let args = TransferArgs {
        from: None,
        to: owner_acc.clone(),
        token_ids: HashSet::from([1.into()]),
        memo: None,
        created_at_time: None,
        is_atomic: None,
    };

    // cannot transfer to self
    let reply = perform_transfer(&c, args.clone(), owner_acc.owner).await;
    assert!(matches!(
        reply.unwrap_err(),
        TransferError::GenericError { .. }
    ));
}

/// helper to call transfer on the canister with predefined time
async fn perform_transfer(
    c: &CanisterHandle<'_>,
    args: TransferArgs,
    caller: Principal,
) -> Result<TransferID, TransferError> {
    let env = Env::default()
        .with_entry_mode(EntryMode::Update)
        .with_method_name("icrc7_transfer")
        .with_arg(args)
        .with_time(NOW)
        .with_sender(caller);

    c.run_env(env)
        .await
        .decode_one()
        .expect("call should succeed")
}

async fn perform_approve(
    c: &CanisterHandle<'_>,
    args: ApproveArgs,
    caller: Principal,
) -> Result<ApprovalID, AppprovalError> {
    c.new_call("icrc7_approve")
        .with_arg(args)
        .with_caller(caller)
        .perform()
        .await
        .decode_one()
        .unwrap()
}

#[kit_test]
async fn test_non_existent_tokens(replica: Replica) {
    let c = prepare_initialized_canister(&replica).await;

    assert!(c
        .new_call("icrc7_metadata")
        .with_arg(Nat::from(1))
        .perform()
        .await
        .decode_one::<Option<TokenMetadata>>()
        .unwrap()
        .is_none());
}

#[kit_test]
async fn test_supported_standards(replica: Replica) {
    let c = prepare_initialized_canister(&replica).await;

    let standards = c
        .new_call("icrc7_supported_standards")
        .perform()
        .await
        .decode_one::<Vec<Standard>>()
        .unwrap();

    assert_eq!(standards.len(), 1);
}

async fn prepare_initialized_canister(replica: &Replica) -> CanisterHandle {
    let r = replica.add_canister(Icrc7Canister::anonymous());

    let args = InitArgs {
        name: "test collection".to_owned(),
        symbol: "TEST".to_owned(),
        description: Some("blah".to_owned()),
        royalties: 1000,
        royalty_recipient: Account::default(),
        image: None,
        supply_cap: None,
        authority: Principal::anonymous(),
    };

    let env = ic_kit_runtime::types::Env::init().with_arg(args);
    assert_eq!(
        r.run_env(env).await.rejection_message(),
        Some("Canister did not reply to the call"),
        "Expected canister to reply nothing on init, but got a rejection"
    );

    r
}

async fn add_token(c: &CanisterHandle<'_>, id: TokenID, name: &str, owner: &Account) {
    let resp: Result<TokenID, String> = c
        .new_call("mint_token")
        .with_arg(MintTokenArgs {
            id: id.clone(),
            name: name.to_owned(),
            image: "QUFBQQ".to_owned(),
            owner: owner.clone(),
        })
        .perform()
        .await
        .decode_one()
        .unwrap();

    assert_eq!(resp, Ok(id));
}
