use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Arc};

use axum::extract::FromRequestParts;
use diesel::{
    dsl::{exists, select}, prelude::*
};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use log::error;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use tokio::sync::{RwLock, mpsc, oneshot, watch};

use crate::{
    auth::{AuthenticationRequest, Authenticator, AuthenticatorEntry}, error::{Error, Result}, graphs::{GraphRequest, GraphUpdate, Graphs}, metrics::Metrics, state::State
};

const SCHNICKS_CHANNEL_BUFFER: usize = 128usize;

#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq)]
#[repr(u8)]
pub enum Weapon {
    Rock = 0,
    Scissors = 1,
    Paper = 2,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Interaction {
    pub won: bool,
    pub weapon: Weapon,
}

impl Interaction {
    fn complementary(&self) -> Self {
        Self {
            won: !self.won,
            weapon: match (self.weapon, self.won) {
                (Weapon::Rock, true) => Weapon::Scissors,
                (Weapon::Paper, true) => Weapon::Rock,
                (Weapon::Scissors, true) => Weapon::Paper,
                (Weapon::Rock, false) => Weapon::Paper,
                (Weapon::Paper, false) => Weapon::Scissors,
                (Weapon::Scissors, false) => Weapon::Rock,
            },
        }
    }

    pub fn compatible(&self, other: &Self) -> bool {
        &self.complementary() == other
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Outcome {
    Concluded,
    Retry,
    Aborted
}

pub struct Schnicker {
    connection: AsyncPgConnection,
    active: HashMap<
        i32,
        (
            Rc<RefCell<Option<(i32, Interaction, watch::Sender<Outcome>)>>>,
            i32,
        ),
    >,
    sender: mpsc::Sender<SchnickRequest>,
    receiver: mpsc::Receiver<SchnickRequest>,
    auth: mpsc::Sender<AuthenticationRequest>,
    graphs: mpsc::Sender<GraphRequest>,
    metrics: Arc<RwLock<Metrics>>
}

#[derive(Debug)]
pub enum SchnickRequest {
    StartSchnick {
        id: i32,
        opponent: i32,
        callback: oneshot::Sender<Result<()>>,
    },
    GetOutcomeReceiver {
        id: i32,
        callback: oneshot::Sender<Result<watch::Receiver<Outcome>>>,
    },
    HandleInteraction {
        id: i32,
        interaction: Interaction,
        callback: oneshot::Sender<Result<Option<Outcome>>>,
    },
    InSchnick {
        id: i32,
        callback: oneshot::Sender<Result<bool>>,
    },
    AbortSchnick {
        id: i32,
        callback: oneshot::Sender<Result<()>>,
    },
}

#[derive(Debug, Clone, Insertable, HasQuery)]
#[diesel(table_name=crate::schema::schnicks)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct SavedSchnick {
    pub winner: i32,
    pub loser: i32,
    pub weapon: i32,
}

impl Schnicker {
    pub fn with_connection_graphs_metrics_and_auth(
        connection: AsyncPgConnection,
        graphs: mpsc::Sender<GraphRequest>,
        metrics: Arc<RwLock<Metrics>>,
        auth: mpsc::Sender<AuthenticationRequest>
    ) -> Self {
        let (tx, rx) = mpsc::channel(SCHNICKS_CHANNEL_BUFFER);
        Self {
            connection,
            active: Default::default(),
            sender: tx,
            receiver: rx,
            auth,
            graphs,
            metrics
        }
    }

    pub async fn worker(mut self) {
        while let Some(request) = self.receiver.recv().await {
            match request {
                SchnickRequest::StartSchnick {
                    id,
                    opponent,
                    callback,
                } => {
                    let response = self.start_schnick(id, opponent).await;
                    if let Err(_) = callback.send(response) {
                        error!(target: "schnicks::worker", "dead receiver");
                    }
                }
                SchnickRequest::GetOutcomeReceiver { id, callback } => {
                    let response = self.get_outcome_receiver(id).await;
                    if let Err(_) = callback.send(response) {
                        error!(target: "schnicks::worker", "dead receiver");
                    }
                }
                SchnickRequest::HandleInteraction {
                    id,
                    interaction,
                    callback,
                } => {
                    let response = self.handle_interaction(id, &interaction).await;
                    if let Err(_) = callback.send(response) {
                        error!(target: "schnicks::worker", "dead receiver");
                    }
                }
                SchnickRequest::InSchnick { id, callback } => {
                    let response = self.in_schnick(id).await;
                    if let Err(_) = callback.send(response) {
                        error!(target: "schnicks::worker", "dead receiver");
                    }
                }
                SchnickRequest::AbortSchnick { id, callback } => {
                    let response = self.abort_schnick(id).await;
                    if let Err(_) = callback.send(response) {
                        error!(target: "schnicks::worker", "dead receiver");
                    }
                }
            }
        }
    }

    fn saved_schnick(
        old_id: i32,
        old_interaction: &Interaction,
        id: i32,
        interaction: &Interaction,
    ) -> Option<SavedSchnick> {
        if old_interaction.compatible(interaction) {
            let (winner, loser, weapon) = if old_interaction.won {
                (old_id, id, old_interaction.weapon)
            } else {
                (id, old_id, interaction.weapon)
            };
            Some(SavedSchnick {
                winner,
                loser,
                weapon: weapon as i32,
            })
        } else {
            None
        }
    }

    async fn start_schnick(&mut self, id: i32, opponent: i32) -> Result<()> {
        use crate::schema::schnicks;
        if id == opponent {
            return Err(Error::CannotSchnickOneself);
        }
        let already_schnicked: bool = select(exists(
            schnicks::table.filter(
                (schnicks::winner.eq(id).and(schnicks::loser.eq(opponent)))
                    .or(schnicks::loser.eq(id).and(schnicks::winner.eq(opponent))),
            ),
        ))
        .get_result(&mut self.connection)
        .await
        .map_err(|e| {
            error!(target: "schnicks::start_schnick", "{:?}", e);
            Error::InternalServerError
        })?;
        if already_schnicked {
            return Err(Error::CannotSchnickTwice);
        }
        if self.active.contains_key(&id) || self.active.contains_key(&opponent) {
            return Err(Error::AlreadySchnicking);
        }
        let new = Default::default();
        self.active.insert(id, (Rc::clone(&new), opponent));
        self.active.insert(opponent, (new, id));
        Ok(())
    }

    async fn get_outcome_receiver(&self, id: i32) -> Result<watch::Receiver<Outcome>> {
        let (entry, _) = self.active.get(&id).ok_or(Error::NotInSchnick)?;
        let (old_id, _, sender) = entry.borrow().clone().ok_or(Error::NotFound)?;
        if old_id == id {
            Ok(sender.subscribe())
        } else {
            Err(Error::NotFound)
        }
    }

    async fn handle_interaction(
        &mut self,
        id: i32,
        interaction: &Interaction,
    ) -> Result<Option<Outcome>> {
        use crate::schema::schnicks;
        let active = Rc::clone(&self.active.get(&id).ok_or(Error::NotInSchnick)?.0)
            .borrow()
            .clone();
        if let Some((old_id, old_interaction, sender)) = active {
            if id == old_id {
                return Err(Error::AlreadySubmitted);
            }
            if let Some(saved) = Self::saved_schnick(old_id, &old_interaction, id, interaction) {
                saved
                    .insert_into(schnicks::table)
                    .execute(&mut self.connection)
                    .await
                    .map_err(|e| {
                        error!(target: "schnicks::handle_interaction", "{:?}", e);
                        Error::InternalServerError
                    })?;
                sender.send_replace(Outcome::Concluded);
                Graphs::send_update(GraphUpdate::Schnick { a: old_id, b: id }, &self.graphs).await;
                Authenticator::request_create_invite_if_not_exists(id, &self.auth).await?;
                Authenticator::request_create_invite_if_not_exists(old_id, &self.auth).await?;
                self.metrics.write().await.update(&mut self.connection).await?;
                self.active.remove(&id);
                self.active.remove(&old_id);
                Ok(Some(Outcome::Concluded))
            } else {
                let _ = self
                    .active
                    .get(&id)
                    .ok_or(Error::InternalServerError)?
                    .0
                    .borrow_mut()
                    .take();
                sender.send_replace(Outcome::Retry);
                Ok(Some(Outcome::Retry))
            }
        } else {
            let (tx, _) = watch::channel(Outcome::Retry);
            self.active
                .get(&id)
                .ok_or(Error::InternalServerError)?
                .0
                .borrow_mut()
                .replace((id, interaction.clone(), tx));
            Ok(None)
        }
    }

    async fn in_schnick(&self, id: i32) -> Result<bool> {
        if let Some((entry, _)) = self.active.get(&id) {
            if let Some((old_id, _, _)) = *entry.borrow() {
                Ok(id != old_id)
            } else {
                Ok(true)
            }
        } else {
            Err(Error::NotInSchnick)
        }
    }

    async fn abort_schnick(&mut self, id: i32) -> Result<()> {
        let (_, opponent) = self.active.remove(&id).ok_or(Error::NotInSchnick)?;
        let (active, _) = self
            .active
            .remove(&opponent)
            .ok_or(Error::InternalServerError)?;
        if let Some((_, _, sender)) = active.borrow().clone() {
            sender.send_replace(Outcome::Aborted);
        }
        Ok(())
    }

    pub async fn request_start_schnick(
        id: i32,
        opponent: i32,
        sender: &mpsc::Sender<SchnickRequest>,
    ) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        sender
            .send(SchnickRequest::StartSchnick {
                id,
                opponent,
                callback: tx,
            })
            .await
            .map_err(|e| {
                error!(target: "schnicks::start_schnick", "dead channel: {:?}", e);
                Error::InternalServerError
            })?;
        rx.await.map_err(|e| {
            error!(target: "schnicks::start_schnick", "dead channel: {:?}", e);
            Error::InternalServerError
        })?
    }

    pub async fn request_get_outcome_receiver(
        id: i32,
        sender: &mpsc::Sender<SchnickRequest>,
    ) -> Result<watch::Receiver<Outcome>> {
        let (tx, rx) = oneshot::channel();
        sender
            .send(SchnickRequest::GetOutcomeReceiver { id, callback: tx })
            .await
            .map_err(|e| {
                error!(target: "schnicks::get_outcome_receiver", "dead channel: {:?}", e);
                Error::InternalServerError
            })?;
        rx.await.map_err(|e| {
            error!(target: "schnicks::get_outcome_receiver", "dead channel: {:?}", e);
            Error::InternalServerError
        })?
    }

    pub async fn request_handle_interaction(
        id: i32,
        interaction: Interaction,
        sender: &mpsc::Sender<SchnickRequest>,
    ) -> Result<Option<Outcome>> {
        let (tx, rx) = oneshot::channel();
        sender
            .send(SchnickRequest::HandleInteraction {
                id,
                interaction,
                callback: tx,
            })
            .await
            .map_err(|e| {
                error!(target: "schnicks::handle_interaction", "dead channel: {:?}", e);
                Error::InternalServerError
            })?;
        rx.await.map_err(|e| {
            error!(target: "schnicks::handle_interaction", "dead channel: {:?}", e);
            Error::InternalServerError
        })?
    }

    pub async fn request_in_schnick(
        id: i32,
        sender: &mpsc::Sender<SchnickRequest>,
    ) -> Result<bool> {
        let (tx, rx) = oneshot::channel();
        sender
            .send(SchnickRequest::InSchnick {
                id: id,
                callback: tx,
            })
            .await
            .map_err(|e| {
                error!(target: "schnicks::request_in_schnick", "dead channel: {:?}", e);
                Error::InternalServerError
            })?;
        rx.await.map_err(|e| {
            error!(target: "schnicks::request_in_schnick", "dead channel: {:?}", e);
            Error::InternalServerError
        })?
    }

    pub async fn request_abort_schnick(
        id: i32,
        sender: &mpsc::Sender<SchnickRequest>,
    ) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        sender
            .send(SchnickRequest::AbortSchnick {
                id: id,
                callback: tx,
            })
            .await
            .map_err(|e| {
                error!(target: "schnicks::request_in_schnick", "dead channel: {:?}", e);
                Error::InternalServerError
            })?;
        rx.await.map_err(|e| {
            error!(target: "schnicks::request_in_schnick", "dead channel: {:?}", e);
            Error::InternalServerError
        })?
    }

    pub fn sender(&self) -> mpsc::Sender<SchnickRequest> {
        self.sender.clone()
    }
}

#[derive(Debug, Clone)]
pub struct SchnickOutcomeReceiver(pub watch::Receiver<Outcome>);

impl FromRequestParts<State> for SchnickOutcomeReceiver {
    type Rejection = Error;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &State,
    ) -> Result<Self> {
        let (id, _) = parts
            .extensions
            .get::<(i32, AuthenticatorEntry)>()
            .ok_or(Error::NoLogin)?;
        let receiver = Schnicker::request_get_outcome_receiver(*id, &state.schnicker).await?;
        Ok(Self(receiver))
    }
}
