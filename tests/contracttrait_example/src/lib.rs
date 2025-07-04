#![no_std]

use contracttrait_lib::{Administratable, AdministratableExt, Upgradable};

use soroban_sdk::{contract, derive_contract, Env};

#[contract]
#[derive_contract(
    Administratable,
    Upgradable(ext = AdministratableExt)
)]
pub struct Contract;

#[soroban_sdk::contractimpl]
impl Contract {
    pub fn __constructor(env: &Env, admin: soroban_sdk::Address) {
        Self::init(env, &admin);
    }
}
