use async_trait::async_trait;
use hedera_proto::services;
use hedera_proto::services::schedule_service_client::ScheduleServiceClient;
use tonic::transport::Channel;

use crate::query::{
    AnyQueryData,
    QueryExecute,
    ToQueryProtobuf,
};
use crate::{
    Query,
    ScheduleId,
    ScheduleInfo,
    ToProtobuf,
};

/// Get all the information about a schedule.
pub type ScheduleInfoQuery = Query<ScheduleInfoQueryData>;

#[derive(Default, Clone, serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleInfoQueryData {
    schedule_id: Option<ScheduleId>,
}

impl From<ScheduleInfoQueryData> for AnyQueryData {
    #[inline]
    fn from(data: ScheduleInfoQueryData) -> Self {
        Self::ScheduleInfo(data)
    }
}

impl ScheduleInfoQuery {
    /// Sets the schedule ID for which information is requested.
    pub fn schedule_id(&mut self, id: impl Into<ScheduleId>) -> &mut Self {
        self.data.schedule_id = Some(id.into());
        self
    }
}

impl ToQueryProtobuf for ScheduleInfoQueryData {
    fn to_query_protobuf(&self, header: services::QueryHeader) -> services::Query {
        let schedule_id = self.schedule_id.as_ref().map(|id| id.to_protobuf());

        services::Query {
            query: Some(services::query::Query::ScheduleGetInfo(services::ScheduleGetInfoQuery {
                schedule_id,
                header: Some(header),
            })),
        }
    }
}

#[async_trait]
impl QueryExecute for ScheduleInfoQueryData {
    type Response = ScheduleInfo;

    async fn execute(
        &self,
        channel: Channel,
        request: services::Query,
    ) -> Result<tonic::Response<services::Response>, tonic::Status> {
        ScheduleServiceClient::new(channel).get_schedule_info(request).await
    }
}