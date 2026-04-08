use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use blazesym::symbolize::source::Process;
use blazesym::symbolize::{source, Input, Symbolizer};

#[derive(Debug, Clone)]
pub struct ResolvedSymbol {
    pub name: String,              // "testing::hot_c"
    pub file: Option<String>,      // "main.rs"          ← short, for TUI display
    pub file_full: Option<String>, // "/home/.../main.rs" ← full, for detail view
    pub line: Option<u32>,
}

impl ResolvedSymbol {
    /// One-line display string for the TUI table row.
    /// "testing::hot_c  main.rs:42"
    pub fn display(&self) -> String {
        match (&self.file, self.line) {
            (Some(f), Some(l)) => format!("{}  {}:{}", self.name, f, l),
            (Some(f), None) => format!("{}  {}", self.name, f),
            _ => self.name.clone(),
        }
    }
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
                .context("failed to symbolize address")?;

            let resolved = match syms.first().and_then(|e| e.as_sym()) {
                Some(sym) => {
                    let (file, file_full, line) = match &sym.code_info {
                        Some(info) => {
                            let full = info.to_path().display().to_string();
                            // Short name: just filename, not full path
                            let short = Path::new(&full)
                                .file_name()
                                .map(|f| f.to_string_lossy().into_owned())
                                .unwrap_or_else(|| full.clone());
                            (Some(short), Some(full), info.line)
                        }
                        None => (None, None, None),
                    };
                    ResolvedSymbol {
                        name: sym.name.to_string(),
                        file,
                        file_full,
                        line: line.map(|l| l as u32),
                    }
                }
                None => ResolvedSymbol {
                    name: format!("0x{addr:016x}"),
                    file: None,
                    file_full: None,
                    line: None,
                },
            };

            self.cache.insert(addr, resolved);
        }

        Ok(self.cache.get(&addr).expect("just inserted"))
    }
}
