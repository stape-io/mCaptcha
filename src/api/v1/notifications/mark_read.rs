// Copyright (C) 2022  Aravinth Manivannan <realaravinth@batsense.net>
// SPDX-FileCopyrightText: 2023 Aravinth Manivannan <realaravinth@batsense.net>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use actix_identity::Identity;
use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use crate::errors::*;
use crate::AppData;

#[derive(Deserialize, Serialize)]
pub struct MarkReadReq {
    pub id: i32,
}

/// route handler that marks a notification read
#[my_codegen::post(
    path = "crate::V1_API_ROUTES.notifications.mark_read",
    wrap = "crate::api::v1::get_middleware()"
)]
pub async fn mark_read(
    data: AppData,
    payload: web::Json<MarkReadReq>,
    id: Identity,
) -> ServiceResult<impl Responder> {
    let receiver = id.identity().unwrap();
    // TODO handle error where payload.to doesn't exist

    // TODO get payload from path /api/v1/notifications/{id}/read"
    data.db
        .mark_notification_read(&receiver, payload.id)
        .await?;

    Ok(HttpResponse::Ok())
}

#[cfg(test)]
pub mod tests {
    use actix_web::http::StatusCode;
    use actix_web::test;

    use super::*;
    use crate::api::v1::notifications::add::AddNotificationRequest;
    use crate::api::v1::notifications::get::NotificationResp;
    use crate::tests::*;
    use crate::*;

    #[actix_rt::test]
    async fn notification_mark_read_works_pg() {
        let data = pg::get_data().await;
        notification_mark_read_works(data).await;
    }

    #[actix_rt::test]
    async fn notification_mark_read_works_maria() {
        let data = maria::get_data().await;
        notification_mark_read_works(data).await;
    }

    pub async fn notification_mark_read_works(data: ArcData) {
        const NAME1: &str = "notifuser122";
        const NAME2: &str = "notiuser222";
        const PASSWORD: &str = "longpassworddomain";
        const EMAIL1: &str = "testnotification122@a.com";
        const EMAIL2: &str = "testnotification222@a.com";
        const HEADING: &str = "testing notifications get";
        const MESSAGE: &str = "testing notifications get message";
        let data = &data;

        delete_user(data, NAME1).await;
        delete_user(data, NAME2).await;

        register_and_signin(data, NAME1, EMAIL1, PASSWORD).await;
        register_and_signin(data, NAME2, EMAIL2, PASSWORD).await;
        let (_creds, signin_resp) = signin(data, NAME1, PASSWORD).await;
        let (_creds2, signin_resp2) = signin(data, NAME2, PASSWORD).await;
        let cookies = get_cookie!(signin_resp);
        let cookies2 = get_cookie!(signin_resp2);
        let app = get_app!(data).await;

        let msg = AddNotificationRequest {
            to: NAME2.into(),
            heading: HEADING.into(),
            message: MESSAGE.into(),
        };

        let send_notification_resp = test::call_service(
            &app,
            post_request!(&msg, V1_API_ROUTES.notifications.add)
                .cookie(cookies.clone())
                .to_request(),
        )
        .await;
        assert_eq!(send_notification_resp.status(), StatusCode::OK);

        let get_notifications_resp = test::call_service(
            &app,
            test::TestRequest::get()
                .uri(V1_API_ROUTES.notifications.get)
                .cookie(cookies2.clone())
                .to_request(),
        )
        .await;
        assert_eq!(get_notifications_resp.status(), StatusCode::OK);

        let mut notifications: Vec<NotificationResp> =
            test::read_body_json(get_notifications_resp).await;
        let notification = notifications.pop().unwrap();
        assert_eq!(notification.name, NAME1);
        assert_eq!(notification.message, MESSAGE);
        assert_eq!(notification.heading, HEADING);

        let mark_read_payload = MarkReadReq {
            id: notification.id,
        };
        let mark_read_resp = test::call_service(
            &app,
            post_request!(&mark_read_payload, V1_API_ROUTES.notifications.mark_read)
                .cookie(cookies2.clone())
                .to_request(),
        )
        .await;
        assert_eq!(mark_read_resp.status(), StatusCode::OK);

        let get_notifications_resp = test::call_service(
            &app,
            test::TestRequest::get()
                .uri(V1_API_ROUTES.notifications.get)
                .cookie(cookies2.clone())
                .to_request(),
        )
        .await;
        assert_eq!(get_notifications_resp.status(), StatusCode::OK);
        let mut notifications: Vec<NotificationResp> =
            test::read_body_json(get_notifications_resp).await;
        assert!(notifications.pop().is_none());
    }
}
