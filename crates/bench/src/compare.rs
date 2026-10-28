//! `bench compare` — the team-mode gate (§9): serial vs team paired on
//! (task, model), with a paired bootstrap for the noise band.
//!
//! The gate: team ships when it shows ≥15% higher pass rate OR ≥25%
//! lower wall clock at ≤2× tokens; otherwise it stays experimental.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Deserialize;

/// The gate thresholds, from §9.
pub const PASS_MARGIN: f64 = 0.15;
pub const WALL_MARGIN: f64 = 0.25;
pub const TOKEN_CAP: f64 = 2.0;

#[derive(Debug, Deserialize)]
struct Row {
    task: String,
    model: String,
    #[serde(default)]
    held_out: bool,
    pass: bool,
    prompt_tokens: u64,
    completion_tokens: u64,
    wall_ms: u64,
    /// Board-drain time when the run recorded it (the team's real finish);
    /// falls back to wall_ms for rows written before it existed.
    #[serde(default)]
    work_ms: Option<u64>,
}

/// Pairing key: same task on the same model, serial vs team.
type Key = (String, String);

fn load(path: &PathBuf) -> Result<BTreeMap<Key, Row>, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(path)?;
    let mut map = BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let row: Row = serde_json::from_str(line)
            .map_err(|e| format!("{}:{}: {e}", path.display(), index + 1))?;
        map.insert((row.task.clone(), row.model.clone()), row);
    }
    Ok(map)
}

pub struct CompareReport {
    pub pairs: usize,
    /// How many pairs were held-out variants — verification, not tuning.
    pub held_out_pairs: usize,
    pub serial_pass: usize,
    pub team_pass: usize,
    /// Mean paired pass-rate delta (team − serial), bootstrap CI.
    pub pass_diff: (f64, f64, f64),
    /// Mean paired wall-clock reduction ((serial−team)/serial), CI.
    pub wall_gain: (f64, f64, f64),
    /// Total-token ratio (team / serial); the ≤2× budget.
    pub token_ratio: f64,
    pub verdict: &'static str,
}

/// Deterministic paired bootstrap: resample the pair index with a
/// fixed-seed xorshift so the noise band is reproducible.
fn bootstrap_ci(diffs: &[f64], iterations: usize) -> (f64, f64, f64) {
    let mean = diffs.iter().sum::<f64>() / diffs.len() as f64;
    let mut means = Vec::with_capacity(iterations);
    let mut state = 0x9E3779B97F4A7C15u64;
    let mut rand = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for _ in 0..iterations {
        let mut sum = 0.0;
        for _ in 0..diffs.len() {
            sum += diffs[(rand() as usize) % diffs.len()];
        }
        means.push(sum / diffs.len() as f64);
    }
    means.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let lo = means[iterations / 40];
    let hi = means[iterations - iterations / 40 - 1];
    (mean, lo, hi)
}

/// Compare two result files: every row of `serial` paired with the same
/// (task, model) row of `team`.
pub fn run_compare(
    serial: &PathBuf,
    team: &PathBuf,
) -> Result<CompareReport, Box<dyn std::error::Error>> {
    let base = load(serial)?;
    let cand = load(team)?;
    let mut pass_diffs = Vec::new();
    let mut wall_diffs = Vec::new();
    let mut serial_pass = 0;
    let mut team_pass = 0;
    let mut serial_tokens = 0u64;
    let mut team_tokens = 0u64;
    let mut only_serial = 0;
    let mut held_out_pairs = 0usize;
    for (key, b) in &base {
        let Some(c) = cand.get(key) else {
            only_serial += 1;
            continue;
        };
        serial_pass += b.pass as usize;
        team_pass += c.pass as usize;
        held_out_pairs += b.held_out as usize;
        serial_tokens += b.prompt_tokens + b.completion_tokens;
        team_tokens += c.prompt_tokens + c.completion_tokens;
        pass_diffs.push(c.pass as i32 as f64 - b.pass as i32 as f64);
        // Wall gain is normalized per pair so a long task doesn't
        // dominate the mean. Team rows use work_ms when present — the
        // board's drain time, not the driver's timeout.
        let base_ms = b.work_ms.unwrap_or(b.wall_ms) as f64;
        let cand_ms = c.work_ms.unwrap_or(c.wall_ms) as f64;
        if base_ms > 0.0 {
            wall_diffs.push((base_ms - cand_ms) / base_ms);
        }
    }
    let pairs = pass_diffs.len();
    if pairs == 0 {
        return Err("no paired runs — check task/model alignment".into());
    }
    let pass_ci = bootstrap_ci(&pass_diffs, 10_000);
    let wall_ci = bootstrap_ci(&wall_diffs, 10_000);
    let token_ratio = if serial_tokens == 0 {
        f64::INFINITY
    } else {
        team_tokens as f64 / serial_tokens as f64
    };
    // Each axis is judged by the lower bound of its noise band, never the
    // point estimate: one flipped pair moves a mean, and a small-N mean is
    // the lucky run §11 warns about. A tie on the band is not a win.
    let wins = pass_ci.1 >= PASS_MARGIN || wall_ci.1 >= WALL_MARGIN;
    let verdict = if wins && token_ratio <= TOKEN_CAP {
        "ship"
    } else {
        "experimental"
    };
    if only_serial > 0 {
        eprintln!("warning: {only_serial} serial runs have no team pair");
    }
    Ok(CompareReport {
        pairs,
        held_out_pairs,
        serial_pass,
        team_pass,
        pass_diff: pass_ci,
        wall_gain: wall_ci,
        token_ratio,
        verdict,
    })
}

impl std::fmt::Display for CompareReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "paired runs: {} (held-out: {})",
            self.pairs, self.held_out_pairs
        )?;
        writeln!(
            f,
            "pass rate:  {}/{} -> {}/{}   diff {:+.3} CI [{:+.3}, {:+.3}]",
            self.serial_pass,
            self.pairs,
            self.team_pass,
            self.pairs,
            self.pass_diff.0,
            self.pass_diff.1,
            self.pass_diff.2
        )?;
        writeln!(
            f,
            "wall clock: mean gain {:+.1}%   CI [{:+.1}%, {:+.1}%]",
            self.wall_gain.0 * 100.0,
            self.wall_gain.1 * 100.0,
            self.wall_gain.2 * 100.0
        )?;
        writeln!(f, "tokens:     team/serial ratio {:.2}×", self.token_ratio)?;
        write!(
            f,
            "verdict:    {} (gate: ≥15% pass or ≥25% wall at ≤2× tokens)",
            self.verdict
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn row(task: &str, mode: &str, pass: bool, tokens: u64, wall: u64) -> String {
        serde_json::json!({
            "task": task, "mode": mode, "model": "m", "profile": "p",
            "pass": pass, "prompt_tokens": tokens, "completion_tokens": 0,
            "wall_ms": wall, "held_out": false, "done": pass,
            "stop_reason": "done", "steps": 1, "tool_calls": 1,
            "model_calls": 1, "workers": 0,
        })
        .to_string()
    }

    fn file(dir: &Path, name: &str, rows: &[String]) -> PathBuf {
        let path = dir.join(name);
        let mut f = std::fs::File::create(&path).expect("file");
        for r in rows {
            writeln!(f, "{r}").expect("write");
        }
        path
    }

    use std::path::Path;

    #[test]
    fn team_better_on_wall_and_inside_budget_ships() {
        let dir = tempfile::tempdir().expect("dir");
        let serial = file(
            dir.path(),
            "s.jsonl",
            &[
                row("t1", "serial", true, 1000, 1000),
                row("t2", "serial", true, 1000, 800),
            ],
        );
        let team = file(
            dir.path(),
            "t.jsonl",
            &[
                row("t1", "team", true, 1500, 500),
                row("t2", "team", true, 1500, 400),
            ],
        );
        let report = run_compare(&serial, &team).expect("compare");
        assert_eq!(report.pairs, 2);
        assert!(report.wall_gain.0 > 0.4);
        assert_eq!(report.token_ratio, 1.5);
        assert_eq!(report.verdict, "ship");
    }

    #[test]
    fn over_budget_stays_experimental() {
        let dir = tempfile::tempdir().expect("dir");
        let serial = file(
            dir.path(),
            "s.jsonl",
            &[row("t1", "serial", true, 1000, 1000)],
        );
        // Faster but 3× the tokens: loses the gate.
        let team = file(dir.path(), "t.jsonl", &[row("t1", "team", true, 3000, 400)]);
        let report = run_compare(&serial, &team).expect("compare");
        assert_eq!(report.verdict, "experimental");
    }

    #[test]
    fn a_lucky_flip_does_not_ship_a_slower_team() {
        // The shape the M8 gate hit: one pair flips, so the mean pass
        // diff clears 15%, while the team is an order of magnitude
        // slower. The noise band exists for exactly this run.
        let dir = tempfile::tempdir().expect("dir");
        let serial = file(
            dir.path(),
            "s.jsonl",
            &[
                row("t1", "serial", true, 1000, 1_000),
                row("t2", "serial", false, 1000, 1_000),
                row("t3", "serial", true, 1000, 1_000),
                row("t4", "serial", true, 1000, 1_000),
                row("t5", "serial", true, 1000, 1_000),
            ],
        );
        let team = file(
            dir.path(),
            "t.jsonl",
            &[
                row("t1", "team", true, 1000, 20_000),
                row("t2", "team", true, 1000, 20_000),
                row("t3", "team", true, 1000, 20_000),
                row("t4", "team", true, 1000, 20_000),
                row("t5", "team", true, 1000, 20_000),
            ],
        );
        let report = run_compare(&serial, &team).expect("compare");
        assert!(report.pass_diff.0 > PASS_MARGIN, "the mean clears the bar");
        assert!(
            report.pass_diff.1 < PASS_MARGIN,
            "the noise band does not: {:?}",
            report.pass_diff
        );
        assert_eq!(report.verdict, "experimental");
    }

    #[test]
    fn no_pairs_is_an_error_not_a_verdict() {
        let dir = tempfile::tempdir().expect("dir");
        let serial = file(
            dir.path(),
            "s.jsonl",
            &[row("t1", "serial", true, 100, 100)],
        );
        let team = file(dir.path(), "t.jsonl", &[row("t2", "team", true, 100, 100)]);
        assert!(run_compare(&serial, &team).is_err());
    }
}
