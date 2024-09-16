// Copyright (C) 2022  Aravinth Manivannan <realaravinth@batsense.net>
// SPDX-FileCopyrightText: 2023 Aravinth Manivannan <realaravinth@batsense.net>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use actix_identity::Identity;
use actix_web::{HttpResponse, Responder};
use sailfish::TemplateOnce;

use crate::errors::PageResult;
use crate::pages::auth::sudo::SudoPage;
use crate::AppData;

pub mod routes {
    pub struct Settings {
        pub home: &'static str,
        pub delete_account: &'static str,
        pub update_secret: &'static str,
    }

    impl Settings {
        pub const fn new() -> Self {
            Settings {
                home: "/settings",
                delete_account: "/settings/account/delete",
                update_secret: "/settings/secret/update",
            }
        }

        pub const fn get_sitemap() -> [&'static str; 1] {
            const S: Settings = Settings::new();

            [S.home]
        }
    }
}

pub fn services(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(settings);
    cfg.service(update_secret);
    cfg.service(delete_account);
}

const PAGE: &str = "Settings";

#[derive(TemplateOnce, Clone)]
#[template(path = "panel/settings/index.html")]
pub struct IndexPage<'a> {
    email: Option<String>,
    secret: String,
    username: &'a str,
}

#[my_codegen::get(
    path = "crate::PAGES.panel.settings.home",
    wrap = "crate::pages::get_middleware()"
)]
async fn settings(data: AppData, id: Identity) -> PageResult<impl Responder> {
    let username = id.identity().unwrap();

    let secret = data.db.get_secret(&username).await?;
    let secret = secret.secret;
    let email = data.db.get_email(&username).await?;

    let data = IndexPage {
        email,
        secret,
        username: &username,
    };

    let body = data.render_once().unwrap();
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(body))
}

#[my_codegen::get(
    path = "crate::PAGES.panel.settings.delete_account",
    wrap = "crate::pages::get_middleware()"
)]
async fn delete_account() -> impl Responder {
    let page = SudoPage::<u8, u8>::new(crate::V1_API_ROUTES.account.delete, None)
        .render_once()
        .unwrap();
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(page)
}

#[my_codegen::get(
    path = "crate::PAGES.panel.settings.update_secret",
    wrap = "crate::pages::get_middleware()"
)]
async fn update_secret() -> impl Responder {
    let page = SudoPage::<u8, u8>::new(crate::V1_API_ROUTES.account.update_secret, None)
        .render_once()
        .unwrap();
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(page)
}
