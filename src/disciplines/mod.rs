//! The ten Decathlon disciplines and the building blocks they share.

pub mod best_of_n;
pub mod freeze;
pub mod heights;
pub mod reroll_sets;

pub mod discus;
pub mod highjump;
pub mod hurdles;
pub mod javelin;
pub mod longjump;
pub mod m100;
pub mod m1500;
pub mod m400;
pub mod polevault;
pub mod shotput;

use crate::policy::Solved;

/// A discipline solver entry point.
pub type Solver = fn() -> Solved;

/// All disciplines in competition order, paired with their key.
pub fn registry() -> Vec<(&'static str, Solver)> {
    vec![
        ("100m", m100::solve),
        ("longjump", longjump::solve),
        ("shotput", shotput::solve),
        ("highjump", highjump::solve),
        ("400m", m400::solve),
        ("110mh", hurdles::solve),
        ("discus", discus::solve),
        ("polevault", polevault::solve),
        ("javelin", javelin::solve),
        ("1500m", m1500::solve),
    ]
}

/// Look up a single discipline solver by key.
pub fn find(key: &str) -> Option<Solver> {
    registry()
        .into_iter()
        .find(|(k, _)| *k == key)
        .map(|(_, s)| s)
}
