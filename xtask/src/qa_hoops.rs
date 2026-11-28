//! `cargo xtask qa-hoops`: the Add-on 40 hoop QA matrix (`crates/engine/tests/qa_hoops.rs`).

use std::path::Path;

use crate::{cargo, run};

fn test_command() -> std::process::Command {
    let mut c = cargo();
    c.args(["test", "-p", "vectorcraft-engine", "--test", "qa_hoops"]);
    c
}

/// The full matrix, writing the report (slow: not part of `ci`).
pub fn run_full(root: &Path) -> Result<(), String> {
    let report = root.join("docs/qa/addon-40-report.md");
    // The golden and overlay checks first: no report is written if they fail.
    let mut g = cargo();
    g.args(["test", "-p", "vectorcraft-engine", "--test", "qa_hoops_golden"]);
    run(g, "cargo test -p vectorcraft-engine --test qa_hoops_golden")?;
    let mut p = cargo();
    p.args(["test", "-p", "vectorcraft-engine", "--test", "qa_hoops_persist"]);
    run(p, "cargo test -p vectorcraft-engine --test qa_hoops_persist")?;
    let mut c = test_command();
    c.env("QA_HOOPS_FULL", "1").env("QA_REPORT", &report);
    run(c, "cargo test -p vectorcraft-engine --test qa_hoops (full matrix)")?;
    eprintln!("report: {}", report.display());
    Ok(())
}

/// The 3-hoop subset and the persistence check, no report (what `ci` runs).
pub fn fast() -> Result<(), String> {
    run(test_command(), "cargo test -p vectorcraft-engine --test qa_hoops (3 hoops)")?;
    let mut p = cargo();
    p.args(["test", "-p", "vectorcraft-engine", "--test", "qa_hoops_persist"]);
    run(p, "cargo test -p vectorcraft-engine --test qa_hoops_persist")
}
