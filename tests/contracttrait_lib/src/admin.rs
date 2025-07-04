use soroban_sdk::{contracttrait, symbol_short, Address, Env, Symbol};

/// Trait for using an admin address to control access.
#[contracttrait(default = Admin, is_extension = true)]
pub trait Administratable {
    fn admin(env: &Env) -> soroban_sdk::Address;
    fn set_admin(env: &Env, new_admin: &soroban_sdk::Address);

    #[internal]
    fn require_admin(env: &Env) {
        Self::admin(env).require_auth();
    }

    #[internal]
    fn init(env: &Env, admin: &soroban_sdk::Address);
}

pub const STORAGE_KEY: Symbol = symbol_short!("A");

fn get(env: &Env) -> Option<Address> {
    env.storage().instance().get(&STORAGE_KEY)
}

pub struct Admin;

impl Administratable for Admin {
    type Impl = Admin;
    fn admin(env: &Env) -> soroban_sdk::Address {
        unsafe { get(env).unwrap_unchecked() }
    }
    fn set_admin(env: &Env, new_admin: &soroban_sdk::Address) {
        Self::require_admin(env);
        env.storage().instance().set(&STORAGE_KEY, &new_admin);
    }

    fn init(env: &Env, admin: &soroban_sdk::Address) {
        if get(env).is_some() {
            panic!("Admin already initialized");
        }
        env.storage().instance().set(&STORAGE_KEY, &admin);
    }
}
