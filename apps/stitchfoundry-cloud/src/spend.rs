//! The family-wide AI cost log: every AI call any Foundry app makes (product, feature, model,
//! tokens, what it really cost, the credits charged), kept as totals by day, product and feature,
//! plus what was sold. The totals feed the admin spend page and the monthly budget (email at 80%,
//! free credits paused at 100%). The calls themselves are appended to `ai_calls.jsonl` (or
//! `sf_ai_calls`), never rewritten.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Millionths of a dollar in one cent.
pub const MICRO_PER_CENT: u64 = 10_000;

/// The longest product or feature name kept.
const MAX_NAME: usize = 32;

/// A product or feature name as kept: lower-case `a-z`, `0-9`, `.`, `_` and `-`, up to 32;
/// `None` when it is empty or has anything else (so a name never carries markup or the key's `|`).
pub fn good_name(s: &str) -> Option<String> {
    let s = s.trim().to_lowercase();
    let ok = !s.is_empty()
        && s.chars().count() <= MAX_NAME
        && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-'));
    ok.then_some(s)
}

/// A feature name made from free text (`"sew-out check (free)"` → `"sew-out-check"`).
pub fn feature_of(note: &str) -> String {
    let words: Vec<&str> = note.split(|c: char| !c.is_ascii_alphanumeric() && c != '-').filter(|w| !w.is_empty() && *w != "free").collect();
    good_name(&words.join("-")).unwrap_or_else(|| "ai".into())
}

/// Whole micro-dollars in `usd`, 0 for anything not a finite non-negative number and at most $1,000.
pub fn micro(usd: f64) -> u64 {
    let m = (usd * 1e6).round();
    if m.is_finite() && m > 0.0 { m.min(1e9) as u64 } else { 0 }
}

/// One AI call, as reported.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AiCall {
    pub at: u64,
    pub product: String,
    pub feature: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    /// What it cost (micro-dollars).
    pub cost_micro: u64,
    /// The part of that cost paid by free credits, or by nothing (a free offer).
    pub free_cost_micro: u64,
    /// The credits charged for it.
    pub credits: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<u64>,
}

/// What a set of calls added up to.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Totals {
    pub calls: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_micro: u64,
    pub free_cost_micro: u64,
    pub credits: u64,
}

impl Totals {
    fn add(&mut self, o: &Totals) {
        self.calls = self.calls.saturating_add(o.calls);
        self.input_tokens = self.input_tokens.saturating_add(o.input_tokens);
        self.output_tokens = self.output_tokens.saturating_add(o.output_tokens);
        self.cost_micro = self.cost_micro.saturating_add(o.cost_micro);
        self.free_cost_micro = self.free_cost_micro.saturating_add(o.free_cost_micro);
        self.credits = self.credits.saturating_add(o.credits);
    }

    fn json(&self) -> Value {
        json!({
            "calls": self.calls, "inputTokens": self.input_tokens, "outputTokens": self.output_tokens,
            "costUsd": self.cost_micro as f64 / 1e6, "freeCostUsd": self.free_cost_micro as f64 / 1e6, "credits": self.credits,
        })
    }
}

/// The log's totals, kept in the store's state.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Spend {
    /// By `"<day>|<product>|<feature>"`, a day being unix seconds / 86400.
    totals: BTreeMap<String, Totals>,
    /// Credits bought, by month (`"2026-11"`).
    sold: BTreeMap<String, u64>,
    /// The highest budget level (80 or 100) already emailed, by month.
    mailed: BTreeMap<String, u8>,
}

fn day_key(key: &str) -> Option<(u64, &str, &str)> {
    let mut p = key.splitn(3, '|');
    Some((p.next()?.parse().ok()?, p.next()?, p.next()?))
}

impl Spend {
    pub fn record(&mut self, c: &AiCall) {
        let key = format!("{}|{}|{}", c.at / 86_400, c.product, c.feature);
        let t = Totals {
            calls: 1,
            input_tokens: c.input_tokens,
            output_tokens: c.output_tokens,
            cost_micro: c.cost_micro,
            free_cost_micro: c.free_cost_micro.min(c.cost_micro),
            credits: c.credits,
        };
        self.totals.entry(key).or_default().add(&t);
    }

    pub fn add_sold(&mut self, month: &str, credits: u64) {
        let s = self.sold.entry(month.to_string()).or_insert(0);
        *s = s.saturating_add(credits);
    }

    fn sum(&self, keep: impl Fn(u64, &str, &str) -> bool) -> Totals {
        let mut t = Totals::default();
        for (k, v) in &self.totals {
            if let Some((d, p, f)) = day_key(k)
                && keep(d, p, f)
            {
                t.add(v);
            }
        }
        t
    }

    /// What all products' AI calls cost in `month` (micro-dollars).
    pub fn month_cost(&self, month: &str) -> u64 {
        self.sum(|d, _, _| crate::sewout::month(d * 86_400) == month).cost_micro
    }

    pub fn mailed(&self, month: &str) -> u8 {
        self.mailed.get(month).copied().unwrap_or(0)
    }

    pub fn set_mailed(&mut self, month: &str, level: u8) {
        if level == 0 {
            self.mailed.remove(month);
        } else {
            self.mailed.insert(month.to_string(), level);
        }
    }

    /// Today's and the month's spend, by product and by feature, and credits sold against used.
    pub fn summary(&self, now: u64) -> Value {
        let today = now / 86_400;
        let month = crate::sewout::month(now);
        let in_month = |d: u64| crate::sewout::month(d * 86_400) == month;
        let by = |pick: fn(&str, &str) -> String| {
            let mut m: BTreeMap<String, Totals> = BTreeMap::new();
            for (k, v) in &self.totals {
                if let Some((d, p, f)) = day_key(k)
                    && in_month(d)
                {
                    m.entry(pick(p, f)).or_default().add(v);
                }
            }
            m.into_iter().map(|(k, v)| (k, v.json())).collect::<serde_json::Map<_, _>>()
        };
        let month_t = self.sum(|d, _, _| in_month(d));
        json!({
            "today": self.sum(|d, _, _| d == today).json(),
            "month": month_t.json(),
            "monthName": month,
            "byProduct": by(|p, _| p.to_string()),
            "byFeature": by(|p, f| format!("{p}/{f}")),
            "creditsSold": self.sold.get(&month).copied().unwrap_or(0),
            "creditsUsed": month_t.credits,
            "freeCreditCostUsd": month_t.free_cost_micro as f64 / 1e6,
        })
    }
}

/// The products allowed to report (`SF_CLOUD_PRODUCT_KEYS`, `product:key` apart by commas); a key
/// under 24 characters or a bad name is left out.
pub fn parse_product_keys(s: &str) -> Vec<(String, String)> {
    s.split(',')
        .filter_map(|p| {
            let (name, key) = p.split_once(':')?;
            let key = key.trim();
            (key.chars().count() >= 24).then_some(())?;
            Some((good_name(name)?, key.to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(at: u64, product: &str, feature: &str, cost: u64, free: u64, credits: u64) -> AiCall {
        AiCall {
            at,
            product: product.into(),
            feature: feature.into(),
            model: "m".into(),
            input_tokens: 10,
            output_tokens: 5,
            cost_micro: cost,
            free_cost_micro: free,
            credits,
            account: None,
        }
    }

    #[test]
    fn names_are_plain() {
        assert_eq!(good_name(" Laser-Foundry "), Some("laser-foundry".into()));
        for bad in ["", "a b", "a|b", "<b>", &"x".repeat(33)] {
            assert_eq!(good_name(bad), None, "{bad}");
        }
        assert_eq!(feature_of("sew-out check (free)"), "sew-out-check");
        assert_eq!(feature_of("AI request"), "ai-request");
        assert_eq!(feature_of("!!!"), "ai");
        assert_eq!(micro(f64::NAN), 0);
        assert_eq!(micro(-1.0), 0);
        assert_eq!(micro(0.5), 500_000);
        assert_eq!(micro(1e12), 1_000_000_000);
    }

    #[test]
    fn totals_by_day_product_and_feature() {
        let mut s = Spend::default();
        let day = 20_000 * 86_400; // 2024-10-04
        s.record(&call(day, "stitchfoundry", "ask", 80_000, 0, 24));
        s.record(&call(day + 5, "stitchfoundry", "ask", 20_000, 20_000, 0));
        s.record(&call(day + 9, "laserfoundry", "trace", 50_000, 10_000, 5));
        s.record(&call(day - 86_400 * 40, "laserfoundry", "trace", 999, 0, 1));
        s.add_sold(&crate::sewout::month(day), 1000);
        let v = s.summary(day + 100);
        assert_eq!(v["today"]["calls"], 3);
        assert_eq!(v["month"]["costUsd"], 0.15);
        assert_eq!(v["byProduct"]["stitchfoundry"]["calls"], 2);
        assert_eq!(v["byFeature"]["laserfoundry/trace"]["costUsd"], 0.05);
        assert_eq!(v["creditsSold"], 1000);
        assert_eq!(v["creditsUsed"], 29);
        assert_eq!(v["freeCreditCostUsd"], 0.03);
        assert_eq!(s.month_cost(&crate::sewout::month(day)), 150_000);
    }

    #[test]
    fn product_keys_need_a_good_name_and_a_long_key() {
        let k = "k".repeat(24);
        assert_eq!(
            parse_product_keys(&format!("laser:{k}, bad name:{k},short:abc,stitch:{k}")),
            vec![("laser".into(), k.clone()), ("stitch".into(), k)]
        );
        assert!(parse_product_keys("").is_empty());
    }
}
