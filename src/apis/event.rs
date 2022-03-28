
use std::env;
use crate::contexts as ctx;
use crate::schemas;
use crate::constants::*;
use crate::utils;
use chrono::Utc;
use futures::{executor::block_on, TryFutureExt, TryStreamExt}; //-- based on orphan rule TryStreamExt trait is required to use try_next() method on the future object which is solved by .await - try_next() is used on futures stream or chunks to get the next future IO stream
use bytes::Buf; //-- based on orphan rule it'll be needed to call the reader() method on the whole_body buffer
use mongodb::{bson, bson::{doc, oid::ObjectId}};
use actix_web::{Error, HttpRequest, HttpResponse, Result, get, post, web};











#[post("/add")]
async fn add_event(req: HttpRequest, event_info: web::Json<schemas::event::EventAddRequest>) -> Result<HttpResponse, Error>{
    
    let conn = utils::db::connection().await;
    let app_storage = match conn.as_ref().unwrap().db.as_ref().unwrap().mode{ //-- here as_ref() method convert &Option<T> to Option<&T>
        ctx::app::Mode::On => conn.as_ref().as_ref().unwrap().db.as_ref().unwrap().instance.as_ref(), //-- return the db if it wasn't detached - instance.as_ref() will return the Option<&Client>
        ctx::app::Mode::Off => None, //-- no db is available cause it's off
    };

    let event_info = event_info.into_inner(); //-- into_inner() will deconstruct to an inner value and return T    
    let events = app_storage.unwrap().database("bitrader").collection::<schemas::event::EventInfo>("events"); //-- selecting events collection to fetch all event infos into the EventInfo struct
    match events.find_one(doc!{"title": event_info.clone().title}, None).unwrap(){ //-- finding event based on event title
        Some(event_doc) => { //-- deserializing BSON into the EventInfo struct
            let response_body = ctx::app::Response::<schemas::event::EventInfo>{ //-- we have to specify a generic type for data field in Response struct which in our case is EventInfo struct
                data: Some(event_doc), //-- data is an empty &[u8] array
                message: FOUND_DOCUMENT, //-- collection found in bitrader database
                status: 302,
            };
            Ok(
                HttpResponse::Found()
                .json(
                    response_body
                ).into_body()
            )
        }, 
        None => { //-- means we didn't find any document related to this title and we have to create a new event
            let events = app_storage.unwrap().database("bitrader").collection::<schemas::event::EventAddRequest>("events");
            let now = Utc::now().timestamp_nanos() / 1_000_000_000; // nano to sec
            let exp_time = now + env::var("EVENT_EXPIRATION").expect("⚠️ found no event expiration time").parse::<i64>().unwrap();
            let new_event = schemas::event::EventAddRequest{
                title: event_info.clone().title,
                content: event_info.clone().content,
                creator_wallet_address: event_info.clone().creator_wallet_address,
                upvotes: Some(0),
                downvotes: Some(0),
                voters: Some(vec![]), //-- initializing empty voters
                is_expired: Some(false), //-- a event is not expired yet or at initialization
                expire_at: Some(exp_time), //-- a event will be expired at
                created_at: Some(now),
            };
            match events.insert_one(new_event, None){
                Ok(insert_result) => {
                    let response_body = ctx::app::Response::<ObjectId>{ //-- we have to specify a generic type for data field in Response struct which in our case is ObjectId struct
                        data: Some(insert_result.inserted_id.as_object_id().unwrap()),
                        message: INSERTED,
                        status: 201,
                    };
                    Ok(
                        HttpResponse::Created().json(
                            response_body
                        ).into_body()
                    )
                },
                Err(e) => {
                    let response_body = ctx::app::Response::<ctx::app::Nill>{
                        data: Some(ctx::app::Nill(&[])), //-- data is an empty &[u8] array
                        message: &e.to_string(), //-- take a reference to the string error
                        status: 406,
                    };
                    Ok(
                        HttpResponse::NotAcceptable().json(
                            response_body
                        ).into_body()
                    )
                }
            }
        },
    }

}


#[get("/get/availables")]
async fn get_all_events(req: HttpRequest) -> Result<HttpResponse, Error>{

    let conn = utils::db::connection().await;
    let app_storage = match conn.as_ref().unwrap().db.as_ref().unwrap().mode{ //-- here as_ref() method convert &Option<T> to Option<&T>
        ctx::app::Mode::On => conn.as_ref().as_ref().unwrap().db.as_ref().unwrap().instance.as_ref(), //-- return the db if it wasn't detached - instance.as_ref() will return the Option<&Client>
        ctx::app::Mode::Off => None, //-- no db is available cause it's off
    };

    let filter = doc! { "is_expired": false }; //-- filtering all none expired events
    let events = app_storage.unwrap().database("bitrader").collection::<schemas::event::EventInfo>("events"); //-- selecting events collection to fetch and deserialize all event infos or documents from BSON into the EventInfo struct
    let mut available_events = schemas::event::AvailableEvents{
        events: vec![],
    };

    match events.find(filter, None){
        Ok(cursor) => {

            // ---------------------------------------
            // NOTE - uncomment this for async mongodb
            // ---------------------------------------
            // while let Some(event) = cursor.try_next().await.unwrap(){ //-- calling try_next() method on cursor needs the cursor to be mutable - reading while awaiting on try_next() method doesn't return None
            //     available_events.events.push(event);
            // }
            
            for event in cursor {
                available_events.events.push(event.unwrap());
            }
            let response_body = ctx::app::Response::<schemas::event::AvailableEvents>{
                message: FETCHED,
                data: Some(available_events), //-- data is an empty &[u8] array
                status: 200,
            };
            Ok(
                HttpResponse::Ok().json(
                    response_body
                ).into_body()
            )
        },
        Err(e) => {
            let response_body = ctx::app::Response::<ctx::app::Nill>{
                data: Some(ctx::app::Nill(&[])), //-- data is an empty &[u8] array
                message: &e.to_string(), //-- take a reference to the string error
                status: 500,
            };
            Ok(
                HttpResponse::InternalServerError().json(
                    response_body
                ).into_body()
            )
        },
    }
    
}


#[post("/cast-vote")]
async fn cast_vote_event(req: HttpRequest, vote_info: web::Json<schemas::event::CastVoteRequest>) -> Result<HttpResponse, Error>{
    
    let conn = utils::db::connection().await;
    let app_storage = match conn.as_ref().unwrap().db.as_ref().unwrap().mode{ //-- here as_ref() method convert &Option<T> to Option<&T>
        ctx::app::Mode::On => conn.as_ref().as_ref().unwrap().db.as_ref().unwrap().instance.as_ref(), //-- return the db if it wasn't detached - instance.as_ref() will return the Option<&Client>
        ctx::app::Mode::Off => None, //-- no db is available cause it's off
    };

    let vote_info = vote_info.into_inner(); //-- into_inner() will deconstruct to an inner value and return T
    let event_id = ObjectId::parse_str(vote_info._id.as_str()).unwrap(); //-- generating mongodb object id from the id string 
    let events = app_storage.unwrap().database("bitrader").collection::<schemas::event::EventInfo>("events"); //-- selecting events collection to fetch all event infos into the EventInfo struct
    match events.find_one(doc!{"_id": event_id}, None).unwrap(){ //-- finding event based on event title and id
        Some(event_doc) => { //-- deserializing BSON into the EventInfo struct
            let mut upvotes = event_doc.upvotes.unwrap(); //-- trait Copy is implemented for u16 thus we don't loose the ownership when we move them into a new scope
            let mut downvotes = event_doc.downvotes.unwrap(); //-- trait Copy is implemented for u16 thus we don't loose the ownership when we move them into a new scope
            if vote_info.voter.is_upvote{
                upvotes+=1;
            }
            if !vote_info.voter.is_upvote{
                downvotes+=1;
            }
            let updated_voters = event_doc.clone().add_voter(vote_info.clone().voter).await;
            let serialized_voters = bson::to_bson(&updated_voters).unwrap(); //-- we have to serialize the updated_voters to BSON Document object in order to update voters field inside the collection
            let serialized_upvotes = bson::to_bson(&upvotes).unwrap(); //-- we have to serialize the upvotes to BSON Document object in order to update voters field inside the collection
            let serialized_downvotes = bson::to_bson(&downvotes).unwrap(); //-- we have to serialize the downvotes to BSON Document object in order to update voters field inside the collection
            match events.update_one(doc!{"_id": event_id}, doc!{"$set": { "voters": serialized_voters, "upvotes": serialized_upvotes, "downvotes": serialized_downvotes }}, None){
                Ok(updated_result) => {
                    let response_body = ctx::app::Response::<ctx::app::Nill>{ //-- we have to specify a generic type for data field in Response struct which in our case is Nill struct
                        data: Some(ctx::app::Nill(&[])), //-- data is an empty &[u8] array
                        message: UPDATED, //-- collection found in bitrader document (database)
                        status: 200,
                    };
                    Ok(
                        HttpResponse::Ok().json(
                            response_body
                        ).into_body()
                    )
                },
                Err(e) => {
                    let response_body = ctx::app::Response::<ctx::app::Nill>{
                        data: Some(ctx::app::Nill(&[])), //-- data is an empty &[u8] array
                        message: &e.to_string(), //-- take a reference to the string error
                        status: 500,
                    };
                    Ok(
                        HttpResponse::InternalServerError().json(
                            response_body
                        ).into_body()
                    )
                },
            }
        }, 
        None => { //-- means we didn't find any document related to this title and we have to tell the user to create a new event
            let response_body = ctx::app::Response::<ctx::app::Nill>{ //-- we have to specify a generic type for data field in Response struct which in our case is Nill struct
                data: Some(ctx::app::Nill(&[])), //-- data is an empty &[u8] array
                message: NOT_FOUND_DOCUMENT,
                status: 404,
            };
            Ok(
                HttpResponse::NotFound().json(
                    response_body
                ).into_body()
            )
        },
    }
    
}


#[post("/set-expire")]
async fn expire_event(req: HttpRequest, exp_info: web::Json<schemas::event::ExpireEventRequest>) -> Result<HttpResponse, Error>{
    
    let conn = utils::db::connection().await;
    let app_storage = match conn.as_ref().unwrap().db.as_ref().unwrap().mode{ //-- here as_ref() method convert &Option<T> to Option<&T>
        ctx::app::Mode::On => conn.as_ref().as_ref().unwrap().db.as_ref().unwrap().instance.as_ref(), //-- return the db if it wasn't detached - instance.as_ref() will return the Option<&Client>
        ctx::app::Mode::Off => None, //-- no db is available cause it's off
    };

    let exp_info = exp_info.into_inner(); //-- into_inner() will deconstruct to an inner value and return T
    let event_id = ObjectId::parse_str(exp_info._id.as_str()).unwrap(); //-- generating mongodb object id from the id string
    let events = app_storage.unwrap().database("bitrader").collection::<schemas::event::EventInfo>("events"); //-- selecting events collection to fetch all event infos into the EventInfo struct
    match events.find_one_and_update(doc!{"_id": event_id}, doc!{"$set": {"is_expired": true}}, None).unwrap(){ //-- finding event based on event id
        Some(event_doc) => { //-- deserializing BSON into the EventInfo struct
            let response_body = ctx::app::Response::<schemas::event::EventInfo>{ //-- we have to specify a generic type for data field in Response struct which in our case is EventInfo struct
                data: Some(event_doc), //-- data is an empty &[u8] array
                message: UPDATED, //-- collection found in bitrader document (database)
                status: 200,
            };
            Ok(
                HttpResponse::Ok().json(
                    response_body
                ).into_body()
            )
        }, 
        None => { //-- means we didn't find any document related to this title and we have to tell the user to create a new event
            let response_body = ctx::app::Response::<ctx::app::Nill>{ //-- we have to specify a generic type for data field in Response struct which in our case is Nill struct
                data: Some(ctx::app::Nill(&[])), //-- data is an empty &[u8] array
                message: NOT_FOUND_DOCUMENT,
                status: 404,
            };
            Ok(
                HttpResponse::NotFound().json(
                    response_body
                ).into_body()
            )
        },
    }

}


#[post("/delete/{id}/{api_key}")]
async fn delete_event(req: HttpRequest, param: web::Path<(String, String)>) -> Result<HttpResponse, Error>{
    
    // let storage = req.app_data::<web::Data<Option<Arc<ctx::app::Storage>>>>().unwrap(); //-- unwrapping the db inside the web data structure which is passed inside the app_data() method
    // let app_storage = match storage.as_ref().as_ref().unwrap().db.as_ref().unwrap().mode{ //-- here as_ref() method convert &Option<T> to Option<&T>
    //     ctx::app::Mode::On => storage.as_ref().as_ref().unwrap().db.as_ref().unwrap().instance.as_ref(), //-- return the db if it wasn't detached - instance.as_ref() will return the Option<&Client>
    //     ctx::app::Mode::Off => None, //-- no db is available cause it's off
    // };

    let delete_api_key = env::var("event_DELETE_KEY").expect("⚠️ no api key variable set");
    let db_host = env::var("DB_HOST").expect("⚠️ no db host variable set");
    let db_port = env::var("DB_PORT").expect("⚠️ no db port variable set");
    let db_engine = env::var("DB_ENGINE").expect("⚠️ no db engine variable set");
    let db_addr = format!("{}://{}:{}", db_engine, db_host, db_port);
    let app_storage = Client::with_uri_str(db_addr).unwrap();

    let param = param.into_inner(); //-- into_inner() will convert the id and api_key into its actual type which is of type String - param.1 is the api key and param.0 is the id of the event
    if param.1 == delete_api_key{
        let event_id = ObjectId::parse_str(param.0.as_str()).unwrap(); //-- generating mongodb object id from the id string
        let events = app_storage.database("fishuman").collection::<schemas::fishuman::EventInfo>("events"); //-- selecting events collection to fetch all event infos into the EventInfo struct
        match events.find_one_and_delete(doc!{"_id": event_id}, None).unwrap(){ //-- finding event based on event id
            Some(event_doc) => { //-- deserializing BSON into the eventInfo struct
                let response_body = ctx::app::Response::<schemas::fishuman::EventInfo>{ //-- we have to specify a generic type for data field in Response struct which in our case is EventInfo struct
                    data: Some(event_doc), //-- data is an empty &[u8] array
                    message: DELETED, //-- collection found in fishuman document (database)
                    status: 200,
                };
                Ok(
                    HttpResponse::Ok().json(
                        response_body
                    ).into_body() //-- call this method in order not to get failed to fetch in client side
                )
            }, 
            None => { //-- means we didn't find any document related to this title and we have to tell the user to create a new event
                let response_body = ctx::app::Response::<ctx::app::Nill>{ //-- we have to specify a generic type for data field in Response struct which in our case is Nill struct
                    data: Some(ctx::app::Nill(&[])), //-- data is an empty &[u8] array
                    message: NOT_FOUND_DOCUMENT,
                    status: 404,
                };
                Ok(
                    HttpResponse::NotFound().json(
                        response_body
                    ).into_body() //-- call this method in order not to get failed to fetch in client side
                )
            },
        }
    } else{
        let response_body = ctx::app::Response::<ctx::app::Nill>{ //-- we have to specify a generic type for data field in Response struct which in our case is Nill struct
            data: Some(ctx::app::Nill(&[])), //-- data is an empty &[u8] array
            message: WRONG_API_KEY,
            status: 403,
        };
        Ok(
            HttpResponse::Forbidden().json(
                response_body
            ).into_body() //-- call this method in order not to get failed to fetch in client side
        )
    }

}








pub fn register(config: &mut web::ServiceConfig){
    config.service(add_event);
    config.service(cast_vote_event);
    config.service(expire_event);
    config.service(get_all_events);
    config.service(delete_event);
}
