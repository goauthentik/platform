use ak_platform::log::{LevelFilter, set_log_level};
use ak_platform::storage::cfgmgr::schema::{Config as ConfigSchema, ConfigChangedType};
use ak_platform_facts::GatherOptions;
use eyre::Result;
use serde::{Deserialize, Serialize};

pub mod domain;
pub mod managed;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Config {
    pub debug: bool,
    #[serde(rename = "runtime")]
    pub runtime_dir: String,
    #[serde(rename = "domains")]
    pub domain_dir: String,
    /// Which device facts are collected and sent to authentik on check-in.
    /// Optional; everything is collected when unset.
    #[serde(default)]
    pub facts: GatherOptions,
}

impl ConfigSchema for Config {
    async fn post_load(&mut self) -> Result<()> {
        // Only ever raises, so it can't undo `--debug`.
        if self.debug {
            set_log_level(LevelFilter::Trace);
        }
        Ok(())
    }

    async fn pre_save(&mut self) -> Result<()> {
        Ok(())
    }

    async fn post_update(&self, _prev: Self) -> Result<ConfigChangedType> {
        Ok(ConfigChangedType::Generic)
    }
}

#[cfg(test)]
mod tests {
    use super::Config;
    use ak_platform_facts::ProcessCollection;

    #[test]
    fn facts_default_when_unset() {
        let cfg = serde_json::from_str::<Config>(
            r#"{"debug":false,"domains":"/etc/authentik/domains","runtime":"/var/run/authentik"}"#,
        );
        assert!(cfg.is_ok_and(|c| c.facts.processes == ProcessCollection::Full));
    }

    #[test]
    fn facts_processes() {
        for (value, expected) in [
            ("full", ProcessCollection::Full),
            ("executable", ProcessCollection::Executable),
            ("disabled", ProcessCollection::Disabled),
        ] {
            let cfg = serde_json::from_str::<Config>(&format!(
                r#"{{"debug":false,"domains":"","runtime":"","facts":{{"processes":"{value}"}}}}"#
            ));
            assert!(cfg.is_ok_and(|c| c.facts.processes == expected));
        }
    }

    #[test]
    fn facts_processes_invalid() {
        let cfg = serde_json::from_str::<Config>(
            r#"{"debug":false,"domains":"","runtime":"","facts":{"processes":"cmdline"}}"#,
        );
        assert!(cfg.is_err());
    }
}
