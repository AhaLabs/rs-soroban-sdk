use crate::admin::{Administratable, AdministratableExt};
use soroban_sdk::contracttrait;

#[contracttrait(default = Upgrader, extension_required = true)]
pub trait Upgradable {
    fn upgrade(env: &soroban_sdk::Env, wasm_hash: soroban_sdk::BytesN<32>);
}

pub struct Upgrader;

impl Upgradable for Upgrader {
    type Impl = Upgrader;
    fn upgrade(env: &soroban_sdk::Env, wasm_hash: soroban_sdk::BytesN<32>) {
        env.deployer().update_current_contract_wasm(wasm_hash);
    }
}

impl<T: Administratable, N: Upgradable> Upgradable for AdministratableExt<T, N> {
    type Impl = N;
    fn upgrade(env: &soroban_sdk::Env, wasm_hash: soroban_sdk::BytesN<32>) {
        T::require_admin(env);
        N::upgrade(env, wasm_hash);
    }
}
