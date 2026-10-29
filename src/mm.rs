//! My market maker's pricing: one bid and one ask at a fair value ± the
//! side's spread, shifted by inventory skew, kept off the touch.
//!
//! The fair value is the Binance mid plus the basis, or the Bullet mid (see
//! [`crate::fair_value`]). Everything else (slots, refresh, guards, history,
//! control, hard limits, the risk gate) is the framework's.
//!
//! This starts as a copy of the framework's simple-mm blueprint; it is yours
//! to change.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "Decimal prices, sizes and bps sit far below 7.9e28; max_position checked > 0 before dividing"
)]

use bullet_agents::rust_decimal::Decimal;
use bullet_agents::strategy::{
    FairReport, FairValue, MarketView, MmParams, Quote, QuoteStrategy, Quotes, Side, Signal,
    SimpleMmConfig,
};
use bullet_agents::tracing;

use crate::fair_value::{Fair, FairValueTracker};

const BPS_SCALE: i64 = 10_000;

/// simple-mm's [`QuoteStrategy`].
#[derive(Debug)]
pub struct SimpleMm {
    /// Binance mid and basis EMA behind the fair value.
    fair: FairValueTracker,
    /// Binance symbol for the fair value (from the config, upper case).
    binance_symbol: String,
}

impl SimpleMm {
    #[must_use]
    pub fn new(config: &SimpleMmConfig) -> Self {
        Self { fair: FairValueTracker::default(), binance_symbol: config.binance_symbol() }
    }

    /// The fair value to quote around, given the Bullet mid.
    fn fair_value(&self, view: &MarketView<'_>) -> Option<Fair> {
        let mid = view.mid?;
        Some(self.fair.fair(mid, view.now, view.params))
    }

    /// Inventory skew in bps applied to both quotes: `-skew_bps × net /
    /// max_position`, with the ratio clamped to ±1. Long lowers both quotes.
    fn skew_shift_bps(view: &MarketView<'_>) -> Decimal {
        if view.params.skew_bps.is_zero() || view.max_position <= Decimal::ZERO {
            return Decimal::ZERO;
        }
        let ratio =
            (view.net_position / view.max_position).clamp(Decimal::NEGATIVE_ONE, Decimal::ONE);
        -view.params.skew_bps * ratio
    }

    /// Quote price for `side`: `fair` ± the side's spread, shifted by skew,
    /// then kept off the touch.
    fn desired_price(view: &MarketView<'_>, side: Side, fair: Decimal) -> Decimal {
        let spread_bps = match side {
            Side::Buy => -view.params.bid_spread_bps,
            Side::Sell => view.params.ask_spread_bps,
        };
        let price = fair
            * (Decimal::ONE + (spread_bps + Self::skew_shift_bps(view)) / Decimal::from(BPS_SCALE));
        view.keep_off_touch(side, price)
    }
}

impl QuoteStrategy for SimpleMm {
    fn on_book(&mut self, view: &MarketView<'_>) {
        let Some(mid) = view.mid else { return };
        if let Some(bps) = self.fair.on_bullet_mid(mid, view.now, view.params) {
            tracing::warn!("simple-mm: Bullet–Binance basis moved by {bps} bps; re-learning it");
        }
    }

    /// Binance best bid / ask for the fair value. Only recorded: quotes move on
    /// the next Bullet book update or tick.
    fn on_signal(&mut self, signal: &Signal, view: &MarketView<'_>) {
        let Signal::Reference(event) = signal else { return };
        // Ignored while pricing off Bullet: switching to binance starts a fresh warm-up.
        if view.params.fair_value == FairValue::Binance
            && event.symbol.eq_ignore_ascii_case(&self.binance_symbol)
        {
            // Stamped with the feed's receive time, so a backlog reads as stale.
            let received_at = event.received_at.min(view.now);
            self.fair.on_reference(event.bid, event.ask, received_at);
        }
    }

    fn on_params(&mut self, before: &MmParams, after: &MmParams) {
        if before.fair_value != FairValue::Binance && after.fair_value == FairValue::Binance {
            // Never reuse a basis warmed before the switch away from binance.
            self.fair = FairValueTracker::default();
        }
    }

    fn quote(&self, view: &MarketView<'_>) -> Quotes {
        let Some(report) = self.fair(view) else { return Quotes::default() };
        let quote = |side| Quote {
            price: Self::desired_price(view, side, report.price),
            size: view.params.order_size,
        };
        Quotes { bid: Some(quote(Side::Buy)), ask: Some(quote(Side::Sell)), fair: Some(report) }
    }

    fn fair(&self, view: &MarketView<'_>) -> Option<FairReport> {
        let fair = self.fair_value(view)?;
        Some(FairReport {
            price: fair.price,
            source: fair.source,
            fallback: fair.fallback,
            basis_bps: view.mid.and_then(|mid| self.fair.basis_bps(mid)),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bullet_agents::strategy::{BASIS_WARMUP_SECS, Fallback};
    use bullet_agents::testing::{Market, config, dec, params, reference};

    use super::*;

    /// Quoting around the Bullet mid: 100 ± 2 bps, size 1.
    fn bullet_params() -> MmParams {
        MmParams { fair_value: FairValue::Bullet, ..params() }
    }

    #[test]
    fn no_book_no_quotes() {
        let mm = SimpleMm::new(&config());
        assert_eq!(mm.quote(&Market::new().view()), Quotes::default());
    }

    #[test]
    fn quotes_both_sides_around_the_bullet_mid() {
        let mm = SimpleMm::new(&config());
        let market = Market::new().book(dec("99"), dec("101")).params(bullet_params());
        let q = mm.quote(&market.view());
        assert_eq!(q.bid, Some(Quote { price: dec("99.98"), size: dec("1") }));
        assert_eq!(q.ask, Some(Quote { price: dec("100.02"), size: dec("1") }));
        let fair = q.fair.unwrap();
        assert_eq!((fair.price, fair.source, fair.fallback), (dec("100"), FairValue::Bullet, None));
    }

    #[test]
    fn long_inventory_lowers_both_quotes() {
        let mm = SimpleMm::new(&config());
        let p = MmParams { skew_bps: dec("10"), ..bullet_params() };
        let market = Market::new().book(dec("99"), dec("101")).params(p).position(dec("0.5"));
        let q = mm.quote(&market.view());
        // -10 bps × 0.5 / 1 = -5 bps on both sides.
        assert_eq!(q.bid.unwrap().price, dec("99.93"));
        assert_eq!(q.ask.unwrap().price, dec("99.97"));
    }

    #[test]
    fn quotes_stay_off_the_touch() {
        let mm = SimpleMm::new(&config());
        let p = MmParams { skew_bps: dec("100"), ..bullet_params() };
        // A tight book and full long skew would put the ask under the best bid.
        let market = Market::new()
            .book(dec("99.99"), dec("100.01"))
            .params(p)
            .position(dec("1"))
            .tick(dec("0.01"));
        let q = mm.quote(&market.view());
        assert_eq!(q.ask.unwrap().price, dec("100.00"));
    }

    /// With `fair_value = "binance"`, quotes start on the Bullet mid while the
    /// basis warms up, then move to the Binance mid plus the basis.
    #[test]
    fn binance_fair_value_after_warm_up() {
        let mut mm = SimpleMm::new(&config());
        let mut market = Market::new().book(dec("99"), dec("101"));
        let start = market.now;
        for s in 0..=BASIS_WARMUP_SECS {
            market = market.at(start + Duration::from_secs(s));
            let binance = reference("BTCUSDT", dec("99.9"), dec("100.1"), market.now);
            mm.on_signal(&Signal::Reference(binance), &market.view());
            mm.on_book(&market.view());
            if s == 0 {
                let fair = mm.fair(&market.view()).unwrap();
                assert_eq!(fair.fallback, Some(Fallback::BasisWarming));
            }
        }
        let fair = mm.fair(&market.view()).unwrap();
        assert_eq!(
            (fair.price, fair.source, fair.fallback),
            (dec("100"), FairValue::Binance, None)
        );
        // Another symbol's prices are ignored.
        let other = reference("ETHUSDT", dec("1"), dec("2"), market.now);
        mm.on_signal(&Signal::Reference(other), &market.view());
        assert_eq!(mm.fair(&market.view()).unwrap().price, dec("100"));
    }
}
