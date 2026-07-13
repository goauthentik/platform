use std::collections::HashMap;

use authentik_client::apis::configuration::Configuration;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use crate::executor::{FlowError, FlowExecutor, HEADER_AUTHENTIK_REMOTE_IP, Solver};

#[derive(Default)]
pub struct FlowExecutorBuilder {
    flow_slug: Option<String>,
    ref_config: Option<Configuration>,
    solvers: Vec<Box<dyn Solver>>,
    pub(crate) answers: HashMap<String, String>,
    pub(crate) headers: HeaderMap,

    pending_err: Option<FlowError>,
}

impl FlowExecutorBuilder {
    pub fn flow<T: ToString>(mut self, slug: T) -> Self {
        self.flow_slug = Some(slug.to_string());
        self
    }
    pub fn base_url<T: ToString>(mut self, url: T) -> Self {
        self.ref_config = Some(Configuration {
            base_path: url.to_string(),
            ..Default::default()
        });
        self
    }
    pub fn reference_config(mut self, config: Configuration) -> Self {
        self.ref_config = Some(config);
        self
    }
    pub fn with_answer<T: ToString>(mut self, component: T, answer: T) -> Self {
        self.answers
            .insert(component.to_string(), answer.to_string());
        self
    }
    pub fn with_solver<S: Solver + 'static>(mut self, solver: S) -> Self {
        self.solvers.push(Box::new(solver));
        self
    }
    pub fn with_delegated_client_ip(mut self, ip: String) -> Self {
        let value = match HeaderValue::from_str(&ip).map_err(|e| FlowError::Other(eyre::eyre!(e))) {
            Ok(v) => v,
            Err(e) => {
                self.pending_err = Some(e);
                return self;
            }
        };
        self.headers
            .insert(HeaderName::from_static(HEADER_AUTHENTIK_REMOTE_IP), value);
        self
    }

    pub async fn build(self) -> Result<FlowExecutor, FlowError> {
        if let Some(err) = self.pending_err {
            return Err(err);
        }
        let Some(flow_slug) = self.flow_slug else {
            return Err(FlowError::Other(eyre::eyre!("Missing flow slug")));
        };
        let Some(ref_config) = self.ref_config else {
            return Err(FlowError::Other(eyre::eyre!("Missing reference config")));
        };
        let mut fe = FlowExecutor::new(flow_slug, ref_config, self.headers).await?;
        fe.answers = self.answers;
        if !self.solvers.is_empty() {
            fe.solvers = self.solvers;
        }
        Ok(fe)
    }
}
