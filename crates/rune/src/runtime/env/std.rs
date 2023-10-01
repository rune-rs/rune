use core::cell::Cell;

use super::Env;

std::thread_local!(static ENV: Cell<Env> = Cell::new(Env::null()));

pub(super) fn rune_env_get() -> Env {
    ENV.with(|env| env.get())
}

pub(super) fn rune_env_replace(env: Env) -> Env {
    ENV.with(|e| e.replace(env))
}
