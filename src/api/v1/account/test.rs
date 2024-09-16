// Copyright (C) 2022  Aravinth Manivannan <realaravinth@batsense.net>
// SPDX-FileCopyrightText: 2023 Aravinth Manivannan <realaravinth@batsense.net>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use actix_web::http::StatusCode;
use actix_web::test;

use super::email::*;
use super::username::Username;
use super::*;
use crate::api::v1::auth::runners::Password;
use crate::api::v1::ROUTES;
use crate::*;

use crate::errors::*;
use crate::tests::*;

#[actix_rt::test]
async fn uname_email_exists_works_pg() {
    let data = crate::tests::pg::get_data().await;
    uname_email_exists_works(data).await;
}

#[actix_rt::test]
async fn uname_email_exists_works_maria() {
    let data = crate::tests::maria::get_data().await;
    uname_email_exists_works(data).await;
}

pub async fn uname_email_exists_works(data: ArcData) {
    const NAME: &str = "testuserexists";
    const PASSWORD: &str = "longpassword2";
    const EMAIL: &str = "testuserexists@a.com2";
    let data = &data;
    delete_user(data, NAME).await;

    let (_, signin_resp) = register_and_signin(data, NAME, EMAIL, PASSWORD).await;
    let cookies = get_cookie!(signin_resp);
    let app = get_app!(data).await;

    // check if get user secret works
    let resp = test::call_service(
        &app,
        test::TestRequest::get()
            .cookie(cookies.clone())
            .uri(ROUTES.account.get_secret)
            .to_request(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);

    // check if get user secret works
    let resp = test::call_service(
        &app,
        test::TestRequest::post()
            .cookie(cookies.clone())
            .uri(ROUTES.account.update_secret)
            .to_request(),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let mut payload = AccountCheckPayload { val: NAME.into() };

    let user_exists_resp = test::call_service(
        &app,
        post_request!(&payload, ROUTES.account.username_exists)
            .cookie(cookies.clone())
            .to_request(),
    )
    .await;
    assert_eq!(user_exists_resp.status(), StatusCode::OK);
    let mut resp: AccountCheckResp = test::read_body_json(user_exists_resp).await;
    assert!(resp.exists);

    payload.val = PASSWORD.into();

    let user_doesnt_exist = test::call_service(
        &app,
        post_request!(&payload, ROUTES.account.username_exists)
            .cookie(cookies.clone())
            .to_request(),
    )
    .await;
    assert_eq!(user_doesnt_exist.status(), StatusCode::OK);
    resp = test::read_body_json(user_doesnt_exist).await;
    assert!(!resp.exists);

    let email_doesnt_exist = test::call_service(
        &app,
        post_request!(&payload, ROUTES.account.email_exists)
            .cookie(cookies.clone())
            .to_request(),
    )
    .await;
    assert_eq!(email_doesnt_exist.status(), StatusCode::OK);
    resp = test::read_body_json(email_doesnt_exist).await;
    assert!(!resp.exists);

    payload.val = EMAIL.into();

    let email_exist = test::call_service(
        &app,
        post_request!(&payload, ROUTES.account.email_exists)
            .cookie(cookies.clone())
            .to_request(),
    )
    .await;
    assert_eq!(email_exist.status(), StatusCode::OK);
    resp = test::read_body_json(email_exist).await;
    assert!(resp.exists);
}

#[actix_rt::test]
async fn email_udpate_password_validation_del_userworks_pg() {
    let data = crate::tests::pg::get_data().await;
    email_udpate_password_validation_del_userworks(data).await;
}

#[actix_rt::test]
async fn email_udpate_password_validation_del_userworks_maria() {
    let data = crate::tests::maria::get_data().await;
    email_udpate_password_validation_del_userworks(data).await;
}

pub async fn email_udpate_password_validation_del_userworks(data: ArcData) {
    const NAME: &str = "testuser2";
    const PASSWORD: &str = "longpassword2";
    const EMAIL: &str = "testuser1@a.com2";
    const NAME2: &str = "eupdauser";
    const EMAIL2: &str = "eupdauser@a.com";

    let data = &data;
    delete_user(data, NAME).await;
    delete_user(data, NAME2).await;

    let _ = register_and_signin(data, NAME2, EMAIL2, PASSWORD).await;
    let (_creds, signin_resp) = register_and_signin(data, NAME, EMAIL, PASSWORD).await;
    let cookies = get_cookie!(signin_resp);
    let app = get_app!(data).await;

    // update email
    let mut email_payload = Email {
        email: EMAIL.into(),
    };
    let email_update_resp = test::call_service(
        &app,
        post_request!(&email_payload, ROUTES.account.update_email)
            //post_request!(&email_payload, EMAIL_UPDATE)
            .cookie(cookies.clone())
            .to_request(),
    )
    .await;
    assert_eq!(email_update_resp.status(), StatusCode::OK);

    // check duplicate email while duplicate email
    email_payload.email = EMAIL2.into();
    bad_post_req_test(
        data,
        NAME,
        PASSWORD,
        ROUTES.account.update_email,
        &email_payload,
        ServiceError::EmailTaken,
    )
    .await;

    // wrong password while deleting account
    let mut payload = Password {
        password: NAME.into(),
    };
    bad_post_req_test(
        data,
        NAME,
        PASSWORD,
        ROUTES.account.delete,
        &payload,
        ServiceError::WrongPassword,
    )
    .await;

    // delete account
    payload.password = PASSWORD.into();
    let delete_user_resp = test::call_service(
        &app,
        post_request!(&payload, ROUTES.account.delete)
            .cookie(cookies.clone())
            .to_request(),
    )
    .await;

    assert_eq!(delete_user_resp.status(), StatusCode::OK);

    // try to delete an account that doesn't exist
    let account_not_found_resp = test::call_service(
        &app,
        post_request!(&payload, ROUTES.account.delete)
            .cookie(cookies)
            .to_request(),
    )
    .await;
    assert_eq!(account_not_found_resp.status(), StatusCode::NOT_FOUND);
    let txt: ErrorToResponse = test::read_body_json(account_not_found_resp).await;
    assert_eq!(txt.error, format!("{}", ServiceError::AccountNotFound));
}

#[actix_rt::test]
async fn username_update_works_pg() {
    let data = crate::tests::pg::get_data().await;
    username_update_works(data).await;
}

#[actix_rt::test]
async fn username_update_works_maria() {
    let data = crate::tests::maria::get_data().await;
    username_update_works(data).await;
}

pub async fn username_update_works(data: ArcData) {
    const NAME: &str = "testuserupda";
    const EMAIL: &str = "testuserupda@sss.com";
    const EMAIL2: &str = "testuserupda2@sss.com";
    const PASSWORD: &str = "longpassword2";
    const NAME2: &str = "terstusrtds";
    const NAME_CHANGE: &str = "terstusrtdsxx";

    let data = &data;

    futures::join!(
        delete_user(data, NAME),
        delete_user(data, NAME2),
        delete_user(data, NAME_CHANGE),
    );

    let _ = register_and_signin(data, NAME2, EMAIL2, PASSWORD).await;
    let (_creds, signin_resp) = register_and_signin(data, NAME, EMAIL, PASSWORD).await;
    let cookies = get_cookie!(signin_resp);
    let app = get_app!(data).await;

    // update username
    let mut username_udpate = Username {
        username: NAME_CHANGE.into(),
    };
    let username_update_resp = test::call_service(
        &app,
        post_request!(&username_udpate, ROUTES.account.update_username)
            .cookie(cookies)
            .to_request(),
    )
    .await;
    assert_eq!(username_update_resp.status(), StatusCode::OK);

    // check duplicate username with duplicate username
    username_udpate.username = NAME2.into();
    bad_post_req_test(
        data,
        NAME_CHANGE,
        PASSWORD,
        ROUTES.account.update_username,
        &username_udpate,
        ServiceError::UsernameTaken,
    )
    .await;
}
