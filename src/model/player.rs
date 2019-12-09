use crate::schema::{player_factories, player_inventory, player_stats, players_data};
use chrono::prelude::*;
use uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Queryable, Insertable)]
#[table_name = "players_data"]
pub struct PlayerData {
    pub energy: i32,
    pub gold: i32,
    pub exp: i32,
    pub id: uuid::Uuid,
    pub last_updated: NaiveDateTime,
    pub gold_acc: i32,
    pub player_stats_id: uuid::Uuid,
    pub player_inventory_id: uuid::Uuid,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Queryable, Insertable)]
#[table_name = "player_factories"]
pub struct PlayerFactories {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub factory_id: uuid::Uuid,
    pub amount: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Queryable, Insertable)]
#[table_name = "player_inventory"]
pub struct PlayerInventory {
    pub id: uuid::Uuid,
    pub food_q1: i32,
    pub weapon_q1: i32,
    pub capacity: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Queryable, Insertable)]
#[table_name = "player_stats"]
pub struct PlayerStats {
    pub id: uuid::Uuid,
    pub strength: i32,
    pub agility: i32,
    pub stamina: i32,
}
