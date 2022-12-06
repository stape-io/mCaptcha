/*
 * mCaptcha is a PoW based DoS protection software.
 * This is the frontend web component of the mCaptcha system
 * Copyright © 2021 Aravinth Manivnanan <realaravinth@batsense.net>.
 *
 * Use of this source code is governed by Apache 2.0 or MIT license.
 * You shoud have received a copy of MIT and Apache 2.0 along with
 * this program. If not, see <https://spdx.org/licenses/MIT.html> for
 * MIT or <http://www.apache.org/licenses/LICENSE-2.0> for Apache.
 */

import {Work, PoWConfig, ServiceWorkerAction} from "./types";
import fetchPoWConfig from "./fetchPoWConfig";
import sendWork from "./sendWork";
import sendToParent from "./sendToParent";
import * as CONST from "./const";

import "./main.scss";

let LOCK = false;
const worker = new Worker("/bench.js");

/** add  mcaptcha widget element to DOM */
export const registerVerificationEventHandler = (): void => {
  const verificationContainer = <HTMLElement>(
    document.querySelector(".widget__verification-container")
  );
  verificationContainer.style.display = "flex";
};

let config: PoWConfig;

export const solveCaptchaRunner = async (): Promise<void> => {
  try {
    LOCK = true;
    // steps:
    // 1. show during
    CONST.messageText().during();
    // 1. get config
    config = await fetchPoWConfig();
    // 2. prove work
    worker.postMessage(config);
  } catch (e) {
    CONST.messageText().error();
    console.error(e);
    LOCK = false;
  }
};

registerVerificationEventHandler();

worker.onmessage = async (event: MessageEvent) => {
  const resp: ServiceWorkerAction = event.data;
  switch (resp.type) {
    case "init":
      return await solveCaptchaRunner();
    case "result":
      const proof: Work = {
        key: CONST.sitekey(),
        string: config.string,
        nonce: resp.payload.work.nonce,
        result: resp.payload.work.result,
      };

      // 3. submit work
      const token = await sendWork(proof);
      // 4. send token
      sendToParent(token);
      // 5. mark checkbox checked
      CONST.btn().checked = true;
      CONST.messageText().after();
      LOCK = false;
  }
};