#![no_std]

use test_contracttrait_lib::{Administratable, AdministratableExt, Upgradable};

use soroban_sdk::{contract, contractimpl, contracttrait, derive_contract, Env};

#[contract]
#[derive_contract(
    Upgradable(ext = AdministratableExt),
    // Constructable(default = Contract),
)]
pub struct Contract;

#[contracttrait]
impl Administratable for Contract {
    // type Impl = Administratable!();
}
// type admin = Administratable!();
// Administratable!(Contract, Contract, crate::Contract);

// #[contractimpl]
// impl Constructable<soroban_sdk::Address> for Contract {
//     type Impl = Self;
//     fn __constructor(env: &Env, admin: soroban_sdk::Address) {
//         Self::init(env, &admin);
//     }
// }
