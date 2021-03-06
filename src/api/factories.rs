use actix_web::{web, Error, HttpResponse};
use diesel::prelude::*;
use uuid;

use crate::model::{
    factory::Factory,
    player::{PlayerData, PlayerFactories, PlayerInventory},
    user::User,
};
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
            .expect("General buying new factory"),
    )
}
/// diesel::work at specific company => - 10 energy + products
fn work_query(
    payload: web::Json<PlayerPayload>,
    pool: web::Data<Pool>,
) -> Result<String, diesel::result::Error> {
    use crate::schema::factories::dsl::factories;
    // use crate::schema::player_factories::dsl::{amount, factory_id, player_factories, user_id};
    use crate::schema::player_inventory::dsl::{food_q1, player_inventory, weapon_q1};
    use crate::schema::players_data::dsl::{energy, gold_acc, players_data};
    use crate::schema::users::dsl::users;
    let conn: &PgConnection = &pool.get().unwrap();

    let current_factory = factories
        .filter(factories.primary_key().eq(&payload.factory_id))
        .first::<Factory>(conn)
        .unwrap();

    let player: User = users.find(&payload.user_id).first(conn).unwrap();
    let curr_player_data: PlayerData = players_data
        .find(&player.player_data_id)
        .first(conn)
        .unwrap();

    let storage: PlayerInventory = player_inventory
        .filter(
            player_inventory
                .primary_key()
                .eq(&curr_player_data.player_inventory_id),
        )
        .first(conn)
        .unwrap();

    // ?. check if has storage space
    let current_storage = storage.food_q1 + storage.weapon_q1;
    if storage.capacity < current_storage + current_factory.product_amount {
        return Ok(format!(
            "Cappacity Reached {}: current:{}, new:{}",
            storage.capacity, current_storage, current_factory.product_amount
        ));
    }

    // 1. Take from player_data -10 energy
    diesel::update(players_data)
        .filter(players_data.primary_key().eq(&player.player_data_id))
        .set(energy.eq(energy - 10))
        .execute(conn)
        .unwrap();
    // 2. add specific factory product to player inventory
    if current_factory.product == "food".to_owned() {
        println!("TESTING FOOD");
        diesel::update(player_inventory)
            .filter(
                player_inventory
                    .primary_key()
                    .eq(&curr_player_data.player_inventory_id),
            )
            .set(food_q1.eq(food_q1 + current_factory.product_amount))
            .execute(conn)
            .unwrap();
    }
    if current_factory.product == "weapon".to_owned() {
        diesel::update(player_inventory)
            .filter(
                player_inventory
                    .primary_key()
                    .eq(&curr_player_data.player_inventory_id),
            )
            .set(weapon_q1.eq(weapon_q1 + current_factory.product_amount))
            .execute(conn)
            .unwrap();
    }
    // ?check if he owns that company
    // let item = player_factories
    //     .filter(user_id.eq(&payload.user_id))
    //     .filter(factory_id.eq(&payload.factory_id))
    //     .get_result::<PlayerFactories>(conn)
    //     .optional()?;

    diesel::update(players_data)
        .filter(players_data.primary_key().eq(&player.player_data_id))
        .set(gold_acc.eq(gold_acc + current_factory.gold_per_day))
        .execute(conn)
        .unwrap();

    //new_factories
    Ok(format!(
        "Success, u earned {} {}",
        current_factory.product_amount, current_factory.product
    ))
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

/// delete old company, - resourses, + new company
fn upgrade_factory_query(
    payload: web::Json<PlayerPayload>,
    pool: web::Data<Pool>,
) -> Result<String, diesel::result::Error> {
    use crate::schema::factories::dsl::{factories, level, product};
    use crate::schema::player_factories::dsl::{amount, factory_id, player_factories, user_id};
    use crate::schema::player_inventory::dsl::{player_inventory, special_currency};
    use crate::schema::players_data::dsl::{gold, gold_acc, players_data};
    use crate::schema::users::dsl::users;
    let conn: &PgConnection = &pool.get().unwrap();

    let player: User = users
        .find(&payload.user_id)
        .first(conn)
        .expect("Cant Find Player");
    let curr_player_data: PlayerData = players_data
        .find(&player.player_data_id)
        .first(conn)
        .expect("No player data");

    let current_factory: Factory = factories
        .filter(factories.primary_key().eq(&payload.factory_id))
        .first::<Factory>(conn)
        .expect("Cant Find Factory");

    //  dbg!(&player);
    let inventory: PlayerInventory = player_inventory
        .filter(
            player_inventory
                .primary_key()
                .eq(&curr_player_data.player_inventory_id),
        )
        .first(conn)
        .expect("Cant Find Inventory");

    //1. check if you have enough gold and resourses
    // check if has storage space
    // //! ADD SPECIAL CURRECNCY PRICE !!
    if curr_player_data.gold < current_factory.price || inventory.special_currency < 10 {
        return Ok(format!("You don't have enough resourses"));
    }
    //2. delete old company
    let item = player_factories
        .filter(user_id.eq(&payload.user_id))
        .filter(factory_id.eq(&payload.factory_id))
        .get_result::<PlayerFactories>(conn)
        .optional()?;

    let _item_test = match item {
        Some(factory) => {
            if factory.amount > 1 {
                diesel::update(player_factories)
                    .filter(user_id.eq(&payload.user_id))
                    .filter(factory_id.eq(&payload.factory_id))
                    .set(amount.eq(amount - 1))
                    .execute(conn)
                    .expect("Updating player_factories amount");
            } else {
                diesel::delete(
                    player_factories
                        .filter(user_id.eq(&payload.user_id))
                        .filter(factory_id.eq(&payload.factory_id)),
                )
                .execute(conn)
                .expect("Deleting old Factory");
            }
        }
        None => return Ok(format!("You don't own this factory")),
    };

    //3. remove resourses
    diesel::update(players_data)
        .filter(players_data.primary_key().eq(&player.player_data_id))
        .set(gold.eq(gold - current_factory.price))
        .execute(conn)
        .expect("Error updating player_data gold");

    diesel::update(player_inventory)
        .filter(
            player_inventory
                .primary_key()
                .eq(&curr_player_data.player_inventory_id),
        )
        .set(special_currency.eq(special_currency - 0))
        .execute(conn)
        .expect("Can't update special currency");
    //4. add new company
    let new_factory: Factory = factories
        .filter(product.eq(current_factory.product))
        .filter(level.eq(current_factory.level + 1))
        .first(conn)
        .expect("No factory to upgrade");

    let new_factories = PlayerFactories {
        id: uuid::Uuid::new_v4(),
        user_id: payload.user_id,
        factory_id: new_factory.id,
        amount: 1,
    };

    diesel::insert_into(player_factories)
        .values(&new_factories)
        .execute(conn)?;

    diesel::update(players_data)
        .set(gold_acc.eq(gold_acc - current_factory.gold_per_day + new_factory.gold_per_day))
        .execute(conn)
        .expect("Updating player_data Err");

    //new_factories
    Ok(format!(
        "Successfully upgraded to level {}",
        new_factory.level
    ))
}

/// upgrade company => - resourses + add new factory remove old
pub async fn upgrade_factory(
    player_data: web::Json<PlayerPayload>,
    pool: web::Data<Pool>,
) -> Result<HttpResponse, Error> {
    Ok(web::block(move || upgrade_factory_query(player_data, pool))
        .await
        .map(|result| HttpResponse::Ok().json(result))
        .map_err(|_| HttpResponse::InternalServerError())
        .expect("General upgrade factory Error"))
}
