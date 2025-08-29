use std::collections::HashMap;

pub struct CoinSymbolReserved {
    reserved_list: HashMap<&'static str, &'static str>,
}

impl CoinSymbolReserved {
    pub fn new() -> Self {
        let mut reserved_list = HashMap::new();

        reserved_list.insert("BTC", "SYSTEM");
        reserved_list.insert("ETH", "SYSTEM");
        reserved_list.insert("USDT", "SYSTEM");
        reserved_list.insert("XRP", "SYSTEM");
        reserved_list.insert("BNB", "SYSTEM");
        reserved_list.insert("SOL", "SYSTEM");
        reserved_list.insert("USDC", "SYSTEM");
        reserved_list.insert("DOGE", "SYSTEM");
        reserved_list.insert("ADA", "SYSTEM");
        reserved_list.insert("TRX", "SYSTEM");
        reserved_list.insert("DOT", "SYSTEM");
        reserved_list.insert("MATIC", "SYSTEM");
        reserved_list.insert("SHIB", "SYSTEM");
        reserved_list.insert("LTC", "SYSTEM");
        reserved_list.insert("BCH", "SYSTEM");
        reserved_list.insert("AVAX", "SYSTEM");
        reserved_list.insert("LINK", "SYSTEM");
        reserved_list.insert("XLM", "SYSTEM");
        reserved_list.insert("ATOM", "SYSTEM");
        reserved_list.insert("UNI", "SYSTEM");
        reserved_list.insert("ETC", "SYSTEM");
        reserved_list.insert("XMR", "SYSTEM");
        reserved_list.insert("TON", "SYSTEM");
        reserved_list.insert("OKB", "SYSTEM");
        reserved_list.insert("LDO", "SYSTEM");
        reserved_list.insert("ICP", "SYSTEM");
        reserved_list.insert("APT", "SYSTEM");
        reserved_list.insert("FIL", "SYSTEM");
        reserved_list.insert("ARB", "SYSTEM");
        reserved_list.insert("CRO", "SYSTEM");
        reserved_list.insert("QNT", "SYSTEM");
        reserved_list.insert("NEAR", "SYSTEM");
        reserved_list.insert("VET", "SYSTEM");
        reserved_list.insert("OP", "SYSTEM");
        reserved_list.insert("MKR", "SYSTEM");
        reserved_list.insert("XDC", "SYSTEM");
        reserved_list.insert("AAVE", "SYSTEM");
        reserved_list.insert("GRT", "SYSTEM");
        reserved_list.insert("ALGO", "SYSTEM");
        reserved_list.insert("SAND", "SYSTEM");
        reserved_list.insert("STX", "SYSTEM");
        reserved_list.insert("EGLD", "SYSTEM");
        reserved_list.insert("IMX", "SYSTEM");
        reserved_list.insert("THETA", "SYSTEM");
        reserved_list.insert("AXS", "SYSTEM");
        reserved_list.insert("XTZ", "SYSTEM");
        reserved_list.insert("MANA", "SYSTEM");
        reserved_list.insert("FLOW", "SYSTEM");
        reserved_list.insert("CHZ", "SYSTEM");
        reserved_list.insert("EOS", "SYSTEM");
        reserved_list.insert("NEO", "SYSTEM");
        reserved_list.insert("KAVA", "SYSTEM");
        reserved_list.insert("SNX", "SYSTEM");
        reserved_list.insert("CAKE", "SYSTEM");
        reserved_list.insert("ZIL", "SYSTEM");
        reserved_list.insert("CRV", "SYSTEM");
        reserved_list.insert("ENJ", "SYSTEM");
        reserved_list.insert("1INCH", "SYSTEM");
        reserved_list.insert("BAT", "SYSTEM");
        reserved_list.insert("GALA", "SYSTEM");
        reserved_list.insert("DYDX", "SYSTEM");
        reserved_list.insert("FTM", "SYSTEM");
        reserved_list.insert("RUNE", "SYSTEM");
        reserved_list.insert("ZRX", "SYSTEM");
        reserved_list.insert("CELO", "SYSTEM");
        reserved_list.insert("HOT", "SYSTEM");
        reserved_list.insert("WAVES", "SYSTEM");
        reserved_list.insert("KSM", "SYSTEM");
        reserved_list.insert("COMP", "SYSTEM");
        reserved_list.insert("LRC", "SYSTEM");
        reserved_list.insert("OMG", "SYSTEM");
        reserved_list.insert("ICX", "SYSTEM");
        reserved_list.insert("ONT", "SYSTEM");
        reserved_list.insert("QTUM", "SYSTEM");
        reserved_list.insert("ZEN", "SYSTEM");
        reserved_list.insert("XEM", "SYSTEM");
        reserved_list.insert("ANKR", "SYSTEM");
        reserved_list.insert("KNC", "SYSTEM");
        reserved_list.insert("SC", "SYSTEM");
        reserved_list.insert("STORJ", "SYSTEM");
        reserved_list.insert("DASH", "SYSTEM");
        reserved_list.insert("BAND", "SYSTEM");
        reserved_list.insert("RVN", "SYSTEM");
        reserved_list.insert("CVC", "SYSTEM");
        reserved_list.insert("NEXO", "SYSTEM");
        reserved_list.insert("SXP", "SYSTEM");
        reserved_list.insert("HNT", "SYSTEM");
        reserved_list.insert("DCR", "SYSTEM");
        reserved_list.insert("AR", "SYSTEM");
        reserved_list.insert("XYM", "SYSTEM");
        reserved_list.insert("BAL", "SYSTEM");
        reserved_list.insert("FLUX", "SYSTEM");
        reserved_list.insert("GLM", "SYSTEM");
        reserved_list.insert("CELR", "SYSTEM");
        reserved_list.insert("IOST", "SYSTEM");
        reserved_list.insert("CHSB", "SYSTEM");
        reserved_list.insert("FET", "SYSTEM");
        reserved_list.insert("RAY", "SYSTEM");
        reserved_list.insert("BNT", "SYSTEM");
        reserved_list.insert("SRM", "SYSTEM");
        reserved_list.insert("REN", "SYSTEM");
        reserved_list.insert("OCEAN", "SYSTEM");
        reserved_list.insert("REEF", "SYSTEM");
        reserved_list.insert("COTI", "SYSTEM");

        Self { reserved_list }
    }

    pub fn is_free(&self, symbol: &str, caller: Option<&str>) -> bool {
        let upcase_symbol = symbol.to_uppercase();

        // If caller owns this exact reserved symbol, allow it
        if let Some(c) = caller {
            if let Some(owner) = self.reserved_list.get(upcase_symbol.as_str()) {
                if owner.to_uppercase() == c.to_uppercase() {
                    return true;
                }
            }
        }

        // If symbol starts with "AMA", it's always reserved
        if upcase_symbol.starts_with("AMA") {
            return false;
        }

        // If not in reserved list, it's free
        if !self.reserved_list.contains_key(upcase_symbol.as_str()) {
            return true;
        }

        // Otherwise reserved
        false
    }
}
