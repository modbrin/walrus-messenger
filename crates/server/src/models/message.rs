use crate::models::resource::ResourceId;

pub type MessageId = i64;

#[derive(Clone, Debug)]
pub struct Message {
    pub id: MessageId,
    pub text: String,
    pub reply_to: Option<MessageId>,
    pub resource_id: Option<ResourceId>,
}
