//! Fair value for quoting: the Binance USD-M perp mid plus a slow EMA of the
//! basis (Bullet mid minus Binance mid), or the Bullet mid.
//!
//! The basis EMA absorbs USD vs USDT and any steady venue premium, so the fair
//! value tracks Binance moves without drifting from where Bullet trades. When
//! the Binance mid is missing or stale, or the basis has not been averaged for
//! [`BASIS_WARMUP_SECS`], the Bullet mid is used and the reason is reported.
//! Quoting never stops because Binance is down.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "Decimal prices and basis sit far below 7.9e28; mid checked > 0 before dividing"
)]

use std::time::{Duration, Instant};

use bullet_agents::rust_decimal::Decimal;
use bullet_agents::rust_decimal::prelude::FromPrimitive;
pub use bullet_agents::strategy::{BASIS_WARMUP_SECS, Fallback};
use bullet_agents::strategy::{FairValue, MmParams};

/// Seconds of continuous divergence after which the basis is re-learned.
pub const BASIS_RELEARN_SECS: u64 = 300;
/// Decimal places kept in the basis EMA (keeps quote prices tidy).
const BASIS_DP: u32 = 8;

/// The fair value quotes are centred on, and where it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fair {
    pub price: Decimal,
    pub source: FairValue,
    /// Set when `binance` is configured but the Bullet mid is used.
    pub fallback: Option<Fallback>,
}

#[derive(Debug, Clone, Copy)]
struct Basis {
    ema: Decimal,
    updated: Instant,
    since: Instant,
    /// Start of the current run of divergent (left out) samples.
    diverged_since: Option<Instant>,
}

/// Tracks the Binance mid and the basis EMA. Times are actor clock instants
/// (monotonic, so a wall-clock step cannot make a stale mid look fresh).
#[derive(Debug, Default)]
pub struct FairValueTracker {
    /// Last Binance mid and when it arrived (the feed's receive time, so a
    /// backlogged update is judged by its real age).
    reference: Option<(Decimal, Instant)>,
    basis: Option<Basis>,
}

impl FairValueTracker {
    /// A Binance best bid / ask, received at `received_at`.
    pub fn on_reference(&mut self, bid: Decimal, ask: Decimal, received_at: Instant) {
        let newer = self.reference.is_none_or(|(_, at)| received_at >= at);
        if bid > Decimal::ZERO && ask >= bid && newer {
            self.reference = Some(((bid + ask) / Decimal::TWO, received_at));
        }
    }

    fn fresh_reference(&self, now: Instant, stale_secs: u64) -> Result<Decimal, Fallback> {
        match self.reference {
            None => Err(Fallback::NoReference),
            Some((_, at))
                if now.saturating_duration_since(at) > Duration::from_secs(stale_secs) =>
            {
                Err(Fallback::ReferenceStale { secs: stale_secs })
            }
            Some((mid, _)) => Ok(mid),
        }
    }

    /// A fresh Bullet mid: fold `mid - binance_mid` into the basis. Returns the
    /// shift in bps when a sustained divergence restarts the warm-up. Without a
    /// fresh Binance mid the basis is dropped, so it warms up again. Each
    /// sample gets weight `max(EMA weight, dt / time since the first sample)`:
    /// early on that is the time-weighted mean of the samples so far (the first
    /// one does not dominate the warm-up), later the half-life EMA.
    pub fn on_bullet_mid(
        &mut self,
        mid: Decimal,
        now: Instant,
        params: &MmParams,
    ) -> Option<Decimal> {
        let Ok(reference) = self.fresh_reference(now, params.reference_stale_secs) else {
            self.basis = None;
            return None;
        };
        let sample = mid - reference;
        let seed = Basis { ema: sample, updated: now, since: now, diverged_since: None };
        let Some(b) = self.basis else {
            self.basis = Some(seed);
            return None;
        };
        let warm = now.saturating_duration_since(b.since) >= Duration::from_secs(BASIS_WARMUP_SECS);
        let off_bps = if mid > Decimal::ZERO {
            (sample - b.ema) / mid * Decimal::from(10_000)
        } else {
            Decimal::ZERO
        };
        // Once warm, a sample that would put the fair value more than
        // `max_divergence_bps` from the Bullet mid is left out, so a divergent
        // Binance does not drag the basis after it. If that lasts
        // `BASIS_RELEARN_SECS`, the basis has really moved: warm up again from
        // the current sample.
        if warm && off_bps.abs() > params.max_divergence_bps {
            let since = b.diverged_since.unwrap_or(now);
            if now.saturating_duration_since(since) >= Duration::from_secs(BASIS_RELEARN_SECS) {
                self.basis = Some(seed);
                return Some(off_bps.round_dp(1));
            }
            self.basis = Some(Basis { diverged_since: Some(since), ..b });
            return None;
        }
        let alpha = ema_alpha(
            now.saturating_duration_since(b.updated),
            now.saturating_duration_since(b.since),
            params.basis_halflife_secs,
        );
        self.basis = Some(Basis {
            ema: (b.ema + alpha * (sample - b.ema)).round_dp(BASIS_DP),
            updated: now,
            since: b.since,
            diverged_since: None,
        });
        None
    }

    /// The fair value at `now` given the Bullet mid.
    #[must_use]
    pub fn fair(&self, bullet_mid: Decimal, now: Instant, params: &MmParams) -> Fair {
        let bullet = |fallback| Fair { price: bullet_mid, source: FairValue::Bullet, fallback };
        if params.fair_value == FairValue::Bullet {
            return bullet(None);
        }
        let reference = match self.fresh_reference(now, params.reference_stale_secs) {
            Ok(r) => r,
            Err(why) => return bullet(Some(why)),
        };
        let price = match self.basis {
            Some(b)
                if now.saturating_duration_since(b.since)
                    >= Duration::from_secs(BASIS_WARMUP_SECS) =>
            {
                reference + b.ema
            }
            _ => return bullet(Some(Fallback::BasisWarming)),
        };
        if bullet_mid > Decimal::ZERO {
            let bps = ((price - bullet_mid).abs() / bullet_mid * Decimal::from(10_000)).round_dp(1);
            if bps > params.max_divergence_bps {
                return bullet(Some(Fallback::Diverged { bps }));
            }
        }
        Fair { price, source: FairValue::Binance, fallback: None }
    }

    /// The basis EMA in bps of `bullet_mid` (2 dp), once there is one.
    #[must_use]
    pub fn basis_bps(&self, bullet_mid: Decimal) -> Option<Decimal> {
        let b = self.basis?;
        (bullet_mid > Decimal::ZERO)
            .then(|| (b.ema / bullet_mid * Decimal::from(10_000)).round_dp(2))
    }
}

/// Weight of a sample `dt` after the previous one, `elapsed` after the first:
/// `max(1 - 2^(-dt/half_life), dt/elapsed)`. A sample at the same instant as
/// the previous one (`dt` zero) weighs nothing.
fn ema_alpha(dt: Duration, elapsed: Duration, halflife_secs: u64) -> Decimal {
    if dt.is_zero() {
        return Decimal::ZERO;
    }
    let half_life = Duration::from_secs(halflife_secs.max(1)).as_secs_f64();
    let ema = 1.0 - 0.5_f64.powf(dt.as_secs_f64() / half_life);
    // elapsed >= dt > 0.
    let mean = dt.as_secs_f64() / elapsed.as_secs_f64();
    Decimal::from_f64(ema.max(mean).clamp(0.0, 1.0)).unwrap_or(Decimal::ONE)
}

#[cfg(test)]
mod tests {
    use std::sync::OnceLock;

    use bullet_agents::testing;

    use super::*;

    /// `ms` after a fixed test epoch.
    fn at(ms: u64) -> Instant {
        static BASE: OnceLock<Instant> = OnceLock::new();
        *BASE.get_or_init(Instant::now) + Duration::from_millis(ms)
    }

    fn d(s: &str) -> Decimal {
        testing::dec(s)
    }

    fn params(fair_value: FairValue) -> MmParams {
        MmParams { fair_value, ..testing::params() }
    }

    /// Binance at 100 / 102 (mid 101), Bullet mid 101.5 every second from
    /// t = 0 to `secs`.
    fn warmed(secs: u64, p: &MmParams) -> FairValueTracker {
        let mut t = FairValueTracker::default();
        for s in 0..=secs {
            t.on_reference(d("100"), d("102"), at(s * 1000));
            t.on_bullet_mid(d("101.5"), at(s * 1000), p);
        }
        t
    }

    #[test]
    fn fair_is_binance_mid_plus_basis_once_warm() {
        let p = params(FairValue::Binance);
        let mut t = warmed(BASIS_WARMUP_SECS, &p);
        let fair = t.fair(d("101.5"), at(BASIS_WARMUP_SECS * 1000), &p);
        assert_eq!(fair, Fair { price: d("101.5"), source: FairValue::Binance, fallback: None });
        assert_eq!(t.basis_bps(d("101.5")), Some(d("49.26")));
        // Binance moves up 0.1; the slow basis barely moves, so fair follows Binance.
        let now = (BASIS_WARMUP_SECS + 1) * 1000;
        t.on_reference(d("100.1"), d("102.1"), at(now));
        t.on_bullet_mid(d("101.5"), at(now), &p);
        let fair = t.fair(d("101.5"), at(now), &p);
        assert_eq!(fair.source, FairValue::Binance);
        assert!(fair.price > d("101.59") && fair.price < d("101.60"), "{fair:?}");
    }

    #[test]
    fn basis_ema_moves_half_way_in_one_half_life() {
        let p = params(FairValue::Binance);
        let mut t = FairValueTracker::default();
        // Basis 0 for long enough that the half-life EMA weight dominates.
        let late = 10 * p.basis_halflife_secs * 1000;
        for ms in [0, late] {
            t.on_reference(d("100"), d("100"), at(ms));
            t.on_bullet_mid(d("100"), at(ms), &p);
        }
        let then = late + p.basis_halflife_secs * 1000;
        t.on_reference(d("100"), d("100"), at(then));
        t.on_bullet_mid(d("100.1"), at(then), &p); // sample 0.1 (10 bps)
        assert_eq!(t.basis.unwrap().ema, d("0.05"));
    }

    /// The warm value is the average over the warm-up, not mostly the first
    /// sample: +10 bps once, then 60 s at 0, reads well under 1 bps.
    #[test]
    fn warm_basis_is_the_warm_up_average() {
        let p = params(FairValue::Binance);
        let mut t = FairValueTracker::default();
        t.on_reference(d("100"), d("100"), at(0));
        t.on_bullet_mid(d("100.1"), at(0), &p);
        for s in 1..=BASIS_WARMUP_SECS {
            t.on_reference(d("100"), d("100"), at(s * 1000));
            t.on_bullet_mid(d("100"), at(s * 1000), &p);
        }
        let bps = t.basis_bps(d("100")).unwrap();
        assert!(bps.abs() < d("1"), "{bps}");
    }

    #[test]
    fn falls_back_to_bullet_mid_without_binance() {
        let p = params(FairValue::Binance);
        let t = FairValueTracker::default();
        let fair = t.fair(d("101.5"), at(0), &p);
        assert_eq!(fair.price, d("101.5"));
        assert_eq!(fair.source, FairValue::Bullet);
        assert_eq!(fair.fallback, Some(Fallback::NoReference));
        assert_eq!(t.basis_bps(d("101.5")), None);
    }

    #[test]
    fn falls_back_while_basis_warms_up() {
        let p = params(FairValue::Binance);
        let t = warmed(BASIS_WARMUP_SECS - 1, &p);
        let fair = t.fair(d("101.5"), at((BASIS_WARMUP_SECS - 1) * 1000), &p);
        assert_eq!((fair.price, fair.fallback), (d("101.5"), Some(Fallback::BasisWarming)));
    }

    #[test]
    fn stale_binance_falls_back_and_restarts_warm_up() {
        let p = params(FairValue::Binance);
        let mut t = warmed(BASIS_WARMUP_SECS, &p);
        let stale = (BASIS_WARMUP_SECS + p.reference_stale_secs + 1) * 1000;
        let fair = t.fair(d("101.5"), at(stale), &p);
        assert_eq!(fair.price, d("101.5"));
        assert_eq!(fair.fallback, Some(Fallback::ReferenceStale { secs: 5 }));
        assert!(fair.fallback.unwrap().to_string().contains("Binance mid stale"));
        // A Bullet update while Binance is stale drops the basis: warm-up again.
        t.on_bullet_mid(d("101.5"), at(stale), &p);
        t.on_reference(d("100"), d("102"), at(stale + 1000));
        t.on_bullet_mid(d("101.5"), at(stale + 1000), &p);
        let fair = t.fair(d("101.5"), at(stale + 1000), &p);
        assert_eq!(fair.fallback, Some(Fallback::BasisWarming));
    }

    #[test]
    fn bullet_setting_uses_bullet_mid_without_a_reason() {
        let p = params(FairValue::Bullet);
        let t = warmed(BASIS_WARMUP_SECS, &p);
        let fair = t.fair(d("101.7"), at(BASIS_WARMUP_SECS * 1000), &p);
        assert_eq!(fair, Fair { price: d("101.7"), source: FairValue::Bullet, fallback: None });
    }

    #[test]
    fn divergent_binance_fair_falls_back_and_recovers() {
        let p = params(FairValue::Binance); // max_divergence_bps 50
        let mut t = warmed(BASIS_WARMUP_SECS, &p); // Binance 101, basis +0.5
        let now = (BASIS_WARMUP_SECS + 1) * 1000;
        // Binance jumps to 102.01: fair 102.51 is 99.5 bps above Bullet's 101.5.
        t.on_reference(d("102"), d("102.02"), at(now));
        let fair = t.fair(d("101.5"), at(now), &p);
        assert_eq!(fair.price, d("101.5"));
        assert_eq!(fair.source, FairValue::Bullet);
        assert_eq!(fair.fallback, Some(Fallback::Diverged { bps: d("99.5") }));
        assert_eq!(
            fair.fallback.unwrap().to_string(),
            "Binance fair value diverges from Bullet by 99.5 bps"
        );
        // Exactly at the limit is still within it.
        let at_limit = MmParams { max_divergence_bps: d("99.5"), ..p.clone() };
        assert_eq!(t.fair(d("101.5"), at(now), &at_limit).fallback, None);
        // Back within the limit: Binance again.
        t.on_reference(d("101"), d("101.02"), at(now + 1000));
        let fair = t.fair(d("101.5"), at(now + 1000), &p);
        assert_eq!((fair.source, fair.fallback), (FairValue::Binance, None));
        assert_eq!(fair.price, d("101.51"));
    }

    /// Once warm, a sample beyond `max_divergence_bps` from the basis is left
    /// out; one within it is averaged in.
    #[test]
    fn warm_basis_ignores_divergent_samples() {
        let p = params(FairValue::Binance); // max_divergence_bps 20
        let mut t = warmed(BASIS_WARMUP_SECS, &p); // basis +0.5
        let now = (BASIS_WARMUP_SECS + 1) * 1000;
        t.on_reference(d("102"), d("102.02"), at(now)); // sample -0.51: 100 bps off
        t.on_bullet_mid(d("101.5"), at(now), &p);
        assert_eq!(t.basis.unwrap().ema, d("0.5"));
        t.on_reference(d("100.9"), d("100.9"), at(now + 1000)); // sample 0.6: 10 bps off
        t.on_bullet_mid(d("101.5"), at(now + 1000), &p);
        assert!(t.basis.unwrap().ema > d("0.5"));
    }

    /// Binance mid 100 and Bullet mid `bullet` every second over `[from, to]`
    /// seconds; returns the shifts reported (re-learn starts).
    fn run(t: &mut FairValueTracker, p: &MmParams, bullet: &str, from: u64, to: u64) -> usize {
        let mut relearned = 0;
        for s in from..=to {
            t.on_reference(d("100"), d("100"), at(s * 1000));
            relearned += usize::from(t.on_bullet_mid(d(bullet), at(s * 1000), p).is_some());
        }
        relearned
    }

    /// A basis step of +30 bps that stays: Bullet mid at once, then after
    /// [`BASIS_RELEARN_SECS`] of divergence the basis re-learns the new level.
    #[test]
    fn sustained_basis_shift_is_relearned() {
        let p = params(FairValue::Binance); // max_divergence_bps 20
        let mut t = FairValueTracker::default();
        assert_eq!(run(&mut t, &p, "100", 0, BASIS_WARMUP_SECS), 0);
        let start = BASIS_WARMUP_SECS + 1;
        assert_eq!(run(&mut t, &p, "100.3", start, start), 0);
        let fair = t.fair(d("100.3"), at(start * 1000), &p);
        assert!(matches!(fair.fallback, Some(Fallback::Diverged { .. })), "{fair:?}");

        // Just short of the re-learn time: still divergent, nothing reported.
        let last = start + BASIS_RELEARN_SECS - 1;
        assert_eq!(run(&mut t, &p, "100.3", start + 1, last), 0);
        let fair = t.fair(d("100.3"), at(last * 1000), &p);
        assert!(matches!(fair.fallback, Some(Fallback::Diverged { .. })), "{fair:?}");
        // At the re-learn time the shift is reported once and warm-up restarts.
        let relearn = last + 1;
        t.on_reference(d("100"), d("100"), at(relearn * 1000));
        let moved = t.on_bullet_mid(d("100.3"), at(relearn * 1000), &p);
        assert_eq!(moved, Some(d("29.9")));
        assert_eq!(
            t.fair(d("100.3"), at(relearn * 1000), &p).fallback,
            Some(Fallback::BasisWarming)
        );
        // Warm again: priced off Binance with the new basis.
        let warm = relearn + BASIS_WARMUP_SECS;
        assert_eq!(run(&mut t, &p, "100.3", relearn + 1, warm), 0);
        let fair = t.fair(d("100.3"), at(warm * 1000), &p);
        assert_eq!((fair.source, fair.fallback), (FairValue::Binance, None));
        assert_eq!(t.basis_bps(d("100.3")), Some(d("29.91")));
    }

    /// A divergence shorter than [`BASIS_RELEARN_SECS`] recovers to the old
    /// basis without a re-warm.
    #[test]
    fn brief_divergence_keeps_the_basis() {
        let p = params(FairValue::Binance);
        let mut t = FairValueTracker::default();
        run(&mut t, &p, "100", 0, BASIS_WARMUP_SECS);
        let start = BASIS_WARMUP_SECS + 1;
        let end = start + BASIS_RELEARN_SECS - 60;
        assert_eq!(run(&mut t, &p, "100.3", start, end), 0);
        assert_eq!(run(&mut t, &p, "100", end + 1, end + 1), 0);
        let fair = t.fair(d("100"), at((end + 1) * 1000), &p);
        assert_eq!((fair.source, fair.fallback), (FairValue::Binance, None));
        assert_eq!(t.basis_bps(d("100")), Some(d("0")));
        // A later divergence starts its own clock.
        let again = end + 2;
        assert_eq!(run(&mut t, &p, "100.3", again, again + BASIS_RELEARN_SECS - 1), 0);
    }

    /// A second Bullet mid at the same instant as the first carries no
    /// weight: the basis stays at the first sample's value.
    #[test]
    fn same_instant_sample_does_not_replace_the_basis() {
        let p = params(FairValue::Binance);
        let mut t = FairValueTracker::default();
        t.on_reference(d("100"), d("100"), at(0));
        t.on_bullet_mid(d("100.1"), at(0), &p);
        t.on_bullet_mid(d("100.3"), at(0), &p);
        assert_eq!(t.basis.unwrap().ema, d("0.1"));
    }

    #[test]
    fn crossed_or_empty_binance_book_is_ignored() {
        let mut t = FairValueTracker::default();
        t.on_reference(d("102"), d("100"), at(0));
        t.on_reference(d("0"), d("100"), at(0));
        assert!(t.reference.is_none());
    }
}
