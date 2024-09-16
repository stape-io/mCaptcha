// Copyright (C) 2022  Aravinth Manivannan <realaravinth@batsense.net>
// SPDX-FileCopyrightText: 2023 Aravinth Manivannan <realaravinth@batsense.net>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use actix_identity::Identity;
use actix_web::{HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use crate::errors::*;
use crate::AppData;

use db_core::Notification;

#[derive(Default, PartialEq, Clone, Deserialize, Serialize)]
pub struct NotificationResp {
    pub name: String,
    pub heading: String,
    pub message: String,
    pub received: i64,
    pub id: i32,
}

impl From<Notification> for NotificationResp {
    fn from(n: Notification) -> Self {
        NotificationResp {
            name: n.name.unwrap(),
            heading: n.heading.unwrap(),
            received: n.received.unwrap(),
            id: n.id.unwrap(),
            message: n.message.unwrap(),
        }
    }
}

impl NotificationResp {
    pub fn from_notifications(mut n: Vec<Notification>) -> Vec<Self> {
        let mut notifications = Vec::with_capacity(n.len());

        n.drain(0..).for_each(|x| {
            let y: NotificationResp = x.into();
            notifications.push(y)
        });

        notifications
    }
}

/// route handler that gets all unread notifications
#[my_codegen::get(
    path = "crate::V1_API_ROUTES.notifications.get",
    wrap = "crate::api::v1::get_middleware()"
)]
pub async fn get_notification(
    data: AppData,
    id: Identity,
) -> ServiceResult<impl Responder> {
    let receiver = id.identity().unwrap();
    // TODO handle error where payload.to doesn't exist

    let notifications = data.db.get_all_unread_notifications(&receiver).await?;
    let notifications = NotificationResp::from_notifications(notifications);
    Ok(HttpResponse::Ok().json(notifications))
}

#[cfg(test)]
pub mod tests {
    use actix_web::http::StatusCode;
    use actix_web::test;

    use super::*;
    use crate::api::v1::notifications::add::AddNotificationRequest;
    use crate::tests::*;
    use crate::*;

    #[actix_rt::test]
    async fn notification_get_works_pg() {
        let data = pg::get_data().await;
        notification_get_works(data).await;
    }

    #[actix_rt::test]
    async fn notification_get_works_maria() {
        let data = maria::get_data().await;
        notification_get_works(data).await;
    }

    pub async fn notification_get_works(data: ArcData) {
        const NAME1: &str = "notifuser12";
        const NAME2: &str = "notiuser22";
        const PASSWORD: &str = "longpassworddomain";
        const EMAIL1: &str = "testnotification12@a.com";
        const EMAIL2: &str = "testnotification22@a.com";
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
    }
}
