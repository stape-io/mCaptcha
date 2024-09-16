// Copyright (C) 2022  Aravinth Manivannan <realaravinth@batsense.net>
// SPDX-FileCopyrightText: 2023 Aravinth Manivannan <realaravinth@batsense.net>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! PoW Verification module

use actix_web::HttpRequest;
use actix_web::{web, HttpResponse, Responder};
use libmcaptcha::pow::Work;
use serde::{Deserialize, Serialize};

use crate::errors::*;
use crate::AppData;
use crate::V1_API_ROUTES;

#[derive(Clone, Debug, Deserialize, Serialize)]
/// validation token that clients receive as proof for submiting
/// valid PoW
pub struct ValidationToken {
    pub token: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ApiWork {
    pub string: String,
    pub result: String,
    pub nonce: u64,
    pub key: String,
    pub time: Option<u32>,
    pub worker_type: Option<String>,
}

impl From<ApiWork> for Work {
    fn from(value: ApiWork) -> Self {
        Self {
            string: value.string,
            nonce: value.nonce,
            result: value.result,
            key: value.key,
        }
    }
}

// API keys are mcaptcha actor names

/// route handler that verifies PoW and issues a solution token
/// if verification is successful
#[my_codegen::post(path = "V1_API_ROUTES.pow.verify_pow()")]
pub async fn verify_pow(
    req: HttpRequest,
    payload: web::Json<ApiWork>,
    data: AppData,
) -> ServiceResult<impl Responder> {
    #[cfg(not(test))]
    let ip = req.connection_info().peer_addr().unwrap().to_string();
    // From actix-web docs:
    //  Will only return None when called in unit tests unless TestRequest::peer_addr is used.
    //
    // ref: https://docs.rs/actix-web/latest/actix_web/struct.HttpRequest.html#method.peer_addr
    #[cfg(test)]
    let ip = "127.0.1.1".into();

    let key = payload.key.clone();
    let payload = payload.into_inner();
    let worker_type = payload.worker_type.clone();
    let time = payload.time;
    let nonce = payload.nonce;
    let (res, difficulty_factor) = data.captcha.verify_pow(payload.into(), ip).await?;
    data.stats.record_solve(&data, &key).await?;
    if let (Some(time), Some(worker_type)) = (time, worker_type) {
        let analytics = db_core::CreatePerformanceAnalytics {
            difficulty_factor,
            time,
            worker_type,
        };
        data.db.analysis_save(&key, &analytics).await?;
    }
    data.db
        .update_max_nonce_for_level(&key, difficulty_factor, nonce as u32)
        .await?;
    let payload = ValidationToken { token: res };
    Ok(HttpResponse::Ok().json(payload))
}

#[cfg(test)]
pub mod tests {
    use actix_web::http::StatusCode;
    use actix_web::test;
    use libmcaptcha::pow::PoWConfig;

    use super::*;
    use crate::api::v1::pow::get_config::GetConfigPayload;
    use crate::tests::*;
    use crate::*;

    #[actix_rt::test]
    async fn verify_pow_works_pg() {
        let data = crate::tests::pg::get_data().await;
        verify_pow_works(data).await;
    }

    #[actix_rt::test]
    async fn verify_pow_works_maria() {
        let data = crate::tests::maria::get_data().await;
        verify_pow_works(data).await;
    }

    #[actix_rt::test]
    async fn verify_analytics_pow_works_pg() {
        let data = crate::tests::pg::get_data().await;
        verify_analytics_pow_works(data).await;
    }

    #[actix_rt::test]
    async fn verify_analytics_pow_works_maria() {
        let data = crate::tests::maria::get_data().await;
        verify_analytics_pow_works(data).await;
    }

    pub async fn verify_analytics_pow_works(data: ArcData) {
        const NAME: &str = "powanalyticsuser";
        const PASSWORD: &str = "testingpas";
        const EMAIL: &str = "powanalyticsuser@a.com";
        let data = &data;

        delete_user(data, NAME).await;

        register_and_signin(data, NAME, EMAIL, PASSWORD).await;
        let (_, _signin_resp, token_key) = add_levels_util(data, NAME, PASSWORD).await;
        let app = get_app!(data).await;

        let get_config_payload = GetConfigPayload {
            key: token_key.key.clone(),
        };

        // update and check changes

        let get_config_resp = test::call_service(
            &app,
            post_request!(&get_config_payload, V1_API_ROUTES.pow.get_config)
                .to_request(),
        )
        .await;
        assert_eq!(get_config_resp.status(), StatusCode::OK);
        let config: PoWConfig = test::read_body_json(get_config_resp).await;

        let pow = mcaptcha_pow_sha256::ConfigBuilder::default()
            .salt(config.salt)
            .build()
            .unwrap();
        let work = pow
            .prove_work(&config.string.clone(), config.difficulty_factor)
            .unwrap();

        let work = ApiWork {
            string: config.string.clone(),
            result: work.result,
            nonce: work.nonce,
            key: token_key.key.clone(),
            time: Some(100),
            worker_type: Some("wasm".into()),
        };

        let pow_verify_resp = test::call_service(
            &app,
            post_request!(&work, V1_API_ROUTES.pow.verify_pow).to_request(),
        )
        .await;
        assert_eq!(pow_verify_resp.status(), StatusCode::OK);
        let limit = 50;
        let offset = 0;
        let mut analytics = data
            .db
            .analytics_fetch(&token_key.key, limit, offset)
            .await
            .unwrap();
        assert_eq!(analytics.len(), 1);
        let a = analytics.pop().unwrap();
        assert_eq!(a.time, work.time.unwrap());
        assert_eq!(a.worker_type, work.worker_type.unwrap());
    }

    pub async fn verify_pow_works(data: ArcData) {
        const NAME: &str = "powverifyusr";
        const PASSWORD: &str = "testingpas";
        const EMAIL: &str = "verifyuser@a.com";
        let data = &data;

        delete_user(data, NAME).await;

        register_and_signin(data, NAME, EMAIL, PASSWORD).await;
        let (_, _signin_resp, token_key) = add_levels_util(data, NAME, PASSWORD).await;
        let app = get_app!(data).await;

        let get_config_payload = GetConfigPayload {
            key: token_key.key.clone(),
        };

        // update and check changes

        let get_config_resp = test::call_service(
            &app,
            post_request!(&get_config_payload, V1_API_ROUTES.pow.get_config)
                .to_request(),
        )
        .await;
        assert_eq!(get_config_resp.status(), StatusCode::OK);
        let config: PoWConfig = test::read_body_json(get_config_resp).await;

        let pow = mcaptcha_pow_sha256::ConfigBuilder::default()
            .salt(config.salt)
            .build()
            .unwrap();
        let work = pow
            .prove_work(&config.string.clone(), config.difficulty_factor)
            .unwrap();

        let work = Work {
            string: config.string.clone(),
            result: work.result,
            nonce: work.nonce,
            key: token_key.key.clone(),
        };

        let pow_verify_resp = test::call_service(
            &app,
            post_request!(&work, V1_API_ROUTES.pow.verify_pow).to_request(),
        )
        .await;
        assert_eq!(pow_verify_resp.status(), StatusCode::OK);
        assert!(data
            .db
            .analytics_fetch(&token_key.key, 50, 0)
            .await
            .unwrap()
            .is_empty());

        let string_not_found = test::call_service(
            &app,
            post_request!(&work, V1_API_ROUTES.pow.verify_pow).to_request(),
        )
        .await;
        assert_eq!(string_not_found.status(), StatusCode::BAD_REQUEST);
        let err: ErrorToResponse = test::read_body_json(string_not_found).await;
        assert_eq!(err.error, "Challenge: not found");

        // let pow_config_resp = test::call_service(
        //     &app,
        //     post_request!(&get_config_payload, V1_API_ROUTES.pow.get_config).to_request(),
        // )
        // .await;
        // assert_eq!(pow_config_resp.status(), StatusCode::OK);
        // I'm not checking for errors because changing work.result triggered
        // InssuficientDifficulty, which is possible because libmcaptcha calculates
        // difficulty with the submitted result. Besides, this endpoint is merely
        // propagating errors from libmcaptcha and libmcaptcha has tests covering the
        // pow aspects ¯\_(ツ)_/¯
    }
}
