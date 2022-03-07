




use serde::{Serialize, Deserialize};
use mongodb::bson::{oid::ObjectId};


// NOTE - a mongodb document has been serialized into the BSON format before inserting into the collection



#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Voter{
    pub event_owner_wallet_address: String,
    pub is_upvote: bool,
    pub score: u32, // NOTE - this is the number of NFTs that this owner owns
}



#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CastVoteRequest{
    pub _id: String, //-- this is the id of the event took from the mongodb
    pub voter: Voter,
}



#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EventAddRequest{
    pub title: String,
    pub content: String,
    pub creator_wallet_address: String,
    pub upvotes: Option<u16>, // NOTE - we set this field to Option cause we don't want to pass the upvotes inside the request body, we'll fill it inside the server
    pub downvotes: Option<u16>, // NOTE - we set this field to Option cause we don't want to pass the downvotes inside the request body, we'll fill it inside the server
    pub voters: Option<Vec<Voter>>, // NOTE - we set this field to Option cause we don't want to pass the voters inside the request body, we'll update it later on using cast-vote route
    pub is_expired: Option<bool>, // NOTE - we set this field to Option cause we don't want to pass the is_expired inside the request body, we'll update it once a event reached the deadline
    pub expire_at: Option<i64>, // NOTE - we set this field to Option cause we don't want to pass the expire_at inside the request body, we'll update it while we want to create a new event object
    pub created_at: Option<i64>, // NOTE - we set this field to Option cause we don't want to pass the created time inside the request body, we'll fill it inside the server
}



#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EventInfo{
    pub _id: Option<ObjectId>,
    pub title: String,
    pub content: String,
    pub upvotes: Option<u16>,
    pub downvotes: Option<u16>,
    pub voters: Option<Vec<Voter>>,
    pub is_expired: Option<bool>,
    pub expire_at: Option<i64>,
    pub created_at: Option<i64>,
}



#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AvailableEvents{
    pub events: Vec<EventInfo>,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExpireEventRequest{
    pub _id: String, //-- this is the id of the event took from the mongodb
}


impl EventInfo{

    pub async fn add_voter(self, voter: Voter) -> Vec<Voter>{ //-- we don't take a reference to self cause we can't dereference a shared reference and if we do that then cannot borrow `*voters` as mutable, cause it is behind a `&` reference and `voters` is a `&` reference, so the data it refers to cannot be borrowed as mutable cause we have to define the first argument as &mut self
        let mut voters = self.voters.unwrap();
        let index = voters.iter().position(|v| v.event_owner_wallet_address == voter.event_owner_wallet_address); //-- this owner has alreay voted to this event
        if index == None{
            voters.push(voter);
        }
        voters
    }
}