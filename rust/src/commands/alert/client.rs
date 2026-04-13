//! Minimal Grafana alerting API client.
//! Wraps typed request wrappers for list/import/export flows and hides raw HTTP plumbing from CLI handlers.
use serde_json::{Map, Value};

use crate::common::Result;
use crate::grafana_api::{AlertingResourceClient, GrafanaApiClient, GrafanaConnection};

use super::AlertAuthContext;

/// Struct definition for GrafanaAlertClient.
pub struct GrafanaAlertClient {
    api: GrafanaApiClient,
}

impl GrafanaAlertClient {
    pub fn new(context: &AlertAuthContext) -> Result<Self> {
        Ok(Self {
            api: GrafanaApiClient::from_connection(GrafanaConnection::new(
                context.url.clone(),
                context.headers.clone(),
                context.timeout,
                context.verify_ssl,
                None,
                "unknown".to_string(),
            ))?,
        })
    }

    fn alerting(&self) -> AlertingResourceClient<'_> {
        self.api.alerting()
    }

    pub fn list_alert_rules(&self) -> Result<Vec<Map<String, Value>>> {
        self.alerting().list_alert_rules()
    }

    pub fn list_orgs(&self) -> Result<Vec<Map<String, Value>>> {
        self.alerting().list_orgs()
    }

    /// Search dashboards by query.
    pub fn search_dashboards(&self, query: &str) -> Result<Vec<Map<String, Value>>> {
        self.alerting().search_dashboards(query)
    }

    pub fn get_dashboard(&self, uid: &str) -> Result<Map<String, Value>> {
        self.alerting().get_dashboard(uid)
    }

    pub fn get_alert_rule(&self, uid: &str) -> Result<Map<String, Value>> {
        self.alerting().get_alert_rule(uid)
    }

    pub fn create_alert_rule(&self, payload: &Map<String, Value>) -> Result<Map<String, Value>> {
        self.alerting().create_alert_rule(payload)
    }

    pub fn update_alert_rule(
        &self,
        uid: &str,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        self.alerting().update_alert_rule(uid, payload)
    }

    pub fn list_contact_points(&self) -> Result<Vec<Map<String, Value>>> {
        self.alerting().list_contact_points()
    }

    pub fn create_contact_point(&self, payload: &Map<String, Value>) -> Result<Map<String, Value>> {
        self.alerting().create_contact_point(payload)
    }

    pub fn update_contact_point(
        &self,
        uid: &str,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        self.alerting().update_contact_point(uid, payload)
    }

    pub fn list_mute_timings(&self) -> Result<Vec<Map<String, Value>>> {
        self.alerting().list_mute_timings()
    }

    pub fn create_mute_timing(&self, payload: &Map<String, Value>) -> Result<Map<String, Value>> {
        self.alerting().create_mute_timing(payload)
    }

    pub fn update_mute_timing(
        &self,
        name: &str,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        self.alerting().update_mute_timing(name, payload)
    }

    pub fn get_notification_policies(&self) -> Result<Map<String, Value>> {
        self.alerting().get_notification_policies()
    }

    pub fn update_notification_policies(
        &self,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        self.alerting().update_notification_policies(payload)
    }

    pub fn list_templates(&self) -> Result<Vec<Map<String, Value>>> {
        self.alerting().list_templates()
    }

    pub fn get_template(&self, name: &str) -> Result<Map<String, Value>> {
        self.alerting().get_template(name)
    }

    pub fn update_template(
        &self,
        name: &str,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        self.alerting().update_template(name, payload)
    }
}
