use std::collections::HashMap;

use anyhow::{Context, Result};
use blazesym::symbolize::source::Process;
use blazesym::symbolize::{source, Input, Symbolizer};

#[derive(Debug, Clone)]
pub struct ResolvedSymbol {
    pub name: String,
    pub module: Option<String>,
}

pub struct Symbols {
    pid: u32,
    symbolize: Symbolizer,
    cache: HashMap<u64, ResolvedSymbol>,
}

impl Symbols {
    pub fn new(pid: u32) -> Self {
        Self {
            pid,
            symbolize: Symbolizer::new(),
            cache: HashMap::new(),
        }
    }

    pub fn resolve(&mut self, addr: u64) -> Result<&ResolvedSymbol> {
        if !self.cache.contains_key(&addr) {
            let src = source::Source::Process(Process::new(self.pid.into()));

            let syms = self
                .symbolize
                .symbolize(&src, Input::AbsAddr(&[addr]))
                .context("failed to symbolize process address")?;

            let resolved = match syms.first().and_then(|entry| entry.as_sym()) {
                Some(sym) => ResolvedSymbol {
                    name: sym.name.to_string(),
                    module: sym
                        .code_info
                        .as_ref()
                        .map(|info| info.to_path().display().to_string()),
                },
                None => ResolvedSymbol {
                    name: format!("0x{addr:016x}"),
                    module: None,
                },
            };

            self.cache.insert(addr, resolved);
        }

        Ok(self.cache.get(&addr).expect("symbol must exist in cache"))
    }
}
