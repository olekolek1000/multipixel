pub struct SessionID(pub u16);

pub struct Session {
    pub id: SessionID,
    pub nickname: String, // Max 255 characters
}
