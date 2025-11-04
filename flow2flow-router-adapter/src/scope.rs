use std::fmt;

use flow2flow_contract::Scope;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScopeKey {
    pub tenant: Option<String>,
    pub team: Option<String>,
    pub user: Option<String>,
}

impl ScopeKey {
    pub fn global() -> Self {
        Self { tenant: None, team: None, user: None }
    }

    pub fn from_scope(scope: &Scope) -> Self {
        Self {
            tenant: Some(scope.tenant.clone()),
            team: scope.team.clone(),
            user: scope.user.clone(),
        }
    }

    pub fn with_parts(tenant: Option<String>, team: Option<String>, user: Option<String>) -> Self {
        Self { tenant, team, user }
    }

    pub fn path_prefix(&self) -> String {
        match (&self.tenant, &self.team, &self.user) {
            (Some(tenant), Some(team), Some(user)) => {
                format!("/tenants/{tenant}/teams/{team}/users/{user}")
            }
            (Some(tenant), Some(team), None) => format!("/tenants/{tenant}/teams/{team}"),
            (Some(tenant), None, Some(user)) => format!("/tenants/{tenant}/users/{user}"),
            (Some(tenant), None, None) => format!("/tenants/{tenant}"),
            (None, _, _) => "/global".to_string(),
        }
    }
}

impl fmt::Display for ScopeKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.tenant, &self.team, &self.user) {
            (Some(tenant), Some(team), Some(user)) => {
                write!(f, "{tenant}/{team}/{user}")
            }
            (Some(tenant), Some(team), None) => write!(f, "{tenant}/{team}"),
            (Some(tenant), None, Some(user)) => write!(f, "{tenant}/_/{}", user),
            (Some(tenant), None, None) => write!(f, "{tenant}"),
            (None, _, _) => write!(f, "global"),
        }
    }
}

pub fn fallback_order(scope: &Scope) -> Vec<ScopeKey> {
    let mut order = Vec::new();

    let tenant = scope.tenant.clone();
    let team = scope.team.clone();
    let user = scope.user.clone();

    if let (Some(team), Some(user)) = (team.clone(), user.clone()) {
        order.push(ScopeKey::with_parts(Some(tenant.clone()), Some(team), Some(user)));
    }

    if let Some(team) = team.clone() {
        order.push(ScopeKey::with_parts(Some(tenant.clone()), Some(team), None));
    }

    if let Some(user) = user.clone() {
        order.push(ScopeKey::with_parts(Some(tenant.clone()), None, Some(user)));
    }

    order.push(ScopeKey::with_parts(Some(tenant), None, None));
    order.push(ScopeKey::global());

    dedup(order)
}

fn dedup(scopes: Vec<ScopeKey>) -> Vec<ScopeKey> {
    let mut unique = Vec::new();
    for scope in scopes.into_iter() {
        if !unique.contains(&scope) {
            unique.push(scope);
        }
    }
    unique
}
