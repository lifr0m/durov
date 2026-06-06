use crate::datacenters::static_dc;
use crate::sessions::encoding::{decode_peer_id, encode_peer_id, PeerType};
use crate::sessions::{get_date, Peer, Session};
use crate::{tl, Error};
use async_trait::async_trait;
use sqlx::sqlite::{SqliteConnectOptions, SqliteRow};
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use std::str::FromStr;
use tl::types::updates::State;

const VERSION: i32 = 7;

pub struct Telethon {
    pool: SqlitePool,
}

#[async_trait]
impl Session for Telethon {
    async fn connect(path: &str) -> Result<Self, Error> {
        let url = format!("sqlite://{path}");
        let options = SqliteConnectOptions::from_str(&url)?
            .create_if_missing(true);
        let pool = SqlitePool::connect_with(options).await?;

        Ok(Self { pool })
    }

    async fn init(&self) -> Result<(), Error> {
        self.create_tables().await?;
        self.ensure_version().await?;

        Ok(())
    }

    async fn get_peer_by_id(&self, id: i64, typ: PeerType) -> Result<Option<Peer>, Error> {
        let row = sqlx::query("SELECT * FROM entities WHERE id = ?")
            .bind(encode_peer_id(id, typ))
            .fetch_optional(&self.pool).await?;

        Ok(row.map(peer_from_row))
    }

    async fn get_peer_by_username(&self, username: &str) -> Result<Option<Peer>, Error> {
        let row = sqlx::query("SELECT * FROM entities WHERE username = ?")
            .bind(username)
            .fetch_optional(&self.pool).await?;

        Ok(row.map(peer_from_row))
    }

    async fn set_peers<I>(&self, iter: I) -> Result<(), Error>
    where
        I: Iterator<Item = Peer> + Send,
    {
        let mut transaction = self.pool.begin().await?;

        for peer in iter {
            sqlx::query("DELETE FROM entities WHERE id = ?")
                .bind(encode_peer_id(peer.id, peer.typ))
                .execute(&mut *transaction).await?;

            if peer.username.is_some() {
                sqlx::query("UPDATE entities SET username = NULL WHERE username = ?")
                    .bind(&peer.username)
                    .execute(&mut *transaction).await?;
            }

            sqlx::query("INSERT INTO entities VALUES (?, ?, ?, NULL, NULL, ?)")
                .bind(encode_peer_id(peer.id, peer.typ))
                .bind(peer.access_hash)
                .bind(&peer.username)
                .bind(get_date())
                .execute(&mut *transaction).await?;
        }

        transaction.commit().await?;

        Ok(())
    }

    async fn get_main_dc(&self) -> Result<Option<i32>, Error> {
        let row = sqlx::query("SELECT * FROM sessions")
            .fetch_optional(&self.pool).await?;

        Ok(row.map(|row| row.get("dc_id")))
    }

    async fn set_main_dc(&self, dc_id: i32) -> Result<(), Error> {
        sqlx::query("DELETE FROM sessions WHERE dc_id != ?")
            .bind(dc_id)
            .execute(&self.pool).await?;

        Ok(())
    }

    async fn get_auth_key(&self, dc_id: i32) -> Result<Option<[u8; 256]>, Error> {
        let row = sqlx::query("SELECT * FROM sessions WHERE dc_id = ?")
            .bind(dc_id)
            .fetch_optional(&self.pool).await?;

        Ok(row.map(|row| {
            row.get::<&[u8], _>("auth_key")
                .try_into()
                .unwrap()
        }))
    }

    async fn set_auth_key(&self, dc_id: i32, auth_key: [u8; 256]) -> Result<(), Error> {
        let mut transaction = self.pool.begin().await?;

        let result = sqlx::query("DELETE FROM sessions WHERE dc_id = ?")
            .bind(dc_id)
            .execute(&mut *transaction).await?;

        if result.rows_affected() > 0 {
            sqlx::query("INSERT INTO sessions VALUES (?, ?, ?, ?, NULL)")
                .bind(dc_id)
                .bind(static_dc(dc_id).host)
                .bind(static_dc(dc_id).port)
                .bind(&auth_key[..])
                .execute(&mut *transaction).await?;
        }

        transaction.commit().await?;

        Ok(())
    }

    async fn get_states(&self) -> Result<HashMap<i64, State>, Error> {
        let row_list = sqlx::query("SELECT * FROM update_state")
            .fetch_all(&self.pool).await?;

        let map = row_list.into_iter()
            .map(|row| (
                row.get("id"),
                State {
                    pts: row.get("pts"),
                    qts: row.get("qts"),
                    date: row.get("date"),
                    seq: row.get("seq"),
                    unread_count: 0,
                },
            ))
            .collect();

        Ok(map)
    }

    async fn set_states(&self, map: &HashMap<i64, State>) -> Result<(), Error> {
        let mut transaction = self.pool.begin().await?;

        sqlx::query("DELETE FROM update_state")
            .execute(&mut *transaction).await?;

        for (&id, state) in map {
            sqlx::query("INSERT INTO update_state VALUES (?, ?, ?, ?, ?)")
                .bind(id)
                .bind(state.pts)
                .bind(state.qts)
                .bind(state.date)
                .bind(state.seq)
                .execute(&mut *transaction).await?;
        }

        transaction.commit().await?;

        Ok(())
    }
}

impl Telethon {
    async fn create_tables(&self) -> Result<(), Error> {
        sqlx::query("
CREATE TABLE IF NOT EXISTS entities (
    id INTEGER PRIMARY KEY,
    hash INTEGER NOT NULL,
    username TEXT,
    phone INTEGER,
    name TEXT,
    date INTEGER
)
").execute(&self.pool).await?;

        sqlx::query("
CREATE TABLE IF NOT EXISTS sent_files (
    md5_digest BLOB,
    file_size INTEGER,
    type INTEGER,
    id INTEGER,
    hash INTEGER,
    PRIMARY KEY (md5_digest, file_size, type)
)
").execute(&self.pool).await?;

        sqlx::query("
CREATE TABLE IF NOT EXISTS sessions (
    dc_id INTEGER PRIMARY KEY,
    server_address TEXT,
    port INTEGER,
    auth_key BLOB,
    takeout_id INTEGER
)
").execute(&self.pool).await?;

        sqlx::query("
CREATE TABLE IF NOT EXISTS update_state (
    id INTEGER PRIMARY KEY,
    pts INTEGER,
    qts INTEGER,
    date INTEGER,
    seq INTEGER
)
").execute(&self.pool).await?;

        sqlx::query("
CREATE TABLE IF NOT EXISTS version (
    version INTEGER PRIMARY KEY
)
").execute(&self.pool).await?;

        Ok(())
    }

    async fn ensure_version(&self) -> Result<(), Error> {
        let row = sqlx::query("SELECT * FROM version")
            .fetch_optional(&self.pool).await?;

        if let Some(row) = row {
            assert_eq!(row.get::<i32, _>("version"), VERSION);
        } else {
            sqlx::query("INSERT INTO version VALUES (?)")
                .bind(VERSION)
                .execute(&self.pool).await?;
        }

        Ok(())
    }
}

fn peer_from_row(row: SqliteRow) -> Peer {
    let id = row.get("id");
    let (id, typ) = decode_peer_id(id);
    let access_hash = row.get("hash");
    let username = row.get("username");
    Peer { id, typ, access_hash, username }
}
