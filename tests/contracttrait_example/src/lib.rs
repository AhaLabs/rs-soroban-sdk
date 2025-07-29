#![no_std]

use test_contracttrait_lib::{Administratable, Upgradable};

use soroban_sdk::{contract, contractimpl, contracttrait, Env};

#[contract]
pub struct Contract;

#[contracttrait]
impl Administratable for Contract {}

#[contracttrait]
impl Upgradable for Contract {}

#[contractimpl]
impl Contract {
    fn __constructor(env: &Env, admin: soroban_sdk::Address) {
        Self::set_admin(env, admin);
    }
}
