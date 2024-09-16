// Copyright (C) 2022  Aravinth Manivannan <realaravinth@batsense.net>
// SPDX-FileCopyrightText: 2023 Aravinth Manivannan <realaravinth@batsense.net>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use actix_identity::Identity;
use actix_web::{web, HttpResponse, Responder};
use sailfish::TemplateOnce;

use db_core::Captcha;
use libmcaptcha::defense::Level;

use crate::errors::*;
use crate::stats::CaptchaStats;
use crate::AppData;

const PAGE: &str = "SiteKeys";

#[derive(TemplateOnce, Clone)]
#[template(path = "panel/sitekey/view/index.html")]
struct IndexPage {
    duration: u32,
    name: String,
    key: String,
    levels: Vec<Level>,
    stats: CaptchaStats,
    publish_benchmarks: bool,
}

impl IndexPage {
    fn new(
        stats: CaptchaStats,
        config: Captcha,
        levels: Vec<Level>,
        key: String,
        publish_benchmarks: bool,
    ) -> Self {
        IndexPage {
            duration: config.duration as u32,
            name: config.description,
            levels,
            key,
            stats,
            publish_benchmarks,
        }
    }
}

/// route handler that renders individual views for sitekeys
#[my_codegen::get(
    path = "crate::PAGES.panel.sitekey.view",
    wrap = "crate::pages::get_middleware()"
)]
pub async fn view_sitekey(
    path: web::Path<String>,
    data: AppData,
    id: Identity,
) -> PageResult<impl Responder> {
    let username = id.identity().unwrap();
    let key = path.into_inner();
    let config = data.db.get_captcha_config(&username, &key).await?;
    let levels = data.db.get_captcha_levels(Some(&username), &key).await?;
    let stats = data.stats.fetch(&data, &username, &key).await?;
    let publish_benchmarks = data.db.analytics_captcha_is_published(&key).await?;

    let body = IndexPage::new(stats, config, levels, key, publish_benchmarks)
        .render_once()
        .unwrap();
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(body))
}

#[cfg(test)]
mod test {
    use actix_web::http::StatusCode;
    use actix_web::test;
    use actix_web::web::Bytes;

    use crate::tests::*;
    use crate::*;

    #[actix_rt::test]
    async fn view_sitekey_work_pg_test() {
        let data = pg::get_data().await;
        view_sitekey_work(data).await;
    }

    #[actix_rt::test]
    async fn view_sitekey_work_maria_test() {
        let data = maria::get_data().await;
        view_sitekey_work(data).await;
    }

    async fn view_sitekey_work(data: ArcData) {
        const NAME: &str = "viewsitekeyuser";
        const PASSWORD: &str = "longpassworddomain";
        const EMAIL: &str = "viewsitekeyuser@a.com";

        let data = &data;
        delete_user(data, NAME).await;

        register_and_signin(data, NAME, EMAIL, PASSWORD).await;
        let (_, signin_resp, key) = add_levels_util(data, NAME, PASSWORD).await;
        let cookies = get_cookie!(signin_resp);

        let app = get_app!(data).await;

        let url = format!("/sitekey/{}/", &key.key);

        let list_sitekey_resp = test::call_service(
            &app,
            test::TestRequest::get()
                .uri(&url)
                .cookie(cookies.clone())
                .to_request(),
        )
        .await;

        assert_eq!(list_sitekey_resp.status(), StatusCode::OK);

        let body: Bytes = test::read_body(list_sitekey_resp).await;
        let body = String::from_utf8(body.to_vec()).unwrap();

        assert!(body.contains(&key.name));

        assert!(body.contains(&L1.visitor_threshold.to_string()));
        assert!(body.contains(&L1.difficulty_factor.to_string()));
        assert!(body.contains(&L2.difficulty_factor.to_string()));
        assert!(body.contains(&L2.visitor_threshold.to_string()));
    }
}
