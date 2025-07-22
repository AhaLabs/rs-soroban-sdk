#![no_std]

use test_contracttrait_lib::{Administratable, Upgradable};

use soroban_sdk::{contract, contractimpl, contracttrait, Env};

#[contract]
pub struct Contract;

#[contracttrait]
impl Administratable for Contract {
    // type Impl = Administratable!();
}

#[contracttrait]
impl Upgradable for Contract {
    type Impl = Upgradable!();
    // type Impl = Administratable!();
}

// type admin = Administratable!();
// Administratable!(Contract, Contract, crate::Contract);

#[contractimpl]
impl Contract {
    fn __constructor(env: &Env, admin: soroban_sdk::Address) {
        Self::init(env, &admin);
    }
}
