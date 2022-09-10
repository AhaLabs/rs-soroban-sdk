#![no_std]
use soroban_auth::{Identifier, Signature};
use soroban_sdk::{contractimpl, contracttype, xdr::ScStatusType, BigInt, Env, Status};

pub struct Contract;

#[repr(u32)]
pub enum ErrorCodes {
    AlreadyInitialized = 1,
    AmountNegative = 2,
    InsufficientBalance = 3,
}

const OK: Status = Status::OK;
const ERROR_ALREADY_INITIALIZED: Status = Status::from_type_and_code(
    ScStatusType::UnknownError,
    ErrorCodes::AlreadyInitialized as u32,
);
const ERROR_AMOUNT_NEGATIVE: Status = Status::from_type_and_code(
    ScStatusType::UnknownError,
    ErrorCodes::AmountNegative as u32,
);
const ERROR_INSUFFICIENT_BALANCE: Status = Status::from_type_and_code(
    ScStatusType::UnknownError,
    ErrorCodes::InsufficientBalance as u32,
);

#[contracttype]
pub struct Config {
    pub admin: Identifier,
}

#[contracttype]
pub enum DataKey {
    Config,
    Balance(Identifier),
}

#[contractimpl]
impl Contract {
    pub fn initialize(env: Env, config: Config) -> Status {
        if env.contract_data().has(DataKey::Config) {
            ERROR_ALREADY_INITIALIZED
        } else {
            env.contract_data().set(DataKey::Config, config);
            OK
        }
    }

    pub fn balance(env: Env, id: Identifier) -> BigInt {
        let data = env.contract_data();

        data.get(DataKey::Balance(id.clone()))
            .unwrap_or_else(|| Ok(BigInt::zero(&env)))
            .unwrap()
    }

    pub fn send(
        env: Env,
        _nonce: BigInt,
        from: Signature,
        to: Identifier,
        amount: BigInt,
    ) -> Status {
        // TODO: auth

        let from = from.get_identifier(&env);

        if amount < 0 {
            return ERROR_AMOUNT_NEGATIVE;
        }

        let data = env.contract_data();

        let from_balance_key = DataKey::Balance(from);
        let mut from_balance: BigInt = data
            .get(&from_balance_key)
            .unwrap_or_else(|| Ok(BigInt::zero(&env)))
            .unwrap();

        let to_balance_key = DataKey::Balance(to);
        let mut to_balance: BigInt = data
            .get(&to_balance_key)
            .unwrap_or_else(|| Ok(BigInt::zero(&env)))
            .unwrap();

        from_balance -= &amount;
        to_balance += &amount;

        if from_balance >= 0 {
            data.set(&from_balance_key, from_balance);
            data.set(&to_balance_key, to_balance);
            OK
        } else {
            ERROR_INSUFFICIENT_BALANCE
        }
    }

    pub fn mint(
        env: Env,
        _nonce: BigInt,
        _admin: Signature,
        to: Identifier,
        amount: BigInt,
    ) -> Status {
        // TODO: auth

        if amount < 0 {
            return ERROR_AMOUNT_NEGATIVE;
        }

        let data = env.contract_data();

        let to_balance_key = DataKey::Balance(to);
        let mut to_balance: BigInt = data
            .get(&to_balance_key)
            .unwrap_or_else(|| Ok(BigInt::zero(&env)))
            .unwrap();

        to_balance += amount;

        data.set(&to_balance_key, to_balance);
        OK
    }
}

#[cfg(test)]
mod test;
