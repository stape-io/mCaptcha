<div align="center">
  <h1>mCaptcha Guard</h1>
  <p>
    <strong>Back-end component of mCaptcha</strong>
  </p>

[![Documentation](https://img.shields.io/badge/docs-master-blue?style=flat-square)](https://mcaptcha.github.io/guard/guard/)
[![Build](https://github.com/mCaptcha/guard/actions/workflows/linux.yml/badge.svg)](https://github.com/mCaptcha/guard/actions/workflows/linux.yml)
[![dependency status](https://deps.rs/repo/github/mCaptcha/guard/status.svg?style=flat-square)](https://deps.rs/repo/github/mCaptcha/guard)
[![codecov](https://codecov.io/gh/mCaptcha/guard/branch/master/graph/badge.svg?style=flat-square)](https://codecov.io/gh/mCaptcha/guard)
<br />
[![AGPL License](https://img.shields.io/badge/license-AGPL-blue.svg?style=flat-square)](http://www.gnu.org/licenses/agpl-3.0)
[![Chat](https://img.shields.io/badge/matrix-+mcaptcha:matrix.batsense.net-purple?style=flat-square)](https://matrix.to/#/+mcaptcha:matrix.batsense.net)

**STATUS: ACTIVE DEVELOPMENT**

</div>

</div>

Guard is the back-end component of the [mCaptcha](https://mcaptcha.org)
system.

---

mCaptcha uses SHA256 based proof-of-work(PoW) to rate limit users.

When a user wants to do something on an mCaptcha-protected website,

1. they will have to generate proof-of-work(a bunch of math that will takes
   time to compute) and submit it to mCaptcha.

2. We'll validate the proof:

   - **if validation is unsuccessful**, they will be prevented from
     accessing their target website
   - **if validation is successful**, read on,

3. They will be issued a token that they should submit along
   with their request/form submission to the target website.

4. The target website should validate the user-submitted token with mCaptcha
   before processing the user's request.

The whole process is automated from the user's POV. All they have to do
is click on a button to initiate the process.

mCaptcha makes interacting with websites (computationally)expensive for
the user. A well-behaving user will experience a slight delay(no delay
when under moderate load to 2s when under attack; PoW difficulty is
variable) but if someone wants to hammer your site, they will have to do
more work to send requests than your server will have to do to respond
to their request.

## Why use mCaptcha?

- **Free software, privacy focused**
- **Seamless UX** - No more annoying CAPTCHAs!
- **IP address independent:** your users are behind a NAT? We got you covered!
- **Automatic bot throttling:**
- **Resistant to replay attacks:** proof-of-work configurations have
  short lifetimes(30s) and can be used only once. If a user submits a
  PoW to an already used configuration or an expired one, their proof
  will be rejected.

## Demo

### Demo servers are available at:

- https://demo.mcaptcha.org/
- https://demo2.mcaptcha.org/ (runs on a Raspberry Pi!)

> Core functionality is working but it's still very much
> work-in-progress. Since we don't have a stable release yet, hosted
> demo servers might be a few versions behind `master`. Please check footer for
> build commit.

Feel free to provide bogus information while signing up(project under
development, database frequently wiped).

### Self-hosted:

Clone the repo and run the following from the root of the repo:

```bash
$ docker-compose -d up
```

It takes a while to build the image so please be patient :)

## Development:

See [HACKING.md](./docs/HACKING.md)

## Deployment:

See [DEPLOYMENT.md](./docs/DEPLOYMET.md)

## Configuration:

See [CONFIGURATION.md](./docs/CONFIGURATION.md)
