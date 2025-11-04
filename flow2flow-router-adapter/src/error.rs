use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdapterError {
    #[error("flow spec `{flow_id}` missing version")]
    MissingVersion { flow_id: String },
    #[error("flow spec `{flow_id}` missing inbound node")]
    MissingInNode { flow_id: String },
    #[error("flow spec `{flow_id}` missing outbound node")]
    MissingOutNode { flow_id: String },
    #[error("invalid version requirement `{requirement}`: {source}")]
    VersionReq {
        requirement: String,
        #[source]
        source: semver::Error,
    },
    #[error("registry error: {0}")]
    Registry(String),
}

impl AdapterError {
    pub fn registry(message: impl Into<String>) -> Self {
        Self::Registry(message.into())
    }
}
