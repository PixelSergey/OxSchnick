use std::fmt::Debug;

use anyhow::anyhow;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::error::{Error, Result};
use crate::schema::{metrics, users};

pub const METRICS_LEADERBOARD_LENGTH: i64 = 10;

#[derive(Debug, Clone, HasQuery, Identifiable, QueryableByName)]
#[diesel(table_name=crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct MetricsUser {
    pub id: i32,
    pub username: String,
}

pub struct Metrics {
    num_children: Vec<(MetricsUser, i32)>,
}

impl Metrics {
    pub async fn new(conn: &mut AsyncPgConnection) -> anyhow::Result<Self> {
        let mut metrics = Self {
            num_children: vec![] 
        };
        metrics.update(conn).await.map_err(|_| anyhow!("could not get initial metrics"))?;
        Ok(metrics)
    }

    async fn get_num_children(&mut self, conn: &mut AsyncPgConnection) -> Result<Vec<(MetricsUser, i32)>> {
        Ok(metrics::table
            .filter(metrics::num_children.gt(0))
            .order(metrics::num_children.desc())
            .limit(METRICS_LEADERBOARD_LENGTH)
            .inner_join(users::table)
            .select(((users::id, users::username), metrics::num_children))
            .get_results::<(MetricsUser, i32)>(conn)
            .await
            .map_err(|_| Error::InternalServerError)?)
    }

    pub async fn update(&mut self, conn: &mut AsyncPgConnection) -> Result<()> {
        self.num_children = self.get_num_children(conn).await?;
        Ok(())
    }
}
