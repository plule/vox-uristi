use rand::{rngs::StdRng, Rng};

pub trait IsSomeAnd<T> {
    fn some_and(&self, f: impl FnOnce(&T) -> bool) -> bool;
}

impl<T> IsSomeAnd<T> for Option<T> {
    fn some_and(&self, f: impl FnOnce(&T) -> bool) -> bool {
        match self {
            None => false,
            Some(x) => f(x),
        }
    }
}

pub trait GenBoolSafe: Rng {
    fn gen_bool_safe(&mut self, probability: f64) -> bool {
        self.gen_bool(probability.clamp(0.0, 1.0))
    }
}

impl<T: Rng> GenBoolSafe for T {}

pub trait StableRng {
    fn stable_rng(&self) -> StdRng;
}

/// Ability to be read from dwarf fortress
pub trait FromDwarfFortress {
    fn read_from_df(&mut self, df: &mut dfhack_remote::Client) -> anyhow::Result<()>;
}
