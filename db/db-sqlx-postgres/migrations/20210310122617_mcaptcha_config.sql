-- SPDX-FileCopyrightText: 2023 Aravinth Manivannan <realaravinth@batsense.net>
--
-- SPDX-License-Identifier: AGPL-3.0-or-later

CREATE TABLE IF NOT EXISTS mcaptcha_config (
	config_id SERIAL PRIMARY KEY NOT NULL,
	user_id INTEGER NOT NULL references mcaptcha_users(ID) ON DELETE CASCADE,
	key varchar(100) NOT NULL UNIQUE,
	name varchar(100) NOT NULL,
	duration integer NOT NULL DEFAULT 30
);
