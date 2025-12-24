use std::fmt::Debug;

use anyhow::anyhow;
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::Integer;
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
    pub score: Vec<(MetricsUser, i32, i32, i32)>,
    pub num_schnicks: Vec<(MetricsUser, i32)>,
    pub num_children: Vec<(MetricsUser, i32)>,
}

impl Metrics {
    pub async fn new(conn: &mut AsyncPgConnection) -> anyhow::Result<Self> {
        let mut metrics = Self {
            score: vec![],
            num_schnicks: vec![],
            num_children: vec![] 
        };
        metrics.update(conn).await.map_err(|_| anyhow!("could not get initial metrics"))?;
        Ok(metrics)
    }

    async fn get_score(conn: &mut AsyncPgConnection) -> Result<Vec<(MetricsUser, i32, i32, i32)>> {
        let score = sql::<Integer>(
            "CAST((erf(((num_won - num_schnicks * 0.5) / sqrt(num_schnicks * 0.25)) / sqrt(2)) * 10) ^ 3 AS INTEGER)"
        );
        Ok(metrics::table
            .filter(metrics::num_schnicks.gt(0))
            .limit(METRICS_LEADERBOARD_LENGTH)
            .inner_join(users::table)
            .select(((users::id, users::username), metrics::num_won, metrics::num_schnicks, score))
            .get_results::<(MetricsUser, i32, i32, i32)>(conn)
            .await
            .map_err(|_| Error::InternalServerError)?)
    }

    async fn get_num_schnicks(conn: &mut AsyncPgConnection) -> Result<Vec<(MetricsUser, i32)>> {
        Ok(metrics::table
            .filter(metrics::num_schnicks.gt(0))
            .order(metrics::num_schnicks.desc())
            .limit(METRICS_LEADERBOARD_LENGTH)
            .inner_join(users::table)
            .select(((users::id, users::username), metrics::num_schnicks))
            .get_results::<(MetricsUser, i32)>(conn)
            .await
            .map_err(|_| Error::InternalServerError)?)
    }

    async fn get_num_children(conn: &mut AsyncPgConnection) -> Result<Vec<(MetricsUser, i32)>> {
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
        self.score = Self::get_score(conn).await?;
        self.num_schnicks = Self::get_num_schnicks(conn).await?;
        self.num_children = Self::get_num_children(conn).await?;
        Ok(())
    }
}
