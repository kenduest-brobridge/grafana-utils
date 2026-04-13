use crate::common::Result;
use crate::http::JsonHttpClient;

use super::{
    AccessResourceClient, AlertingResourceClient, DashboardResourceClient,
    DatasourceResourceClient, GrafanaConnection,
};

#[derive(Clone)]
pub(crate) struct GrafanaApiClient {
    connection: GrafanaConnection,
    http: JsonHttpClient,
}

impl GrafanaApiClient {
    pub(crate) fn from_connection(connection: GrafanaConnection) -> Result<Self> {
        let http = connection.build_http_client()?;
        Ok(Self { connection, http })
    }

    #[allow(dead_code)]
    pub(crate) fn connection(&self) -> &GrafanaConnection {
        &self.connection
    }

    #[allow(dead_code)]
    pub(crate) fn http_client(&self) -> &JsonHttpClient {
        &self.http
    }

    pub(crate) fn into_http_client(self) -> JsonHttpClient {
        self.http
    }

    pub(crate) fn scoped_to_org(&self, org_id: i64) -> Result<Self> {
        Self::from_connection(self.connection.with_org_id(org_id))
    }

    #[allow(dead_code)]
    pub(crate) fn dashboard(&self) -> DashboardResourceClient<'_> {
        DashboardResourceClient::new(&self.http)
    }

    #[allow(dead_code)]
    pub(crate) fn datasource(&self) -> DatasourceResourceClient<'_> {
        DatasourceResourceClient::new(&self.http)
    }

    pub(crate) fn alerting(&self) -> AlertingResourceClient<'_> {
        AlertingResourceClient::new(&self.http)
    }

    #[allow(dead_code)]
    pub(crate) fn access(&self) -> AccessResourceClient<'_> {
        AccessResourceClient::new(&self.http)
    }
}
