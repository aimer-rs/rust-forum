//! The forum's sign-in endpoints.
//!
//! Discourse ties a sign-in's bot check and its CSRF token to the session cookie
//! it hands out with them, so every request here goes through one client that
//! keeps its cookies. Once a member is signed in those cookies are what keep them
//! so, and they are kept in the platform's secret store (the vault) to survive it
//! being closed; see [`restore`].

use super::BASE_URL;
use aimer::vault::{PlatformVault, Vault};
use reqwest::Url;
use reqwest::cookie::{CookieStore, Jar};
use serde::Deserialize;
use serde_json::Value;
use std::sync::{Arc, OnceLock};

/// What the app's own vault is named.
const APP_ID: &str = "dev.aimers.rustforum";
const VAULT_NAMESPACE: &str = "session";
const VAULT_KEY: &str = "cookies";

const COULD_NOT_REACH: &str = "Couldn't reach the forum. Check your connection and try again.";
const COULD_NOT_SEND: &str = "The forum couldn't send a code. Try again in a moment.";
const COULD_NOT_SIGN_IN: &str = "Couldn't sign you in. Try again in a moment.";
const TOO_MANY: &str = "Too many attempts. Wait a little and try again.";

/// The cookies of the forum, kept apart from the client so they can be read back
/// to be saved and filled in again from what was saved.
pub(crate) fn jar() -> &'static Arc<Jar> {
    static JAR: OnceLock<Arc<Jar>> = OnceLock::new();
    JAR.get_or_init(|| Arc::new(Jar::default()))
}

/// The one client every request to the forum goes through, so that what signing
/// in left in [`jar`] goes with each of them. A request made with a client of its
/// own is a guest's, and the forum answers a guest differently: it refuses
/// `new.json` outright.
pub(crate) fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .cookie_provider(jar().clone())
            .build()
            .expect("a client with a cookie store builds")
    })
}

fn forum_url() -> Url {
    Url::parse(BASE_URL).expect("the forum's address is a URL")
}

/// The cookies to send the forum, as the `name=value; name=value` of a `Cookie`
/// header, or `None` when there are none.
fn saved_cookies() -> Option<String> {
    let header = jar().cookies(&forum_url())?;
    header.to_str().ok().map(str::to_owned).filter(|cookies| !cookies.is_empty())
}

/// The cookies of `header`, one `Set-Cookie` line each, as the jar is filled in
/// again with them. They come back as the forum sets its own: for every path,
/// over HTTPS only and out of reach of scripts.
fn cookie_lines(header: &str) -> Vec<String> {
    header
        .split(';')
        .map(str::trim)
        .filter(|pair| pair.split_once('=').is_some_and(|(name, _)| !name.is_empty()))
        .map(|pair| format!("{pair}; Path=/; Secure; HttpOnly"))
        .collect()
}

fn app_vault() -> Result<PlatformVault, String> {
    PlatformVault::for_app(APP_ID).map_err(|error| error.to_string())
}

/// Saves the forum's cookies to the vault, so that the member is still signed in
/// when the app is opened again. They are as good as a password, which is why
/// they are not left in a file.
///
/// The vault is awaited by a Venus task, as is the request that calls this.
async fn save_cookies() -> Result<(), String> {
    let Some(cookies) = saved_cookies() else {
        return Ok(());
    };
    app_vault()?
        .write(VAULT_NAMESPACE, VAULT_KEY, cookies.as_bytes())
        .await
        .map_err(|error| error.to_string())
}

/// Forgets the saved cookies.
async fn forget_cookies() -> Result<(), String> {
    app_vault()?
        .remove(VAULT_NAMESPACE, VAULT_KEY)
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
}

/// The member the app was left signed in as, if the forum still has them so.
///
/// The cookies saved when they signed in are put back and the forum is asked who
/// they belong to. Cookies it no longer honours (the member signed out
/// elsewhere, or they ran out) are forgotten. When the forum cannot be reached
/// nothing is decided: the cookies are kept for the next time, and the member
/// is a guest until then.
pub async fn restore() -> Result<Option<String>, String> {
    let Some(saved) = app_vault()?
        .read(VAULT_NAMESPACE, VAULT_KEY)
        .await
        .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    let saved = String::from_utf8_lossy(saved.expose());
    let url = forum_url();
    for line in cookie_lines(&saved) {
        jar().add_cookie_str(&line, &url);
    }
    let username = session_username().await?;
    if username.is_none() {
        let _ = forget_cookies().await;
    }
    Ok(username)
}

/// Signs out on this device: the cookies in the jar are expired and the saved
/// ones are deleted from the vault.
///
/// The forum is not told, so the sign-in it gave stays good for anyone who has a
/// copy of its cookies until the member ends it on the forum's own site.
pub async fn sign_out() -> Result<(), String> {
    expire_cookies(jar());
    forget_cookies().await
}

/// Expires every cookie the jar holds for the forum, as the forum itself would to
/// end a session, since a jar cannot be emptied any other way.
fn expire_cookies(jar: &Jar) {
    let url = forum_url();
    let held = jar.cookies(&url).and_then(|header| header.to_str().ok().map(str::to_owned));
    for name in cookie_names(held.as_deref().unwrap_or_default()) {
        jar.add_cookie_str(&format!("{name}=; Max-Age=0; Path=/; Secure; HttpOnly"), &url);
    }
}

/// The names of the cookies in the `name=value; name=value` of a `Cookie` header.
fn cookie_names(header: &str) -> Vec<&str> {
    header
        .split(';')
        .filter_map(|pair| pair.split_once('=').map(|(name, _)| name.trim()))
        .filter(|name| !name.is_empty())
        .collect()
}

#[derive(Deserialize)]
struct Csrf {
    csrf: String,
}

/// The pair the forum hands out to tell a person from a script: the value a
/// hidden `password_confirmation` field must come back holding, and the
/// `challenge` to answer.
#[derive(Deserialize)]
struct Honeypot {
    value: String,
    challenge: String,
}

/// What the forum expects back for `challenge`: the same text reversed.
pub fn challenge_answer(challenge: &str) -> String {
    challenge.chars().rev().collect()
}

/// `pairs` as the body of an `application/x-www-form-urlencoded` request.
fn form_body(pairs: &[(&str, &str)]) -> String {
    fn encode(text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        for byte in text.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    out.push(byte as char)
                }
                b' ' => out.push('+'),
                _ => out.push_str(&format!("%{byte:02X}")),
            }
        }
        out
    }
    pairs
        .iter()
        .map(|(key, value)| format!("{}={}", encode(key), encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}

/// The sentence to show for a failed reply: the forum's own when it gave one.
fn failure_message(status: u16, body: &Value) -> String {
    let said = body
        .get("error")
        .and_then(Value::as_str)
        .or_else(|| body.get("errors")?.as_array()?.first()?.as_str());
    match (said, status) {
        (Some(message), _) => message.to_owned(),
        (None, 429) => TOO_MANY.to_owned(),
        _ => COULD_NOT_SEND.to_owned(),
    }
}

/// Whether the forum's reply to a request for a code says it was taken.
fn code_requested(status: u16, body: &Value) -> Result<(), String> {
    if (200..300).contains(&status) && body.get("success").and_then(Value::as_str) == Some("OK") {
        Ok(())
    } else {
        Err(failure_message(status, body))
    }
}

async fn get_json<T: for<'de> Deserialize<'de>>(path: &str) -> Result<T, String> {
    let body = client()
        .get(format!("{BASE_URL}/{path}"))
        .header("Accept", "application/json")
        .header("X-Requested-With", "XMLHttpRequest")
        .send()
        .await
        .map_err(|_| COULD_NOT_REACH.to_owned())?
        .text()
        .await
        .map_err(|_| COULD_NOT_REACH.to_owned())?;
    serde_json::from_str(&body).map_err(|_| COULD_NOT_SEND.to_owned())
}

async fn csrf_token() -> Result<String, String> {
    Ok(get_json::<Csrf>("session/csrf.json").await?.csrf)
}

/// Posts `pairs` as a form to `path`, with the CSRF token the session needs, and
/// returns the status and the JSON of the reply (`Null` when it was not JSON).
async fn post_form(path: &str, pairs: &[(&str, &str)]) -> Result<(u16, Value), String> {
    let response = client()
        .post(format!("{BASE_URL}/{path}"))
        .header("Accept", "application/json")
        .header("X-Requested-With", "XMLHttpRequest")
        .header("X-CSRF-Token", csrf_token().await?)
        .header("Content-Type", "application/x-www-form-urlencoded; charset=UTF-8")
        .body(form_body(pairs))
        .send()
        .await
        .map_err(|_| COULD_NOT_REACH.to_owned())?;
    let status = response.status().as_u16();
    let text = response.text().await.map_err(|_| COULD_NOT_REACH.to_owned())?;
    Ok((status, serde_json::from_str(&text).unwrap_or(Value::Null)))
}

/// Asks the forum to email a sign-in code to `email`.
///
/// The forum answers the same whether or not an account has that address, so a
/// success only says the request was taken. The cookie it sets stays in this
/// module's client for the request that checks the code.
pub async fn request_login_code(email: &str) -> Result<(), String> {
    let honeypot: Honeypot = get_json("session/hp.json").await?;
    let answer = challenge_answer(&honeypot.challenge);
    let (status, body) = post_form(
        "session/login-code",
        &[
            ("email", email),
            ("signup", "false"),
            ("password_confirmation", &honeypot.value),
            ("challenge", &answer),
        ],
    )
    .await?;
    code_requested(status, &body)
}

/// What checking a code came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verified {
    /// The session is now the member's, under this username.
    SignedIn(String),
    /// The code was right, but the account asks for a second factor: the
    /// passcode of an authenticator app, or a backup code.
    NeedsPasscode,
}

/// How a reply to checking a code reads.
#[derive(Debug, PartialEq, Eq)]
enum VerifyReply {
    Accepted,
    NeedsPasscode,
    Refused(String),
}

/// Whether the reply asks for a second factor: Discourse names the reason
/// `invalid_second_factor` and says which kinds the account has.
fn asks_for_passcode(body: &Value) -> bool {
    let flagged = |key: &str| body.get(key).and_then(Value::as_bool) == Some(true);
    body.get("reason")
        .and_then(Value::as_str)
        .is_some_and(|reason| reason.contains("second_factor"))
        || flagged("totp_enabled")
        || flagged("backup_enabled")
}

/// Which kind of second factor `passcode` is, in the numbers Discourse gives
/// them: 1 for the six digits of an authenticator app, 2 for a backup code.
fn second_factor_method(passcode: &str) -> &'static str {
    if passcode.len() == 6 && passcode.chars().all(|c| c.is_ascii_digit()) {
        "1"
    } else {
        "2"
    }
}

fn read_verify_reply(status: u16, body: &Value, sent_passcode: bool) -> VerifyReply {
    let refused = body.get("error").is_some() || body.get("errors").is_some() || !(200..300).contains(&status);
    if !refused {
        VerifyReply::Accepted
    } else if !sent_passcode && asks_for_passcode(body) {
        VerifyReply::NeedsPasscode
    } else {
        VerifyReply::Refused(failure_message(status, body))
    }
}

/// The username the session belongs to, `None` when it belongs to nobody, and an
/// error when the forum could not be asked.
async fn session_username() -> Result<Option<String>, String> {
    #[derive(Deserialize)]
    struct Current {
        current_user: CurrentUser,
    }
    #[derive(Deserialize)]
    struct CurrentUser {
        username: String,
    }
    let response = client()
        .get(format!("{BASE_URL}/session/current.json"))
        .header("Accept", "application/json")
        .header("X-Requested-With", "XMLHttpRequest")
        .send()
        .await
        .map_err(|_| COULD_NOT_REACH.to_owned())?;
    let status = response.status().as_u16();
    let text = response.text().await.map_err(|_| COULD_NOT_REACH.to_owned())?;
    match status {
        200..=299 => Ok(serde_json::from_str::<Current>(&text)
            .ok()
            .map(|current| current.current_user.username)),
        400..=499 => Ok(None),
        _ => Err(COULD_NOT_REACH.to_owned()),
    }
}

/// Checks the code the forum emailed to `email`, with the member's second-factor
/// `passcode` when the account asks for one.
///
/// What the reply says is only read for a refusal or a request for a passcode;
/// that the member is in is decided by asking the forum who the session now
/// belongs to.
pub async fn verify_login_code(
    email: &str,
    code: &str,
    passcode: Option<&str>,
) -> Result<Verified, String> {
    let timezone = iana_time_zone::get_timezone().ok();
    let mut pairs = vec![("email", email), ("code", code)];
    if let Some(timezone) = &timezone {
        pairs.push(("timezone", timezone));
    }
    if let Some(passcode) = passcode {
        pairs.push(("second_factor_token", passcode));
        pairs.push(("second_factor_method", second_factor_method(passcode)));
    }
    let (status, body) = post_form("session/login-code/verify", &pairs).await?;
    match read_verify_reply(status, &body, passcode.is_some()) {
        VerifyReply::Accepted => match session_username().await? {
            Some(username) => {
                // Saving is for the next launch: failing to must not undo this
                // one's sign-in.
                let _ = save_cookies().await;
                Ok(Verified::SignedIn(username))
            }
            None => Err(COULD_NOT_SIGN_IN.to_owned()),
        },
        VerifyReply::NeedsPasscode => Ok(Verified::NeedsPasscode),
        VerifyReply::Refused(note) => Err(note),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_challenge_is_answered_with_its_reverse() {
        assert_eq!(challenge_answer("0d01365403423bb795b38ff53a0212d1"), "1d2120a35ff83b597bb32430456310d0");
        assert_eq!(challenge_answer(""), "");
    }

    #[test]
    fn a_form_body_escapes_what_a_form_must() {
        assert_eq!(
            form_body(&[("email", "a+b@rust-lang.org"), ("signup", "false")]),
            "email=a%2Bb%40rust-lang.org&signup=false"
        );
        assert_eq!(form_body(&[("k", "two words")]), "k=two+words");
    }

    #[test]
    fn a_taken_request_is_a_success_reply_that_says_ok() {
        assert_eq!(code_requested(200, &json!({"success": "OK"})), Ok(()));
        assert!(code_requested(200, &json!({"success": "nope"})).is_err());
        assert!(code_requested(500, &json!({"success": "OK"})).is_err());
    }

    #[test]
    fn a_refusal_shows_the_forums_own_words_before_ours() {
        assert_eq!(failure_message(422, &json!({"error": "Bad."})), "Bad.");
        assert_eq!(failure_message(429, &json!({"errors": ["Slow down."]})), "Slow down.");
        assert_eq!(failure_message(429, &Value::Null), TOO_MANY);
        assert_eq!(failure_message(500, &Value::Null), COULD_NOT_SEND);
    }

    #[test]
    fn a_reply_that_says_nothing_wrong_is_accepted() {
        assert_eq!(read_verify_reply(200, &json!({"success": "OK"}), false), VerifyReply::Accepted);
        assert_eq!(read_verify_reply(200, &Value::Null, false), VerifyReply::Accepted);
    }

    #[test]
    fn a_refused_code_shows_the_forums_words() {
        assert_eq!(
            read_verify_reply(200, &json!({"error": "Wrong code."}), false),
            VerifyReply::Refused("Wrong code.".into())
        );
        assert!(matches!(read_verify_reply(500, &Value::Null, false), VerifyReply::Refused(_)));
    }

    #[test]
    fn a_reply_asking_for_a_second_factor_asks_for_the_passcode_once() {
        let body = json!({"error": "Enter it.", "reason": "invalid_second_factor", "totp_enabled": true});
        assert_eq!(read_verify_reply(200, &body, false), VerifyReply::NeedsPasscode);
        // With a passcode sent, the same reply is the passcode being wrong.
        assert_eq!(read_verify_reply(200, &body, true), VerifyReply::Refused("Enter it.".into()));
    }

    #[test]
    fn six_digits_are_an_authenticator_passcode_and_anything_else_a_backup_code() {
        assert_eq!(second_factor_method("123456"), "1");
        assert_eq!(second_factor_method("a1b2c3d4"), "2");
        assert_eq!(second_factor_method("1234567"), "2");
    }

    #[test]
    fn saved_cookies_come_back_as_the_forum_sets_them() {
        assert_eq!(
            cookie_lines("_t=abc123; _forum_session=xyz%2Fq"),
            vec![
                "_t=abc123; Path=/; Secure; HttpOnly".to_owned(),
                "_forum_session=xyz%2Fq; Path=/; Secure; HttpOnly".to_owned(),
            ]
        );
        assert!(cookie_lines("").is_empty());
        assert!(cookie_lines("; =x; justaword").is_empty());
    }

    #[test]
    fn cookies_put_in_the_jar_are_read_back_for_the_forum() {
        let jar = Jar::default();
        let url = forum_url();
        for line in cookie_lines("_t=abc123; _forum_session=xyz") {
            jar.add_cookie_str(&line, &url);
        }
        let header = jar.cookies(&url).expect("the forum has cookies");
        let header = header.to_str().unwrap();
        assert!(header.contains("_t=abc123") && header.contains("_forum_session=xyz"), "{header}");
    }

    #[test]
    fn expiring_the_cookies_empties_the_jar() {
        let jar = Jar::default();
        let url = forum_url();
        for line in cookie_lines("_t=abc123; _forum_session=xyz") {
            jar.add_cookie_str(&line, &url);
        }
        expire_cookies(&jar);
        assert!(jar.cookies(&url).is_none());
        // An empty jar is already as empty as it gets.
        expire_cookies(&jar);
    }
}
