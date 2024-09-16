// Copyright (C) 2022  Aravinth Manivannan <realaravinth@batsense.net>
// SPDX-FileCopyrightText: 2023 Aravinth Manivannan <realaravinth@batsense.net>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::str::FromStr;

use db_core::dev::*;

use sqlx::mysql::MySqlPoolOptions;
use sqlx::types::time::OffsetDateTime;
use sqlx::ConnectOptions;
use sqlx::MySqlPool;
use uuid::Uuid;

pub mod errors;
#[cfg(test)]
pub mod tests;

#[derive(Clone)]
pub struct Database {
    pub pool: MySqlPool,
}

/// Use an existing database pool
pub struct Conn(pub MySqlPool);

/// Connect to database
pub enum ConnectionOptions {
    /// fresh connection
    Fresh(Fresh),
    /// existing connection
    Existing(Conn),
}

pub struct Fresh {
    pub pool_options: MySqlPoolOptions,
    pub disable_logging: bool,
    pub url: String,
}

pub mod dev {
    pub use super::errors::*;
    pub use super::Database;
    pub use db_core::dev::*;
    pub use sqlx::Error;
}

pub mod prelude {
    pub use super::*;
}

#[async_trait]
impl Connect for ConnectionOptions {
    type Pool = Database;
    async fn connect(self) -> DBResult<Self::Pool> {
        let pool = match self {
            Self::Fresh(fresh) => {
                let mut connect_options =
                    sqlx::mysql::MySqlConnectOptions::from_str(&fresh.url).unwrap();
                if fresh.disable_logging {
                    connect_options = connect_options.disable_statement_logging();
                }
                fresh
                    .pool_options
                    .connect_with(connect_options)
                    .await
                    .map_err(|e| DBError::DBError(Box::new(e)))?
            }

            Self::Existing(conn) => conn.0,
        };
        Ok(Database { pool })
    }
}

use dev::*;

#[async_trait]
impl Migrate for Database {
    async fn migrate(&self) -> DBResult<()> {
        sqlx::migrate!("./migrations/")
            .run(&self.pool)
            .await
            .map_err(|e| DBError::DBError(Box::new(e)))?;
        Ok(())
    }
}

#[async_trait]
impl MCDatabase for Database {
    /// ping DB
    async fn ping(&self) -> bool {
        use sqlx::Connection;

        if let Ok(mut con) = self.pool.acquire().await {
            con.ping().await.is_ok()
        } else {
            false
        }
    }

    /// register a new user
    async fn register(&self, p: &Register) -> DBResult<()> {
        let res = if let Some(email) = &p.email {
            sqlx::query!(
                "insert into mcaptcha_users 
        (name , password, email, secret) values (?, ?, ?, ?)",
                &p.username,
                &p.hash,
                &email,
                &p.secret,
            )
            .execute(&self.pool)
            .await
        } else {
            sqlx::query!(
                "INSERT INTO mcaptcha_users 
        (name , password,  secret) VALUES (?, ?, ?)",
                &p.username,
                &p.hash,
                &p.secret,
            )
            .execute(&self.pool)
            .await
        };
        res.map_err(map_register_err)?;
        Ok(())
    }

    /// delete a user
    async fn delete_user(&self, username: &str) -> DBResult<()> {
        sqlx::query!("DELETE FROM mcaptcha_users WHERE name = (?)", username)
            .execute(&self.pool)
            .await
            .map_err(|e| map_row_not_found_err(e, DBError::AccountNotFound))?;
        Ok(())
    }

    /// check if username exists
    async fn username_exists(&self, username: &str) -> DBResult<bool> {
        match sqlx::query!("SELECT name from mcaptcha_users WHERE name = ?", username,)
            .fetch_one(&self.pool)
            .await
        {
            Ok(_) => Ok(true),
            Err(sqlx::Error::RowNotFound) => Ok(false),
            Err(e) => Err(map_register_err(e)),
        }
    }

    /// get user email
    async fn get_email(&self, username: &str) -> DBResult<Option<String>> {
        struct Email {
            email: Option<String>,
        }

        let res = sqlx::query_as!(
            Email,
            "SELECT email FROM mcaptcha_users WHERE name = ?",
            username
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| map_row_not_found_err(e, DBError::AccountNotFound))?;
        Ok(res.email)
    }

    /// check if email exists
    async fn email_exists(&self, email: &str) -> DBResult<bool> {
        match sqlx::query!("SELECT name from mcaptcha_users WHERE email = ?", email)
            .fetch_one(&self.pool)
            .await
        {
            Ok(_) => Ok(true),
            Err(sqlx::Error::RowNotFound) => Ok(false),
            Err(e) => Err(map_register_err(e)),
        }
    }

    /// update a user's email
    async fn update_email(&self, p: &UpdateEmail) -> DBResult<()> {
        sqlx::query!(
            "UPDATE mcaptcha_users set email = ?
            WHERE name = ?",
            &p.new_email,
            &p.username,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| map_row_not_found_err(e, DBError::AccountNotFound))?;

        Ok(())
    }

    /// get a user's password
    async fn get_password(&self, l: &Login) -> DBResult<NameHash> {
        struct Password {
            name: String,
            password: String,
        }

        let rec = match l {
            Login::Username(u) => sqlx::query_as!(
                Password,
                r#"SELECT name, password  FROM mcaptcha_users WHERE name = ?"#,
                u,
            )
            .fetch_one(&self.pool)
            .await
            .map_err(|e| map_row_not_found_err(e, DBError::AccountNotFound))?,

            Login::Email(e) => sqlx::query_as!(
                Password,
                r#"SELECT name, password  FROM mcaptcha_users WHERE email = ?"#,
                e,
            )
            .fetch_one(&self.pool)
            .await
            .map_err(|e| map_row_not_found_err(e, DBError::AccountNotFound))?,
        };

        let res = NameHash {
            hash: rec.password,
            username: rec.name,
        };

        Ok(res)
    }

    /// update user's password
    async fn update_password(&self, p: &NameHash) -> DBResult<()> {
        sqlx::query!(
            "UPDATE mcaptcha_users set password = ?
            WHERE name = ?",
            &p.hash,
            &p.username,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| map_row_not_found_err(e, DBError::AccountNotFound))?;

        Ok(())
    }

    /// update username
    async fn update_username(&self, current: &str, new: &str) -> DBResult<()> {
        sqlx::query!(
            "UPDATE mcaptcha_users set name = ?
            WHERE name = ?",
            new,
            current,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| map_row_not_found_err(e, DBError::AccountNotFound))?;

        Ok(())
    }

    /// get a user's secret
    async fn get_secret(&self, username: &str) -> DBResult<Secret> {
        let secret = sqlx::query_as!(
            Secret,
            r#"SELECT secret  FROM mcaptcha_users WHERE name = ?"#,
            username,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| map_row_not_found_err(e, DBError::AccountNotFound))?;

        Ok(secret)
    }

    /// get a user's secret from a captcha key
    async fn get_secret_from_captcha(&self, key: &str) -> DBResult<Secret> {
        let secret = sqlx::query_as!(
            Secret,
            r#"SELECT secret  FROM mcaptcha_users WHERE ID = (
                    SELECT user_id FROM mcaptcha_config WHERE captcha_key = ?
                    )"#,
            key,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| map_row_not_found_err(e, DBError::AccountNotFound))?;

        Ok(secret)
    }

    /// update a user's secret
    async fn update_secret(&self, username: &str, secret: &str) -> DBResult<()> {
        sqlx::query!(
            "UPDATE mcaptcha_users set secret = ?
        WHERE name = ?",
            &secret,
            &username,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| map_row_not_found_err(e, DBError::AccountNotFound))?;

        Ok(())
    }

    /// create new captcha
    async fn create_captcha(&self, username: &str, p: &CreateCaptcha) -> DBResult<()> {
        sqlx::query!(
            "INSERT INTO mcaptcha_config
        (`captcha_key`, `user_id`, `duration`, `name`)
        VALUES (?, (SELECT ID FROM mcaptcha_users WHERE name = ?), ?, ?)",
            p.key,
            username,
            p.duration as i32,
            p.description,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| map_row_not_found_err(e, DBError::AccountNotFound))?;

        Ok(())
    }

    /// Get captcha config
    async fn get_captcha_config(&self, username: &str, key: &str) -> DBResult<Captcha> {
        let captcha = sqlx::query_as!(
            InternaleCaptchaConfig,
            "SELECT `config_id`, `duration`, `name`, `captcha_key` from mcaptcha_config WHERE
                        `captcha_key` = ? AND
                        user_id = (SELECT ID FROM mcaptcha_users WHERE name = ?) ",
            &key,
            &username,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| map_row_not_found_err(e, DBError::CaptchaNotFound))?;

        Ok(captcha.into())
    }

    /// Get all captchas belonging to user
    async fn get_all_user_captchas(&self, username: &str) -> DBResult<Vec<Captcha>> {
        let mut res = sqlx::query_as!(
            InternaleCaptchaConfig,
            "SELECT captcha_key, name, config_id, duration FROM mcaptcha_config WHERE
            user_id = (SELECT ID FROM mcaptcha_users WHERE name = ?) ",
            &username,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| map_row_not_found_err(e, DBError::AccountNotFound))?;

        let mut captchas = Vec::with_capacity(res.len());

        res.drain(0..).for_each(|r| captchas.push(r.into()));

        Ok(captchas)
    }

    /// update captcha metadata; doesn't change captcha key
    async fn update_captcha_metadata(
        &self,
        username: &str,
        p: &CreateCaptcha,
    ) -> DBResult<()> {
        sqlx::query!(
            "UPDATE mcaptcha_config SET name = ?, duration = ?
            WHERE user_id = (SELECT ID FROM mcaptcha_users WHERE name = ?)
            AND captcha_key = ?",
            p.description,
            p.duration,
            username,
            p.key,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| map_row_not_found_err(e, DBError::CaptchaNotFound))?;

        Ok(())
    }

    /// update captcha key; doesn't change metadata
    async fn update_captcha_key(
        &self,
        username: &str,
        old_key: &str,
        new_key: &str,
    ) -> DBResult<()> {
        sqlx::query!(
            "UPDATE mcaptcha_config SET captcha_key = ? 
        WHERE captcha_key = ? AND user_id = (SELECT ID FROM mcaptcha_users WHERE name = ?)",
            new_key,
            old_key,
            username,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| map_row_not_found_err(e, DBError::CaptchaNotFound))?;

        Ok(())
    }

    /// Add levels to captcha
    async fn add_captcha_levels(
        &self,
        username: &str,
        captcha_key: &str,
        levels: &[Level],
    ) -> DBResult<()> {
        use futures::future::try_join_all;
        let mut futs = Vec::with_capacity(levels.len());

        for level in levels.iter() {
            let difficulty_factor = level.difficulty_factor as i32;
            let visitor_threshold = level.visitor_threshold as i32;
            let fut = sqlx::query!(
                "INSERT INTO mcaptcha_levels (
            difficulty_factor, 
            visitor_threshold,
            config_id) VALUES  (
            ?, ?, (
                SELECT config_id FROM mcaptcha_config WHERE
                captcha_key = (?) AND user_id = (
                SELECT ID FROM mcaptcha_users WHERE name = ?
                    )));",
                difficulty_factor,
                visitor_threshold,
                &captcha_key,
                username,
            )
            .execute(&self.pool);
            futs.push(fut);
        }

        try_join_all(futs)
            .await
            .map_err(|e| map_row_not_found_err(e, DBError::CaptchaNotFound))?;

        let mut futs = Vec::with_capacity(levels.len());

        for level in levels.iter() {
            let difficulty_factor = level.difficulty_factor as i32;
            let visitor_threshold = level.visitor_threshold as i32;
            let fut = sqlx::query!(
                "INSERT INTO
                    mcaptcha_track_nonce (level_id, nonce)
                VALUES  ((
                    SELECT
                        level_id
                    FROM
                        mcaptcha_levels
                    WHERE
                        config_id = (SELECT config_id FROM mcaptcha_config WHERE captcha_key = ?)
                    AND
                        difficulty_factor = ?
                    AND
                        visitor_threshold = ?
                    ), ?);",
                &captcha_key,
                difficulty_factor,
                visitor_threshold,
                0,
            )
            .execute(&self.pool);
            futs.push(fut);
        }

        try_join_all(futs)
            .await
            .map_err(|e| map_row_not_found_err(e, DBError::CaptchaNotFound))?;

        Ok(())
    }

    /// check if captcha exists
    async fn captcha_exists(
        &self,
        username: Option<&str>,
        captcha_key: &str,
    ) -> DBResult<bool> {
        //        let mut exists = false;

        #[allow(dead_code)]
        struct ConfigId {
            config_id: i32,
        }
        let res = match username {
            Some(username) => {
                sqlx::query_as!(
                    ConfigId,
                    "SELECT config_id FROM mcaptcha_config
                        WHERE
                            captcha_key = ? 
                        AND user_id = (SELECT ID FROM mcaptcha_users WHERE name = ?)",
                    captcha_key,
                    username
                )
                .fetch_one(&self.pool)
                .await
                //                if let Some(x) = x.exists {
                //                    exists = x;
                //                };
            }

            None => {
                sqlx::query_as!(
                    ConfigId,
                    "SELECT config_id from mcaptcha_config WHERE captcha_key = ?",
                    &captcha_key,
                )
                .fetch_one(&self.pool)
                .await
            } //if let Some(x) = x.exists {
              //    exists = x;
              //};
        };
        match res {
            Ok(_) => Ok(true),
            Err(sqlx::Error::RowNotFound) => Ok(false),
            Err(e) => Err(map_register_err(e)),
        }
    }

    /// Delete all levels of a captcha
    async fn delete_captcha_levels(
        &self,
        username: &str,
        captcha_key: &str,
    ) -> DBResult<()> {
        sqlx::query!(
            "DELETE FROM mcaptcha_levels 
        WHERE config_id = (
            SELECT config_id FROM mcaptcha_config where captcha_key= (?) 
            AND user_id = (
            SELECT ID from mcaptcha_users WHERE name = ?
            )
            )",
            captcha_key,
            username
        )
        .execute(&self.pool)
        .await
        .map_err(|e| map_row_not_found_err(e, DBError::CaptchaNotFound))?;

        Ok(())
    }

    /// Delete captcha
    async fn delete_captcha(&self, username: &str, captcha_key: &str) -> DBResult<()> {
        sqlx::query!(
            "DELETE FROM mcaptcha_config where captcha_key= (?)
                AND
            user_id = (SELECT ID FROM mcaptcha_users WHERE name = ?)",
            captcha_key,
            username,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| map_row_not_found_err(e, DBError::CaptchaNotFound))?;

        Ok(())
    }

    /// Get captcha levels
    async fn get_captcha_levels(
        &self,
        username: Option<&str>,
        captcha_key: &str,
    ) -> DBResult<Vec<Level>> {
        struct I32Levels {
            difficulty_factor: i32,
            visitor_threshold: i32,
        }
        let levels = match username {
            None => sqlx::query_as!(
                I32Levels,
                "SELECT difficulty_factor, visitor_threshold FROM mcaptcha_levels  WHERE
            config_id = (
                SELECT config_id FROM mcaptcha_config where captcha_key= (?)
                ) ORDER BY difficulty_factor ASC;",
                captcha_key,
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| map_row_not_found_err(e, DBError::CaptchaNotFound))?,

            Some(username) => sqlx::query_as!(
                I32Levels,
                "SELECT difficulty_factor, visitor_threshold FROM mcaptcha_levels  WHERE
            config_id = (
                SELECT config_id FROM mcaptcha_config where captcha_key= (?)
                AND user_id = (SELECT ID from mcaptcha_users WHERE name = ?)
                )
            ORDER BY difficulty_factor ASC;",
                captcha_key,
                username
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| map_row_not_found_err(e, DBError::CaptchaNotFound))?,
        };

        let mut new_levels = Vec::with_capacity(levels.len());
        for l in levels.iter() {
            new_levels.push(Level {
                difficulty_factor: l.difficulty_factor as u32,
                visitor_threshold: l.visitor_threshold as u32,
            });
        }
        Ok(new_levels)
    }

    /// Get captcha's cooldown period
    async fn get_captcha_cooldown(&self, captcha_key: &str) -> DBResult<i32> {
        struct DurationResp {
            duration: i32,
        }

        let resp = sqlx::query_as!(
            DurationResp,
            "SELECT duration FROM mcaptcha_config  
            where captcha_key= ?",
            captcha_key,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| map_row_not_found_err(e, DBError::CaptchaNotFound))?;

        Ok(resp.duration)
    }
    /// Add traffic configuration
    async fn add_traffic_pattern(
        &self,
        username: &str,
        captcha_key: &str,
        pattern: &TrafficPattern,
    ) -> DBResult<()> {
        sqlx::query!(
            "INSERT INTO mcaptcha_sitekey_user_provided_avg_traffic (
            config_id,
            avg_traffic,
            peak_sustainable_traffic,
            broke_my_site_traffic
            ) VALUES ( 
             (SELECT config_id FROM mcaptcha_config where captcha_key= (?)
             AND user_id = (SELECT ID FROM mcaptcha_users WHERE name = ?)
            ), ?, ?, ?)",
            //payload.avg_traffic,
            captcha_key,
            username,
            pattern.avg_traffic as i32,
            pattern.peak_sustainable_traffic as i32,
            pattern.broke_my_site_traffic.as_ref().map(|v| *v as i32),
        )
        .execute(&self.pool)
        .await
        .map_err(|e| map_row_not_found_err(e, DBError::CaptchaNotFound))?;
        Ok(())
    }

    /// Get traffic configuration
    async fn get_traffic_pattern(
        &self,
        username: &str,
        captcha_key: &str,
    ) -> DBResult<TrafficPattern> {
        struct Traffic {
            peak_sustainable_traffic: i32,
            avg_traffic: i32,
            broke_my_site_traffic: Option<i32>,
        }
        let res = sqlx::query_as!(
            Traffic,
            "SELECT 
          avg_traffic, 
          peak_sustainable_traffic, 
          broke_my_site_traffic 
        FROM 
          mcaptcha_sitekey_user_provided_avg_traffic 
        WHERE 
          config_id = (
            SELECT 
              config_id 
            FROM 
              mcaptcha_config 
            WHERE 
              captcha_key = ? 
              AND user_id = (
                SELECT 
                  id 
                FROM 
                  mcaptcha_users 
                WHERE 
                  NAME = ?
              )
          )
        ",
            captcha_key,
            username
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| map_row_not_found_err(e, DBError::TrafficPatternNotFound))?;
        Ok(TrafficPattern {
            broke_my_site_traffic: res.broke_my_site_traffic.as_ref().map(|v| *v as u32),
            avg_traffic: res.avg_traffic as u32,
            peak_sustainable_traffic: res.peak_sustainable_traffic as u32,
        })
    }

    /// Delete traffic configuration
    async fn delete_traffic_pattern(
        &self,
        username: &str,
        captcha_key: &str,
    ) -> DBResult<()> {
        sqlx::query!(
            "DELETE FROM mcaptcha_sitekey_user_provided_avg_traffic
        WHERE config_id = (
            SELECT config_id 
            FROM 
                mcaptcha_config 
            WHERE
                captcha_key = ?
            AND 
                user_id = (SELECT ID FROM mcaptcha_users WHERE name = ?)
            );",
            captcha_key,
            username,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| map_row_not_found_err(e, DBError::TrafficPatternNotFound))?;
        Ok(())
    }

    /// create new notification
    async fn create_notification(&self, p: &AddNotification) -> DBResult<()> {
        let now = now_unix_time_stamp();
        sqlx::query!(
            "INSERT INTO mcaptcha_notifications (
              heading, message, tx, rx, received)
              VALUES  (
              ?, ?,
                  (SELECT ID FROM mcaptcha_users WHERE name = ?),
                  (SELECT ID FROM mcaptcha_users WHERE name = ?),
                  ?
                      );",
            p.heading,
            p.message,
            p.from,
            p.to,
            now
        )
        .execute(&self.pool)
        .await
        .map_err(map_register_err)?;

        Ok(())
    }

    /// get all unread notifications
    async fn get_all_unread_notifications(
        &self,
        username: &str,
    ) -> DBResult<Vec<Notification>> {
        let mut inner_notifications = sqlx::query_file_as!(
            InnerNotification,
            "./src/get_all_unread_notifications.sql",
            &username
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| map_row_not_found_err(e, DBError::AccountNotFound))?;

        let mut notifications = Vec::with_capacity(inner_notifications.len());

        inner_notifications
            .drain(0..)
            .for_each(|n| notifications.push(n.into()));

        Ok(notifications)
    }

    /// mark a notification read
    async fn mark_notification_read(&self, username: &str, id: i32) -> DBResult<()> {
        sqlx::query_file_as!(
            Notification,
            "./src/mark_notification_read.sql",
            id,
            &username
        )
        .execute(&self.pool)
        .await
        .map_err(|e| map_row_not_found_err(e, DBError::NotificationNotFound))?;

        Ok(())
    }

    /// record PoWConfig fetches
    async fn record_fetch(&self, key: &str) -> DBResult<()> {
        let now = now_unix_time_stamp();
        let _ = sqlx::query!(
        "INSERT INTO mcaptcha_pow_fetched_stats 
        (config_id, time) VALUES ((SELECT config_id FROM mcaptcha_config where captcha_key= ?), ?)",
        key,
        &now,
    )
    .execute(&self.pool)
    .await
        .map_err(|e| map_row_not_found_err(e, DBError::CaptchaNotFound))?;
        Ok(())
    }

    /// record PoWConfig solves
    async fn record_solve(&self, key: &str) -> DBResult<()> {
        let now = OffsetDateTime::now_utc();
        let _ = sqlx::query!(
        "INSERT INTO mcaptcha_pow_solved_stats 
        (config_id, time) VALUES ((SELECT config_id FROM mcaptcha_config where captcha_key= ?), ?)",
        key,
        &now,
    )
    .execute(&self.pool)
    .await
    .map_err(|e| map_row_not_found_err(e, DBError::CaptchaNotFound))?;
        Ok(())
    }

    /// record PoWConfig confirms
    async fn record_confirm(&self, key: &str) -> DBResult<()> {
        let now = now_unix_time_stamp();
        let _ = sqlx::query!(
        "INSERT INTO mcaptcha_pow_confirmed_stats 
        (config_id, time) VALUES ((SELECT config_id FROM mcaptcha_config where captcha_key= ?), ?)",
        key,
        &now
    )
    .execute(&self.pool)
    .await
        .map_err(|e| map_row_not_found_err(e, DBError::CaptchaNotFound))?;
        Ok(())
    }

    /// fetch PoWConfig fetches
    async fn fetch_config_fetched(&self, user: &str, key: &str) -> DBResult<Vec<i64>> {
        let records = sqlx::query_as!(
            Date,
            "SELECT time FROM mcaptcha_pow_fetched_stats
            WHERE 
                config_id = (
                    SELECT 
                        config_id FROM mcaptcha_config 
                    WHERE 
                        captcha_key = ?
                    AND
                        user_id = (
                        SELECT 
                            ID FROM mcaptcha_users WHERE name = ?))
                ORDER BY time DESC",
            &key,
            &user,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| map_row_not_found_err(e, DBError::CaptchaNotFound))?;

        Ok(Date::dates_to_unix(records))
    }

    /// fetch PoWConfig solves
    async fn fetch_solve(&self, user: &str, key: &str) -> DBResult<Vec<i64>> {
        let records = sqlx::query_as!(
            Date,
            "SELECT time FROM mcaptcha_pow_solved_stats 
            WHERE config_id = (
                SELECT config_id FROM mcaptcha_config 
                WHERE 
                    captcha_key = ?
                AND
                     user_id = (
                        SELECT 
                            ID FROM mcaptcha_users WHERE name = ?)) 
                ORDER BY time DESC",
            &key,
            &user
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| map_row_not_found_err(e, DBError::CaptchaNotFound))?;

        Ok(Date::dates_to_unix(records))
    }

    /// fetch PoWConfig confirms
    async fn fetch_confirm(&self, user: &str, key: &str) -> DBResult<Vec<i64>> {
        let records = sqlx::query_as!(
            Date,
            "SELECT time FROM mcaptcha_pow_confirmed_stats 
            WHERE 
                config_id = (
                    SELECT config_id FROM mcaptcha_config 
                WHERE 
                    captcha_key = ?
                AND
                     user_id = (
                        SELECT 
                            ID FROM mcaptcha_users WHERE name = ?))
                ORDER BY time DESC",
            &key,
            &user
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| map_row_not_found_err(e, DBError::CaptchaNotFound))?;

        Ok(Date::dates_to_unix(records))
    }

    /// record PoW timing
    async fn analysis_save(
        &self,
        captcha_id: &str,
        d: &CreatePerformanceAnalytics,
    ) -> DBResult<()> {
        let _ = sqlx::query!(
            "INSERT INTO mcaptcha_pow_analytics 
            (config_id, time, difficulty_factor, worker_type)
        VALUES ((SELECT config_id FROM mcaptcha_config where captcha_key= ?), ?, ?, ?)",
            captcha_id,
            d.time as i32,
            d.difficulty_factor as i32,
            &d.worker_type,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| map_row_not_found_err(e, DBError::CaptchaNotFound))?;
        Ok(())
    }

    /// fetch PoW analytics
    async fn analytics_fetch(
        &self,
        captcha_id: &str,
        limit: usize,
        offset: usize,
    ) -> DBResult<Vec<PerformanceAnalytics>> {
        struct P {
            id: i32,
            time: i32,
            difficulty_factor: i32,
            worker_type: String,
        }

        impl From<P> for PerformanceAnalytics {
            fn from(v: P) -> Self {
                Self {
                    id: v.id as usize,
                    time: v.time as u32,
                    difficulty_factor: v.difficulty_factor as u32,
                    worker_type: v.worker_type,
                }
            }
        }

        let mut c = sqlx::query_as!(
            P,
            "SELECT
                id, time, difficulty_factor, worker_type
            FROM
                mcaptcha_pow_analytics
            WHERE
                config_id = (
                    SELECT config_id FROM mcaptcha_config WHERE captcha_key = ?
                ) 
            ORDER BY ID
            LIMIT ? OFFSET ?",
            &captcha_id,
            limit as i64,
            offset as i64,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| map_row_not_found_err(e, DBError::CaptchaNotFound))?;
        let mut res = Vec::with_capacity(c.len());
        for i in c.drain(0..) {
            res.push(i.into())
        }

        Ok(res)
    }

    /// Create psuedo ID against campaign ID to publish analytics
    async fn analytics_create_psuedo_id_if_not_exists(
        &self,
        captcha_id: &str,
    ) -> DBResult<()> {
        let id = Uuid::new_v4();
        sqlx::query!(
            "
            INSERT INTO
                mcaptcha_psuedo_campaign_id (config_id, psuedo_id)
            VALUES (
                (SELECT config_id FROM mcaptcha_config WHERE captcha_key = (?)),
                ?
            );",
            captcha_id,
            &id.to_string(),
        )
        .execute(&self.pool)
        .await
        .map_err(|e| map_row_not_found_err(e, DBError::CaptchaNotFound))?;

        Ok(())
    }

    /// Get psuedo ID from campaign ID
    async fn analytics_get_psuedo_id_from_capmaign_id(
        &self,
        captcha_id: &str,
    ) -> DBResult<String> {
        let res = sqlx::query_as!(
            PsuedoID,
            "SELECT psuedo_id FROM
                mcaptcha_psuedo_campaign_id
            WHERE
                 config_id = (SELECT config_id FROM mcaptcha_config WHERE captcha_key = (?));
            ",
            captcha_id
        ).fetch_one(&self.pool)
        .await
        .map_err(|e| map_row_not_found_err(e, DBError::CaptchaNotFound))?;

        Ok(res.psuedo_id)
    }

    /// Get campaign ID from psuedo ID
    async fn analytics_get_capmaign_id_from_psuedo_id(
        &self,
        psuedo_id: &str,
    ) -> DBResult<String> {
        struct ID {
            captcha_key: String,
        }

        let res = sqlx::query_as!(
            ID,
            "SELECT
                captcha_key
            FROM
                mcaptcha_config
            WHERE
                 config_id = (
                     SELECT
                         config_id
                     FROM
                         mcaptcha_psuedo_campaign_id
                     WHERE
                         psuedo_id = ?
                 );",
            psuedo_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| map_row_not_found_err(e, DBError::CaptchaNotFound))?;
        Ok(res.captcha_key)
    }

    async fn analytics_delete_all_records_for_campaign(
        &self,
        campaign_id: &str,
    ) -> DBResult<()> {
        let _ = sqlx::query!(
            "
        DELETE FROM
            mcaptcha_psuedo_campaign_id
        WHERE config_id = (
            SELECT config_id FROM mcaptcha_config WHERE captcha_key = ?
        );",
            campaign_id
        )
        .execute(&self.pool)
        .await;

        let _ = sqlx::query!(
            "
            DELETE FROM
                mcaptcha_pow_analytics
            WHERE
                config_id = (
                    SELECT config_id FROM mcaptcha_config WHERE captcha_key = ?
            ) ",
            campaign_id
        )
        .execute(&self.pool)
        .await;

        Ok(())
    }
    /// Get all psuedo IDs
    async fn analytics_get_all_psuedo_ids(&self, page: usize) -> DBResult<Vec<String>> {
        const LIMIT: usize = 50;
        let offset = LIMIT * page;

        let mut res = sqlx::query_as!(
            PsuedoID,
            "
                SELECT
                    psuedo_id
                FROM
                    mcaptcha_psuedo_campaign_id
                    ORDER BY ID ASC LIMIT ? OFFSET ?;",
            LIMIT as i64,
            offset as i64
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| map_row_not_found_err(e, DBError::CaptchaNotFound))?;

        Ok(res.drain(0..).map(|r| r.psuedo_id).collect())
    }

    /// Track maximum nonce received against captcha levels
    async fn update_max_nonce_for_level(
        &self,
        captcha_key: &str,
        difficulty_factor: u32,
        latest_nonce: u32,
    ) -> DBResult<()> {
        let latest_nonce = latest_nonce as i64;
        sqlx::query!(
                "UPDATE mcaptcha_track_nonce SET nonce = ?
                WHERE level_id =  (
                    SELECT
                        level_id
                    FROM
                        mcaptcha_levels
                    WHERE
                        config_id = (SELECT config_id FROM mcaptcha_config WHERE captcha_key = ?)
                    AND
                        difficulty_factor = ?
                    )
                AND nonce <= ?;",
                latest_nonce,
                &captcha_key,
                difficulty_factor as i64,
                latest_nonce
            )
            .execute(&self.pool).await
        .map_err(|e| map_row_not_found_err(e, DBError::CaptchaNotFound))?;

        Ok(())
    }

    /// Get maximum nonce tracked so far for captcha levels
    async fn get_max_nonce_for_level(
        &self,
        captcha_key: &str,
        difficulty_factor: u32,
    ) -> DBResult<u32> {
        struct X {
            nonce: i32,
        }

        async fn inner_get_max_nonce(
            pool: &MySqlPool,
            captcha_key: &str,
            difficulty_factor: u32,
        ) -> DBResult<X> {
            sqlx::query_as!(
                X,
                "SELECT nonce FROM mcaptcha_track_nonce
                WHERE level_id =  (
                    SELECT
                        level_id
                    FROM
                        mcaptcha_levels
                    WHERE
                        config_id = (SELECT config_id FROM mcaptcha_config WHERE captcha_key = ?)
                    AND
                        difficulty_factor = ?
                    );",
                &captcha_key,
                difficulty_factor as i32,
            )
                .fetch_one(pool).await
                .map_err(|e| map_row_not_found_err(e, DBError::CaptchaNotFound))
        }

        let res = inner_get_max_nonce(&self.pool, captcha_key, difficulty_factor).await;
        if let Err(DBError::CaptchaNotFound) = res {
            sqlx::query!(
                "INSERT INTO
                    mcaptcha_track_nonce (level_id, nonce)
                VALUES  ((
                    SELECT
                        level_id
                    FROM
                        mcaptcha_levels
                    WHERE
                        config_id = (SELECT config_id FROM mcaptcha_config WHERE captcha_key =?)
                    AND
                        difficulty_factor = ?
                    ), ?);",
                &captcha_key,
                difficulty_factor as i32,
                0,
            )
            .execute(&self.pool)
            .await
                .map_err(|e| map_row_not_found_err(e, DBError::CaptchaNotFound))?;

            let res =
                inner_get_max_nonce(&self.pool, captcha_key, difficulty_factor).await?;
            Ok(res.nonce as u32)
        } else {
            let res = res?;
            Ok(res.nonce as u32)
        }
    }

    /// Get number of analytics entries that are under a certain duration
    async fn stats_get_num_logs_under_time(&self, duration: u32) -> DBResult<usize> {
        struct Count {
            count: Option<i64>,
        }

        //"SELECT COUNT(*) FROM (SELECT difficulty_factor FROM mcaptcha_pow_analytics WHERE time <= ?) as count",
        let count = sqlx::query_as!(
            Count,
            "SELECT
                COUNT(difficulty_factor) AS count
            FROM
                mcaptcha_pow_analytics
            WHERE time <= ?;",
            duration as i32,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| map_row_not_found_err(e, DBError::CaptchaNotFound))?;

        Ok(count.count.unwrap_or_else(|| 0) as usize)
    }

    /// Get the entry at a location in the list of analytics entires under a certain time limited
    /// and sorted in ascending order
    async fn stats_get_entry_at_location_for_time_limit_asc(
        &self,
        duration: u32,
        location: u32,
    ) -> DBResult<Option<usize>> {
        struct Difficulty {
            difficulty_factor: Option<i32>,
        }

        match sqlx::query_as!(
            Difficulty,
            "SELECT
            difficulty_factor
        FROM
            mcaptcha_pow_analytics
        WHERE
            time <= ?
        ORDER BY difficulty_factor ASC LIMIT 1 OFFSET ?;",
            duration as i32,
            location as i64 - 1,
        )
        .fetch_one(&self.pool)
        .await
        {
            Ok(res) => Ok(Some(res.difficulty_factor.unwrap() as usize)),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => Err(map_row_not_found_err(e, DBError::CaptchaNotFound)),
        }
    }

    /// Get all easy captcha configurations on instance
    async fn get_all_easy_captchas(
        &self,
        limit: usize,
        offset: usize,
    ) -> DBResult<Vec<EasyCaptcha>> {
        struct InnerEasyCaptcha {
            captcha_key: String,
            name: String,
            username: String,
            peak_sustainable_traffic: i32,
            avg_traffic: i32,
            broke_my_site_traffic: Option<i32>,
        }
        let mut inner_res = sqlx::query_as!(
            InnerEasyCaptcha,
                "SELECT 
              mcaptcha_sitekey_user_provided_avg_traffic.avg_traffic, 
              mcaptcha_sitekey_user_provided_avg_traffic.peak_sustainable_traffic, 
              mcaptcha_sitekey_user_provided_avg_traffic.broke_my_site_traffic,
              mcaptcha_config.name,
              mcaptcha_users.name as username,
              mcaptcha_config.captcha_key
            FROM 
              mcaptcha_sitekey_user_provided_avg_traffic 
            INNER JOIN
                mcaptcha_config
            ON
                mcaptcha_config.config_id = mcaptcha_sitekey_user_provided_avg_traffic.config_id
            INNER JOIN
                mcaptcha_users
            ON
                mcaptcha_config.user_id = mcaptcha_users.ID
            ORDER BY mcaptcha_config.config_id
            LIMIT ? OFFSET ?",
            limit as i64,
            offset as i64
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| map_row_not_found_err(e, DBError::TrafficPatternNotFound))?;
        let mut res = Vec::with_capacity(inner_res.len());
        inner_res.drain(0..).for_each(|v| {
            res.push(EasyCaptcha {
                key: v.captcha_key,
                description: v.name,
                username: v.username,
                traffic_pattern: TrafficPattern {
                    broke_my_site_traffic: v
                        .broke_my_site_traffic
                        .as_ref()
                        .map(|v| *v as u32),
                    avg_traffic: v.avg_traffic as u32,
                    peak_sustainable_traffic: v.peak_sustainable_traffic as u32,
                },
            })
        });
        Ok(res)
    }
}

#[derive(Clone)]
struct Date {
    time: OffsetDateTime,
}

impl Date {
    fn dates_to_unix(mut d: Vec<Self>) -> Vec<i64> {
        let mut dates = Vec::with_capacity(d.len());
        d.drain(0..)
            .for_each(|x| dates.push(x.time.unix_timestamp()));
        dates
    }
}

fn now_unix_time_stamp() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

#[derive(Debug, Clone, PartialEq)]
/// Represents notification
pub struct InnerNotification {
    /// receiver name  of the notification
    pub name: String,
    /// heading of the notification
    pub heading: String,
    /// message of the notification
    pub message: String,
    /// when notification was received
    pub received: OffsetDateTime,
    /// db assigned ID of the notification
    pub id: i32,
}

impl From<InnerNotification> for Notification {
    fn from(n: InnerNotification) -> Self {
        Notification {
            name: Some(n.name),
            heading: Some(n.heading),
            message: Some(n.message),
            received: Some(n.received.unix_timestamp()),
            id: Some(n.id),
        }
    }
}

#[derive(Clone)]
struct InternaleCaptchaConfig {
    config_id: i32,
    duration: i32,
    name: String,
    captcha_key: String,
}

impl From<InternaleCaptchaConfig> for Captcha {
    fn from(i: InternaleCaptchaConfig) -> Self {
        Self {
            config_id: i.config_id,
            duration: i.duration,
            description: i.name,
            key: i.captcha_key,
        }
    }
}

struct PsuedoID {
    psuedo_id: String,
}
