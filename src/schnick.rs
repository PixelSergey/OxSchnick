pub type Symbol = usize;

#[derive(Debug, Clone)]
pub struct OngoingSchnick {
    pub inviter: i32,
    pub invitee: i32,
    pub inviter_selection: Option<Symbol>,
    pub invitee_selection: Option<Symbol>,
}