use soroban_auth::{Ed25519Signature, Identifier, Signature};
use soroban_sdk::{bigint, BytesN, Env, Status};

use crate::{Config, Contract, ContractClient};

#[test]
fn test() {
    let env = Env::default();
    let contract_id = BytesN::from_array(&env, &[0; 32]);
    env.register_contract(&contract_id, Contract);
    let client = ContractClient::new(&env, &contract_id);

    let admin = BytesN::from_array(&env, &[1; 32]);
    let aaa = BytesN::from_array(&env, &[2; 32]);
    let bbb = BytesN::from_array(&env, &[3; 32]);

    assert_eq!(
        client.initialize(&Config {
            admin: soroban_auth::Identifier::Ed25519(admin.clone()),
        }),
        Status::OK
    );

    assert_eq!(
        client.mint(
            &bigint!(&env, 0),
            &Signature::Ed25519(Ed25519Signature {
                public_key: admin.clone(),
                signature: BytesN::from_array(&env, &[0; 64])
            }),
            &Identifier::Ed25519(aaa.clone()),
            &bigint!(&env, 10),
        ),
        Status::OK,
    );

    assert_eq!(
        client.balance(&Identifier::Ed25519(aaa.clone()),),
        bigint!(&env, 10),
    );

    assert_eq!(
        client.send(
            &bigint!(&env, 0),
            &Signature::Ed25519(Ed25519Signature {
                public_key: aaa.clone(),
                signature: BytesN::from_array(&env, &[0; 64])
            }),
            &Identifier::Ed25519(bbb.clone()),
            &bigint!(&env, 3),
        ),
        Status::OK,
    );

    assert_eq!(
        client.balance(&Identifier::Ed25519(aaa.clone()),),
        bigint!(&env, 7),
    );

    assert_eq!(
        client.balance(&Identifier::Ed25519(bbb.clone()),),
        bigint!(&env, 3),
    );
}
