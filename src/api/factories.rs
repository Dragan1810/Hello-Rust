use actix_web::{web, Error, HttpResponse};
use diesel::prelude::*;
use uuid;

use crate::model::{factory::Factory, player::PlayerFactories};
use crate::share::db::Pool;

fn query_get_factories(pool: web::Data<Pool>) -> Result<Vec<Factory>, diesel::result::Error> {
    use crate::schema::factories::dsl::*;
    let conn: &PgConnection = &pool.get().unwrap();

    let items = factories.load::<Factory>(conn).unwrap();
    Ok(items)
}

pub async fn get_factories(pool: web::Data<Pool>) -> Result<HttpResponse, Error> {
    Ok(web::block(move || query_get_factories(pool))
        .await
        .map(|user| HttpResponse::Ok().json(user))
        .map_err(|_| HttpResponse::InternalServerError())
        .unwrap())
}

fn query_get_player_factories(
    user: web::Json<UserId>,
    pool: web::Data<Pool>,
) -> Result<Vec<PlayerFactories>, diesel::result::Error> {
    use crate::schema::player_factories::dsl::*;
    let conn: &PgConnection = &pool.get().unwrap();

    let items = player_factories
        .filter(user_id.eq(&user.id))
        .load::<PlayerFactories>(conn)?;

    Ok(items)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserId {
    pub id: uuid::Uuid,
}

pub async fn get_player_factories(
    user: web::Json<UserId>,
    pool: web::Data<Pool>,
) -> Result<HttpResponse, Error> {
    Ok(web::block(move || query_get_player_factories(user, pool))
        .await
        .map(|user| HttpResponse::Ok().json(user))
        .map_err(|_| HttpResponse::InternalServerError())
        .unwrap())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerPayload {
    pub user_id: uuid::Uuid,
    pub factory_id: uuid::Uuid,
}

fn query_add_player_factories(
    payload: web::Json<PlayerPayload>,
    pool: web::Data<Pool>,
) -> Result<PlayerFactories, diesel::result::Error> {
    use crate::schema::factories::dsl::{factories, gold_per_day, id};
    use crate::schema::player_factories::dsl::{amount, factory_id, player_factories, user_id};
    use crate::schema::players_data::dsl::{gold_acc, players_data};
    let conn: &PgConnection = &pool.get().unwrap();

    let item = player_factories
        .filter(user_id.eq(&payload.user_id))
        .filter(factory_id.eq(&payload.factory_id))
        .get_result::<PlayerFactories>(conn)
        .optional()?;

    let new_factories = match item {
        Some(data) => {
            let new_amount = data.amount + 1;

            let updated = diesel::update(player_factories)
                .filter(user_id.eq(&payload.user_id))
                .filter(factory_id.eq(&payload.factory_id))
                .set(amount.eq(new_amount))
                .get_result(conn)?;

            Ok(updated)
        }
        None => {
            let new_factories = PlayerFactories {
                id: uuid::Uuid::new_v4(),
                user_id: payload.user_id,
                factory_id: payload.factory_id,
                amount: 1,
            };

            diesel::insert_into(player_factories)
                .values(&new_factories)
                .execute(conn)?;

            Ok(new_factories)
        }
    };

    let gold = factories
        .filter(id.eq(&payload.factory_id))
        .select(gold_per_day)
        .first::<i32>(conn)
        .unwrap();

    diesel::update(players_data)
        .set(gold_acc.eq(gold_acc + gold))
        .execute(conn)
        .unwrap();

    new_factories
}

pub async fn add_player_factories(
    player_data: web::Json<PlayerPayload>,
    pool: web::Data<Pool>,
) -> Result<HttpResponse, Error> {
    Ok(
        web::block(move || query_add_player_factories(player_data, pool))
            .await
            .map(|user| HttpResponse::Ok().json(user))
            .map_err(|_| HttpResponse::InternalServerError())
            .unwrap(),
    )
}

fn work_query(
    payload: web::Json<PlayerPayload>,
    pool: web::Data<Pool>,
) -> Result<PlayerFactories, diesel::result::Error> {
    use crate::schema::factories::dsl::{factories, gold_per_day, id};
    use crate::schema::player_factories::dsl::{amount, factory_id, player_factories, user_id};
    use crate::schema::players_data::dsl::{gold_acc, players_data};
    let conn: &PgConnection = &pool.get().unwrap();
    // ?. check if has storage space
    // 1. Take from player_data -10 energy
    // 2. add specific factory product to player inventory

    let item = player_factories
        .filter(user_id.eq(&payload.user_id))
        .filter(factory_id.eq(&payload.factory_id))
        .get_result::<PlayerFactories>(conn)
        .optional()?;

    let new_factories = match item {
        Some(data) => {
            let new_amount = data.amount + 1;

            let updated = diesel::update(player_factories)
                .filter(user_id.eq(&payload.user_id))
                .filter(factory_id.eq(&payload.factory_id))
                .set(amount.eq(new_amount))
                .get_result(conn)?;

            Ok(updated)
        }
        None => {
            let new_factories = PlayerFactories {
                id: uuid::Uuid::new_v4(),
                user_id: payload.user_id,
                factory_id: payload.factory_id,
                amount: 1,
            };

            diesel::insert_into(player_factories)
                .values(&new_factories)
                .execute(conn)?;

            Ok(new_factories)
        }
    };

    let gold = factories
        .filter(id.eq(&payload.factory_id))
        .select(gold_per_day)
        .first::<i32>(conn)
        .unwrap();

    diesel::update(players_data)
        .set(gold_acc.eq(gold_acc + gold))
        .execute(conn)
        .unwrap();

    new_factories
}

/// work at specific company => - 10 energy + products
pub async fn work_factory(
    player_data: web::Json<PlayerPayload>,
    pool: web::Data<Pool>,
) -> Result<HttpResponse, Error> {
    Ok(web::block(move || work_query(player_data, pool))
        .await
        .map(|user| HttpResponse::Ok().json(user))
        .map_err(|_| HttpResponse::InternalServerError())
        .unwrap())
}

// upgrade company => - resourses + add new factory remove old
//? cant work if low storage
